use crate::archive::EpubArchive;
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::layout::RenditionLayout;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::{Section, extract_plain_text};
use std::collections::HashMap;

/// Microsoft Reader LIT Format Parser.
pub struct LitBook;

impl LitBook {
    /// Parse Microsoft Reader LIT file bytes into a `Book` struct.
    pub fn parse(bytes: &[u8]) -> Result<Book, String> {
        if bytes.len() < 8 {
            return Err("File too small for LIT header".to_string());
        }

        // LIT files start with ITOL / ITLS header or raw container stream
        if !bytes.starts_with(b"ITOL")
            && !bytes.starts_with(b"ITLS")
            && !bytes.contains_str(b"ITOL")
        {
            return Err("Not a valid Microsoft Reader LIT container".to_string());
        }

        // Extract HTML text strings embedded inside LIT binary container streams
        let text_content = extract_html_from_lit_bytes(bytes);
        let raw_lossy = String::from_utf8_lossy(bytes);

        let mut title = "LIT Document".to_string();
        let mut creators = Vec::new();

        if let Some(t) = extract_tag_content(&text_content, "title")
            .or_else(|| extract_tag_content(&raw_lossy, "dc:title"))
            .or_else(|| extract_meta_attr(&text_content, "title"))
            .or_else(|| extract_meta_attr(&raw_lossy, "title"))
        {
            if !t.trim().is_empty() {
                title = t.trim().to_string();
            }
        }

        if let Some(a) = extract_tag_content(&text_content, "author")
            .or_else(|| extract_tag_content(&raw_lossy, "dc:creator"))
            .or_else(|| extract_meta_attr(&text_content, "author"))
            .or_else(|| extract_meta_attr(&raw_lossy, "author"))
        {
            if !a.trim().is_empty() {
                creators.push(a.trim().to_string());
            }
        }

        if creators.is_empty() {
            creators.push("Unknown Author".to_string());
        }

        // Split text content into sections based on <h1>, <h2>, <h3> tag boundaries
        let raw_parts = split_lit_html(&text_content);
        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();

        for (idx, raw_sec_html) in raw_parts.into_iter().enumerate() {
            let idref = format!("lit_sec_{}", idx);
            let href = format!("lit_sec_{}.html", idx);
            let plain_text = extract_plain_text(&raw_sec_html);
            let plain_text_lower = plain_text.to_lowercase();
            let char_count = plain_text.chars().count();

            sections.push(Section {
                index: idx,
                idref: idref.clone(),
                href: href.clone(),
                full_path: href.clone(),
                raw_html: raw_sec_html.clone(),
                processed_html: raw_sec_html,
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

        let metadata = Metadata {
            title,
            creators,
            publishers: Vec::new(),
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
            accessibility: Default::default(),
        };

        let opf = OpfPackage {
            version: "2.0".to_string(),
            opf_path: "content.opf".to_string(),
            opf_dir: "".to_string(),
            metadata,
            manifest: ahash::AHashMap::new(),
            spine,
            guide: Vec::new(),
            toc_item_id: None,
            nav_item_id: None,
        };

        let mut archive = EpubArchive::empty();
        extract_images_from_lit_bytes(bytes, &mut archive);

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
            media_overlays: HashMap::new(),
            render_cache: parking_lot::Mutex::new(HashMap::new()),
        };

        book.generate_locations(1000);
        Ok(book)
    }
}

trait ContainsStr {
    fn contains_str(&self, needle: &[u8]) -> bool;
}

impl ContainsStr for [u8] {
    fn contains_str(&self, needle: &[u8]) -> bool {
        self.windows(needle.len()).any(|w| w == needle)
    }
}

fn extract_html_from_lit_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();

    // Check if container bytes are UTF-16LE encoded
    let text = if bytes.windows(2).any(|w| w == b"<\0") {
        let u16_data: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        String::from_utf16_lossy(&u16_data)
    } else {
        String::from_utf8_lossy(bytes).to_string()
    };

    for line in text.lines() {
        if line.contains('<') && line.contains('>') {
            out.push_str(line);
            out.push('\n');
        }
    }

    if out.is_empty() {
        "<p>LIT Document Content</p>".to_string()
    } else {
        out
    }
}

fn extract_tag_content(html: &str, tag_name: &str) -> Option<String> {
    let open_tag = format!("<{}", tag_name);
    let close_tag = format!("</{}>", tag_name);
    let lower = html.to_lowercase();

    if let Some(start_idx) = lower.find(&open_tag) {
        if let Some(tag_end) = lower[start_idx..].find('>') {
            let content_start = start_idx + tag_end + 1;
            if let Some(end_idx) = lower[content_start..].find(&close_tag) {
                let val = html[content_start..content_start + end_idx].trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn extract_meta_attr(html: &str, attr_name: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let pat = format!("name=\"{}\"", attr_name);
    if let Some(idx) = lower.find(&pat) {
        if let Some(content_idx) = lower[idx..].find("content=\"") {
            let val_start = idx + content_idx + 9;
            if let Some(val_end) = lower[val_start..].find('"') {
                let val = html[val_start..val_start + val_end].trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn split_lit_html(html: &str) -> Vec<String> {
    let lower = html.to_lowercase();
    let mut boundaries = vec![0];

    for tag in &["<h1", "<h2", "<h3", "<div class=\"chapter\""] {
        let mut pos = 0;
        while let Some(idx) = lower[pos..].find(tag) {
            let abs = pos + idx;
            if abs > 200 {
                boundaries.push(abs);
            }
            pos = abs + tag.len();
        }
    }

    boundaries.sort_unstable();
    boundaries.dedup();

    let mut parts = Vec::new();
    if boundaries.len() > 1 {
        for i in 0..boundaries.len() {
            let start = boundaries[i];
            let end = if i + 1 < boundaries.len() { boundaries[i + 1] } else { html.len() };
            let chunk = html[start..end].trim();
            if !chunk.is_empty() {
                parts.push(chunk.to_string());
            }
        }
    } else {
        parts.push(html.trim().to_string());
    }

    parts
}

fn extract_images_from_lit_bytes(bytes: &[u8], archive: &mut EpubArchive) -> usize {
    let mut image_count = 0;
    let mut i = 0;

    while i + 8 < bytes.len() {
        // PNG magic check: \x89PNG\r\n\x1a\n
        if &bytes[i..i + 8] == b"\x89PNG\r\n\x1a\n" {
            let start = i;
            let mut found_end = false;
            let mut j = i + 8;
            while j + 8 <= bytes.len() {
                if &bytes[j..j + 4] == b"IEND" {
                    let end = j + 8;
                    let img_data = bytes[start..end].to_vec();
                    image_count += 1;
                    let filename = format!("images/img_{:04}.png", image_count);
                    archive.insert(format!("OEBPS/{}", filename), img_data);
                    i = end;
                    found_end = true;
                    break;
                }
                j += 1;
            }
            if found_end {
                continue;
            }
        }

        // JPEG magic check: \xFF\xD8\xFF
        if bytes[i] == 0xFF && bytes[i + 1] == 0xD8 && bytes[i + 2] == 0xFF {
            let start = i;
            let mut j = i + 3;
            let mut found_end = false;
            while j + 2 <= bytes.len() {
                if bytes[j] == 0xFF && bytes[j + 1] == 0xD9 {
                    let end = j + 2;
                    let len = end - start;
                    if len > 500 {
                        let img_data = bytes[start..end].to_vec();
                        image_count += 1;
                        let filename = format!("images/img_{:04}.jpg", image_count);
                        archive.insert(format!("OEBPS/{}", filename), img_data);
                        i = end;
                        found_end = true;
                        break;
                    }
                }
                j += 1;
            }
            if found_end {
                continue;
            }
        }

        i += 1;
    }

    image_count
}


