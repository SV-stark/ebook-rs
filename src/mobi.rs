use crate::archive::EpubArchive;
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::error::EbookError;
use crate::layout::RenditionLayout;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::{Section, extract_plain_text};
use ahash::AHashMap;

/// PalmDOC LZ77 Decompressor.
pub fn decompress_palmdoc(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 2);
    let mut i = 0;

    while i < data.len() {
        let byte = data[i];
        i += 1;

        match byte {
            0x00 | 0x09..=0x7F => {
                out.push(byte);
            }
            0x01..=0x08 => {
                let count = byte as usize;
                let end = (i + count).min(data.len());
                out.extend_from_slice(&data[i..end]);
                i += count;
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
                    } else {
                        out.push(byte);
                        out.push(next);
                    }
                } else {
                    out.push(byte);
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
    pub fn parse(bytes: &[u8]) -> Result<Book, EbookError> {
        if bytes.len() < 78 {
            return Err(EbookError::InvalidFormat(
                "File too small for MOBI/AZW3 PDB header".to_string(),
            ));
        }

        let name_bytes = &bytes[0..32];
        let name = String::from_utf8_lossy(name_bytes)
            .trim_matches('\0')
            .to_string();

        let num_records = u16::from_be_bytes([bytes[76], bytes[77]]) as usize;
        if bytes.len() < 78 + num_records * 8 {
            return Err(EbookError::InvalidFormat(
                "Truncated MOBI PDB record offset table".to_string(),
            ));
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
            return Err(EbookError::InvalidFormat(
                "MOBI archive has 0 records".to_string(),
            ));
        }

        let rec0_offset = record_offsets[0];
        let rec0_end = if record_offsets.len() > 1 {
            record_offsets[1]
        } else {
            bytes.len()
        };

        if rec0_offset >= bytes.len() || rec0_end > bytes.len() || rec0_offset + 16 > rec0_end {
            return Err(EbookError::InvalidFormat(
                "Invalid Record 0 bounds in MOBI header".to_string(),
            ));
        }

        let rec0 = &bytes[rec0_offset..rec0_end];
        let compression = u16::from_be_bytes([rec0[0], rec0[1]]);

        // Check Mobipocket DRM encryption field at rec0[12..14]
        if rec0.len() >= 14 {
            let encryption_type = u16::from_be_bytes([rec0[12], rec0[13]]);
            if encryption_type != 0 {
                return Err(EbookError::DrmProtected("Book is protected by Mobipocket DRM encryption. Please remove DRM to read or convert this book.".to_string()));
            }
        }

        let mut text_record_count = u16::from_be_bytes([rec0[8], rec0[9]]) as usize;
        let max_text_recs = if num_records > 1 { num_records - 1 } else { 1 };
        if text_record_count == 0 || text_record_count >= num_records {
            text_record_count = 1.min(max_text_recs);
        }

        let pdb_name = if name.is_empty() {
            "Untitled MOBI Book".to_string()
        } else {
            // PDB names use underscores instead of spaces — normalize as a fallback
            name.replace('_', " ")
        };
        let mut title = pdb_name.clone();
        let mut author = String::new();
        let mut publisher = None;
        let mut language = "en".to_string();

        if rec0.len() >= 40 && &rec0[16..20] == b"MOBI" {
            let header_len = u32::from_be_bytes([rec0[20], rec0[21], rec0[22], rec0[23]]) as usize;

            // Always probe for EXTH by magic bytes — some AZW3 files have flags=0 and version=0
            // but still contain a valid EXTH block immediately after the MOBI header.
            if header_len > 0 && rec0.len() >= 16 + header_len + 12 {
                let exth_offset = 16 + header_len;
                if exth_offset + 4 <= rec0.len() && &rec0[exth_offset..exth_offset + 4] == b"EXTH" {
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
                                100 => {
                                    if !val_str.is_empty() {
                                        author = val_str;
                                    }
                                }
                                101 => publisher = Some(val_str),
                                524 => {
                                    // Tag 524 = locale/language (BCP-47), always preferred
                                    if !val_str.is_empty() {
                                        language = val_str;
                                    }
                                }
                                106 => {
                                    // Tag 106 = publication date (often a datetime string)
                                    // Only use as language if it looks like a BCP-47 code
                                    let looks_like_lang = val_str.len() <= 8
                                        && val_str
                                            .chars()
                                            .all(|c| c.is_ascii_alphabetic() || c == '-');
                                    if looks_like_lang && language == "en" {
                                        language = val_str;
                                    }
                                }
                                // EXTH 503 = updated title (overrides PDB name)
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

            // Also try reading full-title from MOBI header full_name_offset / full_name_length
            // (bytes 84-87 = offset from record 0 start, bytes 88-91 = length)
            if title == pdb_name && rec0.len() >= 92 {
                let fn_offset =
                    u32::from_be_bytes([rec0[84], rec0[85], rec0[86], rec0[87]]) as usize;
                let fn_len = u32::from_be_bytes([rec0[88], rec0[89], rec0[90], rec0[91]]) as usize;
                if fn_offset > 0 && fn_len > 0 && fn_offset + fn_len <= rec0.len() {
                    let full_name = String::from_utf8_lossy(&rec0[fn_offset..fn_offset + fn_len])
                        .trim()
                        .to_string();
                    if !full_name.is_empty() {
                        title = full_name;
                    }
                }
            }
        }

        // Fallback author if EXTH didn't provide one
        if author.is_empty() {
            author = "Unknown Author".to_string();
        }

        let first_image_index = if rec0.len() >= 112 && &rec0[16..20] == b"MOBI" {
            let img_rec_val =
                u32::from_be_bytes([rec0[108], rec0[109], rec0[110], rec0[111]]) as usize;
            if img_rec_val > 0 && img_rec_val < num_records {
                img_rec_val
            } else {
                1 + text_record_count
            }
        } else {
            1 + text_record_count
        };

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

        let full_html_raw = match std::str::from_utf8(&raw_html_bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(&raw_html_bytes);
                cow.into_owned()
            }
        };
        let mut full_html = full_html_raw;
        if full_html.trim().is_empty() || extract_plain_text(&full_html).trim().is_empty() {
            full_html = extract_fallback_mobi_text(bytes);
        }
        full_html = sanitize_mobi_control_chars(&full_html);

        let mut archive = EpubArchive::empty();
        let full_html = extract_mobi_images_and_populate_archive(
            &full_html,
            bytes,
            &record_offsets,
            first_image_index,
            &mut archive,
        );

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
                viewport_width: None,
                viewport_height: None,
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

        let lang_lower = language.to_lowercase();
        let direction = if lang_lower.starts_with("ar")
            || lang_lower.starts_with("he")
            || lang_lower.starts_with("fa")
            || lang_lower.starts_with("ur")
        {
            PageProgressionDirection::Rtl
        } else {
            PageProgressionDirection::Ltr
        };

        let metadata = Metadata {
            title,
            creators: vec![author],
            publishers: publisher.map(|p| vec![p]).unwrap_or_default(),
            languages: vec![language],
            rights: None,
            description: None,
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects: Vec::new(),
            cover_id: None,
            cover_href: None,
            direction,
            meta_properties: AHashMap::new(),
            accessibility: Default::default(),
        };

        let opf = OpfPackage {
            version: "2.0".to_string(),
            opf_path: "OEBPS/content.opf".to_string(),
            opf_dir: "OEBPS".to_string(),
            metadata,
            manifest: AHashMap::new(),
            spine,
            guide: Vec::new(),
            toc_item_id: None,
            nav_item_id: None,
        };

        let mut book = Book {
            archive,
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
            media_overlays: AHashMap::new(),
            render_cache: parking_lot::Mutex::new(AHashMap::new()),
        };

        book.generate_locations(1000);
        Ok(book)
    }
}

fn find_ignore_case(s: &str, pat: &str) -> Option<usize> {
    if pat.is_empty() {
        return Some(0);
    }
    if s.len() < pat.len() {
        return None;
    }
    for i in 0..=s.len() - pat.len() {
        if s.is_char_boundary(i) {
            if let Some(sub) = s.get(i..i + pat.len()) {
                if sub.eq_ignore_ascii_case(pat) {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn split_mobi_html(html: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut search_idx = 0;

    // Primary split: <mbp:pagebreak> (MOBI)
    while search_idx < html.len() {
        if let Some(pb_idx) = find_ignore_case(&html[search_idx..], "<mbp:pagebreak") {
            let abs_idx = search_idx + pb_idx;
            if abs_idx > search_idx {
                let chunk = html[search_idx..abs_idx].trim();
                if !chunk.is_empty() {
                    parts.push(chunk.to_string());
                }
            }
            if let Some(abs_close) = crate::section::find_tag_end(html, abs_idx) {
                search_idx = abs_close + 1;
            } else {
                search_idx = abs_idx + "<mbp:pagebreak".len();
            }
        } else {
            break;
        }
    }

    if search_idx < html.len() {
        let remainder = html[search_idx..].trim();
        if !remainder.is_empty() {
            parts.push(remainder.to_string());
        }
    }

    // If no <mbp:pagebreak> splits were found (AZW3), fall back to splitting
    // on heading-level anchors: <a id="..."> that precede <h1>/<h2>/<h3> tags.
    // We collect boundary indices where a new chapter begins.
    if parts.len() <= 1 && !html.trim().is_empty() {
        parts.clear();
        let lower = html.to_lowercase();
        let mut boundaries: Vec<usize> = vec![0];

        // Find all <h1>, <h2>, <h3> tag positions as chapter split points
        for tag in &["<h1", "<h2", "<h3"] {
            let mut pos = 0;
            while let Some(idx) = lower[pos..].find(tag) {
                let abs = pos + idx;
                // Skip if it's very close to the start (< 200 chars — likely a book title header)
                if abs > 200 {
                    boundaries.push(abs);
                }
                pos = abs + tag.len();
            }
        }

        boundaries.sort_unstable();
        boundaries.dedup();

        if boundaries.len() > 1 {
            for i in 0..boundaries.len() {
                let start = boundaries[i];
                let end = if i + 1 < boundaries.len() {
                    boundaries[i + 1]
                } else {
                    html.len()
                };
                let chunk = html[start..end].trim();
                if !chunk.is_empty() {
                    parts.push(chunk.to_string());
                }
            }
        } else {
            // Last resort: single section
            parts.push(html.trim().to_string());
        }
    }

    parts
}

fn extract_fallback_mobi_text(bytes: &[u8]) -> String {
    let lossy = String::from_utf8_lossy(bytes);
    let mut out = String::new();
    for line in lossy.lines() {
        let trimmed = line.trim();
        if trimmed.len() > 10 && trimmed.chars().any(|c| c.is_alphabetic()) {
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    if out.is_empty() {
        "<p>AZW3 Book Content</p>".to_string()
    } else {
        out
    }
}

fn sanitize_mobi_control_chars(input: &str) -> String {
    input
        .chars()
        .filter(|&c| c == '\n' || c == '\r' || c == '\t' || !c.is_control())
        .collect()
}

fn extract_mobi_images_and_populate_archive(
    html: &str,
    bytes: &[u8],
    record_offsets: &[usize],
    first_image_index: usize,
    archive: &mut EpubArchive,
) -> String {
    let mut image_map: AHashMap<usize, String> = AHashMap::new();
    let num_records = record_offsets.len();

    let start_img_rec = first_image_index.max(1);
    for rec_idx in start_img_rec..num_records {
        let rec_start = record_offsets[rec_idx];
        let rec_end = if rec_idx + 1 < num_records {
            record_offsets[rec_idx + 1]
        } else {
            bytes.len()
        };

        if rec_start < bytes.len() && rec_end <= bytes.len() && rec_start < rec_end {
            let img_bytes = &bytes[rec_start..rec_end];
            if let Some(mime) = detect_image_mime(img_bytes) {
                let ext = match mime {
                    "image/jpeg" => "jpg",
                    "image/png" => "png",
                    "image/gif" => "gif",
                    "image/webp" => "webp",
                    "image/bmp" => "bmp",
                    _ => "jpg",
                };
                let img_num = (rec_idx - start_img_rec) + 1;
                let rel_path = format!("images/img_{:04}.{}", img_num, ext);
                let full_archive_path = format!("OEBPS/{}", rel_path);
                archive.insert(full_archive_path, img_bytes.to_vec());
                archive.insert(rel_path.clone(), img_bytes.to_vec());
                image_map.insert(img_num, rel_path);
            }
        }
    }

    if image_map.is_empty() {
        return html.to_string();
    }

    let mut output = html.to_string();

    // Sort image entries in DESCENDING numerical order so that multi-digit indices are replaced before single-digit indices
    let mut sorted_entries: Vec<(usize, String)> = image_map.into_iter().collect();
    sorted_entries.sort_by_key(|b| std::cmp::Reverse(b.0));

    for (img_num, rel_path) in sorted_entries {
        let rec_str1 = format!("recindex=\"{}\"", img_num);
        let rec_str2 = format!("recindex=\"{:05}\"", img_num);
        output = output.replace(&rec_str1, &format!("src=\"{}\"", rel_path));
        output = output.replace(&rec_str2, &format!("src=\"{}\"", rel_path));

        let kindle_str1 = format!("kindle:embed:{:04}", img_num);
        let kindle_str2 = format!("kindle:embed:{:05}", img_num);
        let kindle_str3 = format!("kindle:embed:{}", img_num);
        output = output.replace(&kindle_str1, &rel_path);
        output = output.replace(&kindle_str2, &rel_path);
        output = output.replace(&kindle_str3, &rel_path);

        let file_str1 = format!("src=\"{:05}.jpg\"", img_num);
        let file_str2 = format!("src=\"{:04}.jpg\"", img_num);
        let file_str3 = format!("src=\"{}.jpg\"", img_num);
        output = output.replace(&file_str1, &format!("src=\"{}\"", rel_path));
        output = output.replace(&file_str2, &format!("src=\"{}\"", rel_path));
        output = output.replace(&file_str3, &format!("src=\"{}\"", rel_path));
    }

    output
}

fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 4 {
        return None;
    }
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.starts_with(b"BM") {
        Some("image/bmp")
    } else {
        None
    }
}
