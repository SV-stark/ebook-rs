use crate::archive::EpubArchive;
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::layout::RenditionLayout;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::{extract_plain_text, Section};
use std::collections::HashMap;

/// PalmDOC LZ77 Decompressor.
pub fn decompress_palmdoc(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 2);
    let mut i = 0;

    while i < data.len() {
        let byte = data[i];
        i += 1;

        match byte {
            0x00 => {
                out.push(0x00);
            }
            0x01..=0x08 => {
                let count = byte as usize;
                let end = (i + count).min(data.len());
                out.extend_from_slice(&data[i..end]);
                i += count;
            }
            0x09..=0x7F => {
                out.push(byte);
            }
            0x80..=0xBF => {
                if i < data.len() {
                    let next = data[i];
                    i += 1;
                    let b1 = byte as usize;
                    let b2 = next as usize;
                    let distance = ((b1 & 0x3F) << 5) | (b2 >> 3);
                    let length = (b2 & 0x07) + 3;

                    if distance > 0 && distance <= out.len() {
                        let start = out.len() - distance;
                        for j in 0..length {
                            let val = out[start + (j % distance)];
                            out.push(val);
                        }
                    }
                }
            }
            0xC0..=0xFF => {
                out.push(b' ');
                out.push(byte ^ 0x80);
            }
        }
    }

    out
}

/// MOBI / AZW3 PDB Container Parser.
pub struct MobiBook;

impl MobiBook {
    /// Parse MOBI / AZW3 raw binary data into a `Book` struct.
    pub fn parse(bytes: &[u8]) -> Result<Book, String> {
        if bytes.len() < 78 {
            return Err("File too small for MOBI/AZW3 PDB header".to_string());
        }

        let name_bytes = &bytes[0..32];
        let name = String::from_utf8_lossy(name_bytes)
            .trim_matches('\0')
            .to_string();

        let num_records = u16::from_be_bytes([bytes[76], bytes[77]]) as usize;
        if bytes.len() < 78 + num_records * 8 {
            return Err("Truncated MOBI PDB record offset table".to_string());
        }

        let mut record_offsets = Vec::with_capacity(num_records);
        for i in 0..num_records {
            let start = 78 + i * 8;
            let offset = u32::from_be_bytes([
                bytes[start],
                bytes[start + 1],
                bytes[start + 2],
                bytes[start + 3],
            ]) as usize;
            record_offsets.push(offset);
        }

        if record_offsets.is_empty() {
            return Err("MOBI archive has 0 records".to_string());
        }

        let rec0_offset = record_offsets[0];
        let rec0_end = if record_offsets.len() > 1 {
            record_offsets[1]
        } else {
            bytes.len()
        };

        if rec0_offset >= bytes.len() || rec0_end > bytes.len() || rec0_offset + 16 > rec0_end {
            return Err("Invalid Record 0 bounds in MOBI header".to_string());
        }

        let rec0 = &bytes[rec0_offset..rec0_end];
        let compression = u16::from_be_bytes([rec0[0], rec0[1]]);
        let text_record_count = u16::from_be_bytes([rec0[8], rec0[9]]) as usize;

        let mut title = if name.is_empty() {
            "Untitled MOBI Book".to_string()
        } else {
            name
        };
        let mut author = "Unknown Author".to_string();
        let mut publisher = None;

        if rec0.len() >= 40 && &rec0[16..20] == b"MOBI" {
            let header_len = u32::from_be_bytes([rec0[20], rec0[21], rec0[22], rec0[23]]) as usize;
            let exth_flags = if rec0.len() >= 116 {
                u32::from_be_bytes([rec0[112], rec0[113], rec0[114], rec0[115]])
            } else {
                0
            };

            if (exth_flags & 0x40) != 0 && rec0.len() >= 16 + header_len + 12 {
                let exth_offset = 16 + header_len;
                if exth_offset + 12 <= rec0.len() && &rec0[exth_offset..exth_offset + 4] == b"EXTH"
                {
                    let count = u32::from_be_bytes([
                        rec0[exth_offset + 8],
                        rec0[exth_offset + 9],
                        rec0[exth_offset + 10],
                        rec0[exth_offset + 11],
                    ]) as usize;

                    let mut curr = exth_offset + 12;
                    for _ in 0..count {
                        if curr + 8 > rec0.len() {
                            break;
                        }
                        let tag = u32::from_be_bytes([
                            rec0[curr],
                            rec0[curr + 1],
                            rec0[curr + 2],
                            rec0[curr + 3],
                        ]);
                        let len = u32::from_be_bytes([
                            rec0[curr + 4],
                            rec0[curr + 5],
                            rec0[curr + 6],
                            rec0[curr + 7],
                        ]) as usize;

                        if len >= 8 && curr + len <= rec0.len() {
                            let val_bytes = &rec0[curr + 8..curr + len];
                            let val_str = String::from_utf8_lossy(val_bytes).trim().to_string();

                            match tag {
                                100 => author = val_str,
                                101 => publisher = Some(val_str),
                                503 => {
                                    if !val_str.is_empty() {
                                        title = val_str;
                                    }
                                }
                                _ => {}
                            }
                        }
                        curr += len.max(8);
                    }
                }
            }
        }

        let mut raw_html_bytes = Vec::new();
        let max_text_rec = (1 + text_record_count).min(record_offsets.len());

        for i in 1..max_text_rec {
            let start = record_offsets[i];
            let end = if i + 1 < record_offsets.len() {
                record_offsets[i + 1]
            } else {
                bytes.len()
            };

            if start >= bytes.len() || end > bytes.len() || start >= end {
                continue;
            }

            let chunk = &bytes[start..end];
            match compression {
                1 => raw_html_bytes.extend_from_slice(chunk),
                2 => raw_html_bytes.extend_from_slice(&decompress_palmdoc(chunk)),
                _ => raw_html_bytes.extend_from_slice(chunk),
            }
        }

        let full_html = String::from_utf8_lossy(&raw_html_bytes).to_string();

        let raw_sections = split_mobi_html(&full_html);
        let mut sections = Vec::with_capacity(raw_sections.len());
        let mut spine = Vec::with_capacity(raw_sections.len());
        let mut toc = Vec::new();

        for (idx, raw_sec_html) in raw_sections.into_iter().enumerate() {
            let idref = format!("section_{}", idx);
            let href = format!("section_{}.html", idx);
            let plain_text = extract_plain_text(&raw_sec_html);
            let plain_text_lower = plain_text.to_lowercase();
            let char_count = plain_text.chars().count();

            sections.push(Section {
                index: idx,
                idref: idref.clone(),
                href: href.clone(),
                full_path: href.clone(),
                raw_html: raw_sec_html.clone(),
                processed_html: raw_sec_html.clone(),
                plain_text,
                plain_text_lower,
                char_count,
            });

            spine.push(SpineItem {
                index: idx,
                idref,
                href: href.clone(),
                linear: true,
                media_type: "text/html".to_string(),
                properties: Vec::new(),
            });

            toc.push(NavPoint {
                id: format!("toc_{}", idx),
                label: format!("Section {}", idx + 1),
                href: href.clone(),
                full_path: href,
                subitems: Vec::new(),
            });
        }

        let metadata = Metadata {
            title,
            creators: vec![author],
            publishers: publisher.map(|p| vec![p]).unwrap_or_default(),
            languages: vec!["en".to_string()],
            rights: None,
            description: None,
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects: Vec::new(),
            cover_id: None,
            cover_href: None,
            direction: PageProgressionDirection::Ltr,
            meta_properties: HashMap::new(),
        };

        let opf = OpfPackage {
            version: "2.0".to_string(),
            opf_path: "OEBPS/content.opf".to_string(),
            opf_dir: "OEBPS".to_string(),
            metadata,
            manifest: HashMap::new(),
            spine,
            guide: Vec::new(),
            toc_item_id: None,
            nav_item_id: None,
        };

        let mut book = Book {
            archive: EpubArchive::empty(),
            opf,
            layout: RenditionLayout::default(),
            toc,
            landmarks: Vec::new(),
            page_list: Vec::new(),
            sections,
            locations: crate::locations::Locations::default(),
            annotations: crate::annotations::AnnotationManager::default(),
            before_display_hooks: Vec::new(),
            font_deobfuscator: FontDeobfuscator::parse_encryption_xml(""),
        };

        book.generate_locations(1000);
        Ok(book)
    }
}

fn split_mobi_html(html: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let lower = html.to_lowercase();

    let mut search_idx = 0;
    while let Some(pb_idx) = lower[search_idx..].find("<mbp:pagebreak") {
        let abs_idx = search_idx + pb_idx;
        if abs_idx > search_idx {
            let chunk = html[search_idx..abs_idx].trim();
            if !chunk.is_empty() {
                parts.push(chunk.to_string());
            }
        }
        if let Some(close) = html[abs_idx..].find('>') {
            search_idx = abs_idx + close + 1;
        } else {
            search_idx = abs_idx + 14;
        }
    }

    if search_idx < html.len() {
        let remainder = html[search_idx..].trim();
        if !remainder.is_empty() {
            parts.push(remainder.to_string());
        }
    }

    if parts.is_empty() {
        vec![html.to_string()]
    } else {
        parts
    }
}
