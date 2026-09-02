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

/// Microsoft Reader LIT Format Parser.
pub struct LitBook;

impl LitBook {
    /// Parse Microsoft Reader LIT file bytes into a `Book` struct.
    pub fn parse(bytes: &[u8]) -> Result<Book, EbookError> {
        if bytes.len() < 8 {
            return Err(EbookError::InvalidFormat(
                "File too small for LIT header".to_string(),
            ));
        }

        // LIT files start with ITOL / ITLS header or raw container stream
        if !bytes.starts_with(b"ITOL")
            && !bytes.starts_with(b"ITLS")
            && !bytes.contains_str(b"ITOL")
        {
            return Err(EbookError::InvalidFormat(
                "Not a valid Microsoft Reader LIT container".to_string(),
            ));
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
            meta_properties: AHashMap::new(),
            accessibility: Default::default(),
        };

        let opf = OpfPackage {
            version: "2.0".to_string(),
            opf_path: "content.opf".to_string(),
            opf_dir: "".to_string(),
            metadata,
            manifest: AHashMap::new(),
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
            media_overlays: AHashMap::new(),
            render_cache: parking_lot::Mutex::new(AHashMap::new()),
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
        let trimmed = line.trim();
        if !trimmed.is_empty() && (trimmed.contains('<') || trimmed.len() > 15) {
            out.push_str(trimmed);
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
    let open_prefix = format!("<{}", tag_name);
    let close_tag = format!("</{}>", tag_name);
    let open_bytes = open_prefix.as_bytes();
    let close_bytes = close_tag.as_bytes();
    let html_bytes = html.as_bytes();

    for (start_byte, _) in html.char_indices() {
        if start_byte + open_bytes.len() <= html_bytes.len()
            && html_bytes[start_byte..start_byte + open_bytes.len()]
                .eq_ignore_ascii_case(open_bytes)
        {
            let slice = &html[start_byte..];
            let next_idx = open_bytes.len();
            if next_idx < slice.len() {
                let next_char = slice[next_idx..].chars().next().unwrap_or(' ');
                if next_char != '>' && !next_char.is_whitespace() {
                    continue;
                }
            }
            if let Some(tag_end) = slice.find('>') {
                let content_start = start_byte + tag_end + 1;
                let remaining = &html_bytes[content_start..];
                for (end_byte, _) in html[content_start..].char_indices() {
                    if end_byte + close_bytes.len() <= remaining.len()
                        && remaining[end_byte..end_byte + close_bytes.len()]
                            .eq_ignore_ascii_case(close_bytes)
                    {
                        let val = html[content_start..content_start + end_byte].trim();
                        if !val.is_empty() {
                            return Some(val.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_meta_name(html: &str, name: &str) -> Option<String> {
    let target = format!("name=\"{}\"", name);
    let target_bytes = target.as_bytes();
    let html_bytes = html.as_bytes();

    for (byte_idx, _) in html.char_indices() {
        if byte_idx + target_bytes.len() <= html_bytes.len()
            && html_bytes[byte_idx..byte_idx + target_bytes.len()]
                .eq_ignore_ascii_case(target_bytes)
        {
            let remaining = &html[byte_idx..];
            for (c_idx, _) in remaining.char_indices() {
                if c_idx + 9 <= remaining.len()
                    && remaining.as_bytes()[c_idx..c_idx + 9].eq_ignore_ascii_case(b"content=\"")
                {
                    let val_start = byte_idx + c_idx + 9;
                    if val_start < html.len() {
                        if let Some(val_end) = html[val_start..].find('"') {
                            let val = html[val_start..val_start + val_end].trim();
                            if !val.is_empty() {
                                return Some(val.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_meta_attr(html: &str, name: &str) -> Option<String> {
    extract_meta_name(html, name)
}

fn split_lit_html(html: &str) -> Vec<String> {
    let mut boundaries = vec![0];
    let html_bytes = html.as_bytes();

    let patterns = &[
        "<h1",
        "<h2",
        "<h3",
        "<div class=\"chapter\"",
        "chapter i.",
        "chapter ii.",
        "chapter iii.",
        "chapter iv.",
        "chapter v.",
        "chapter vi.",
        "chapter vii.",
        "chapter viii.",
        "chapter ix.",
        "chapter x.",
        "chapter xi.",
        "chapter xii.",
        "chapter i ",
        "chapter ii ",
        "chapter iii ",
        "chapter iv ",
        "chapter v ",
        "chapter vi ",
        "chapter vii ",
        "chapter viii ",
        "chapter ix ",
        "chapter x ",
        "chapter xi ",
        "chapter xii ",
        "chapter 1",
        "chapter 2",
        "chapter 3",
        "chapter 4",
        "chapter 5",
        "chapter 6",
        "chapter 7",
        "chapter 8",
        "chapter 9",
        "chapter 10",
    ];

    for (byte_idx, _) in html.char_indices() {
        if byte_idx > 200 {
            let b = html_bytes[byte_idx];
            if b != b'<' && b != b'c' && b != b'C' {
                continue;
            }
            let slice = &html[byte_idx..];
            for &tag in patterns {
                if let Some(sub) = slice.get(..tag.len()) {
                    if sub.eq_ignore_ascii_case(tag) {
                        boundaries.push(byte_idx);
                        break;
                    }
                }
            }
        }
    }

    boundaries.sort_unstable();
    boundaries.dedup();

    // Fallback: If no tag boundaries were found and the file is large, split into ~15KB chapter chunks at line breaks
    if boundaries.len() <= 1 && html.len() > 8000 {
        boundaries.clear();
        boundaries.push(0);
        let target_size = 15000;
        let mut pos = target_size;
        while pos < html.len() {
            while pos < html.len() && !html.is_char_boundary(pos) {
                pos += 1;
            }
            if pos >= html.len() {
                break;
            }
            let next_break = html[pos..]
                .find('\n')
                .or_else(|| html[pos..].find("<p"))
                .or_else(|| html[pos..].find("<P"))
                .or_else(|| html[pos..].find("<div"))
                .or_else(|| html[pos..].find("<DIV"));
            if let Some(break_offset) = next_break {
                let mut abs = pos + break_offset;
                while abs < html.len() && !html.is_char_boundary(abs) {
                    abs += 1;
                }
                boundaries.push(abs);
                pos = abs + target_size;
            } else {
                break;
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
    }

    let mut parts = Vec::new();
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
            if let Some(iend_pos) = memchr::memmem::find(&bytes[i + 8..], b"IEND") {
                let end = (i + 8 + iend_pos + 8).min(bytes.len());
                let img_data = bytes[start..end].to_vec();
                image_count += 1;
                let filename = format!("images/img_{:04}.png", image_count);
                archive.insert(format!("OEBPS/{}", filename), img_data);
                i = end;
                continue;
            }
        }

        // JPEG magic check: \xFF\xD8\xFF
        if bytes[i] == 0xFF && bytes[i + 1] == 0xD8 && bytes[i + 2] == 0xFF {
            let start = i;
            if let Some(eoi_pos) = memchr::memmem::find(&bytes[i + 3..], b"\xFF\xD9") {
                let end = i + 3 + eoi_pos + 2;
                let len = end - start;
                if len > 500 && len < 20 * 1024 * 1024 {
                    let img_data = bytes[start..end].to_vec();
                    image_count += 1;
                    let filename = format!("images/img_{:04}.jpg", image_count);
                    archive.insert(format!("OEBPS/{}", filename), img_data);
                    i = end;
                    continue;
                }
            }
        }

        i += 1;
    }

    image_count
}
