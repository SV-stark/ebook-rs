use crate::archive::EpubArchive;
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::layout::RenditionLayout;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::Section;
use std::collections::HashMap;

/// Comic Book Archive (CBZ / CBR) Parser.
pub struct CbzBook;

impl CbzBook {
    /// Parse CBZ (ZIP) or raw image container bytes into a `Book` struct.
    pub fn parse(bytes: &[u8], title_fallback: &str) -> Result<Book, String> {
        if bytes.starts_with(b"Rar!\x1a\x07\x00")
            || bytes.starts_with(b"Rar!\x1a\x07\x01\x00")
            || bytes.starts_with(b"Rar!\x1a\x07")
        {
            return Err("CBR (RAR format) is not supported in pure-Rust mode (RARv4/RARv5 detected). Please convert the file to CBZ (ZIP format).".to_string());
        }

        let archive = EpubArchive::from_bytes(bytes)?;
        Self::from_archive(archive, title_fallback)
    }

    /// Parse a CBZ comic book directly from an already-extracted in-memory archive container.
    pub fn from_archive(archive: EpubArchive, title_fallback: &str) -> Result<Book, String> {
        let mut image_names: Vec<String> = archive
            .files()
            .keys()
            .filter(|name| {
                let lower = name.to_lowercase();
                !lower.contains("__macosx")
                    && !lower.contains("/.")
                    && !lower.starts_with('.')
                    && (lower.ends_with(".jpg")
                        || lower.ends_with(".jpeg")
                        || lower.ends_with(".png")
                        || lower.ends_with(".webp")
                        || lower.ends_with(".gif")
                        || lower.ends_with(".bmp")
                        || lower.ends_with(".tif")
                        || lower.ends_with(".tiff"))
            })
            .cloned()
            .collect();

        if image_names.is_empty() {
            return Err("CBZ archive contains no valid image pages".to_string());
        }

        // Sort image pages naturally by filename (e.g. page2 before page10)
        image_names.sort_by(|a, b| natural_cmp(a, b));

        let mut sections = Vec::with_capacity(image_names.len());
        let mut spine = Vec::with_capacity(image_names.len());
        let mut toc = Vec::with_capacity(image_names.len());

        for (idx, img_path) in image_names.into_iter().enumerate() {
            let idref = format!("page_{}", idx);
            let href = format!("page_{}.html", idx);

            let raw_html = format!(
                "<div style=\"text-align:center;\"><img src=\"{}\" style=\"max-width:100%;height:auto;\"/></div>",
                img_path
            );
            let processed_html = raw_html.clone();

            let plain_text = format!("[Comic Page {} - {}]", idx + 1, img_path);
            let plain_text_lower = plain_text.to_lowercase();
            let char_count = plain_text.chars().count();

            sections.push(Section {
                index: idx,
                idref: idref.clone(),
                href: href.clone(),
                full_path: href.clone(),
                raw_html,
                processed_html,
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
                media_type: "application/xhtml+xml".to_string(),
                properties: Vec::new(),
            });

            toc.push(NavPoint {
                id: format!("toc_{}", idx),
                label: format!("Page {}", idx + 1),
                href: href.clone(),
                full_path: href,
                subitems: Vec::new(),
            });
        }

        let metadata = Metadata {
            title: title_fallback.to_string(),
            creators: vec!["Comic Author".to_string()],
            publishers: Vec::new(),
            languages: vec!["en".to_string()],
            rights: None,
            description: Some("Comic Book Archive".to_string()),
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects: vec!["Comics".to_string()],
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

    /// Parse a CBZ comic book archive in Manga mode (Right-to-Left reading progression).
    pub fn parse_manga(bytes: &[u8], title_fallback: &str) -> Result<Book, String> {
        let mut book = Self::parse(bytes, title_fallback)?;
        Self::enable_manga_mode(&mut book);
        Ok(book)
    }

    /// Enable 2-Page Manga Spread mode (Right-to-Left reading progression).
    pub fn enable_manga_mode(book: &mut Book) {
        book.opf.metadata.direction = PageProgressionDirection::Rtl;
        for sec in &mut book.sections {
            if !sec.processed_html.contains("dir=\"rtl\"") {
                sec.processed_html = sec
                    .processed_html
                    .replace("<div", "<div dir=\"rtl\" class=\"manga-spread\"");
            }
        }
    }

    /// Pre-fetch image byte payloads for adjacent comic pages to enable zero-latency page turns.
    pub fn prefetch_page_images(
        book: &Book,
        current_index: usize,
        window: usize,
    ) -> Vec<(usize, String, Vec<u8>)> {
        let mut result = Vec::new();
        let end = (current_index + window).min(book.sections.len());

        for idx in current_index..end {
            if let Some(sec) = book.sections.get(idx) {
                if let Some(img_src) = extract_img_src_from_html(&sec.raw_html) {
                    if let Ok((bytes, _mime)) = book.get_resource_bytes(&img_src) {
                        result.push((idx, img_src, bytes));
                    }
                }
            }
        }
        result
    }
}

fn extract_img_src_from_html(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    if let Some(pos) = lower.find("src=\"") {
        let start = pos + 5;
        if let Some(end) = html[start..].find('"') {
            return Some(html[start..start + end].to_string());
        }
    }
    None
}

#[allow(dead_code)]
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0;

    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as u32
        } else {
            0
        };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);

        if i + 1 < data.len() {
            out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        if i + 2 < data.len() {
            out.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        i += 3;
    }

    out
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let mut a_num: u64 = 0;
                    while let Some(d) = a_chars.peek().and_then(|c| c.to_digit(10)) {
                        a_num = a_num.saturating_mul(10).saturating_add(d as u64);
                        a_chars.next();
                    }
                    let mut b_num: u64 = 0;
                    while let Some(d) = b_chars.peek().and_then(|c| c.to_digit(10)) {
                        b_num = b_num.saturating_mul(10).saturating_add(d as u64);
                        b_chars.next();
                    }
                    if a_num != b_num {
                        return a_num.cmp(&b_num);
                    }
                } else {
                    let ca_lower = ca.to_ascii_lowercase();
                    let cb_lower = cb.to_ascii_lowercase();
                    if ca_lower != cb_lower {
                        return ca_lower.cmp(&cb_lower);
                    }
                    a_chars.next();
                    b_chars.next();
                }
            }
        }
    }
}
