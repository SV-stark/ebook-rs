use crate::archive::EpubArchive;
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::layout::RenditionLayout;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::Section;
use std::collections::HashMap;
use std::io::{Cursor, Read};

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

        let mut reader = Cursor::new(bytes);
        let mut zip = zip::ZipArchive::new(&mut reader)
            .map_err(|e| format!("Failed to open CBZ archive: {}", e))?;

        let mut image_entries: Vec<(String, Vec<u8>)> = Vec::new();

        for i in 0..zip.len() {
            let mut file = zip
                .by_index(i)
                .map_err(|e| format!("Failed to read ZIP entry #{}: {}", i, e))?;
            let name = file.name().to_string();
            let lower = name.to_lowercase();

            if file.is_dir()
                || lower.contains("__macosx")
                || lower.contains("/.")
                || lower.starts_with('.')
            {
                continue;
            }

            let is_image = lower.ends_with(".jpg")
                || lower.ends_with(".jpeg")
                || lower.ends_with(".png")
                || lower.ends_with(".webp")
                || lower.ends_with(".gif")
                || lower.ends_with(".bmp")
                || lower.ends_with(".tif")
                || lower.ends_with(".tiff")
                || (!lower.ends_with(".xml")
                    && !lower.ends_with(".txt")
                    && !lower.ends_with(".json")
                    && !lower.ends_with(".html")
                    && !lower.ends_with(".htm")
                    && file.size() > 100);

            if is_image {
                let mut data = Vec::new();
                if file.read_to_end(&mut data).is_ok() && !data.is_empty() {
                    image_entries.push((name, data));
                }
            }
        }

        if image_entries.is_empty() {
            return Err("CBZ archive contains no valid image pages".to_string());
        }

        // Sort image pages naturally by filename
        image_entries.sort_by(|a, b| a.0.cmp(&b.0));

        let mut sections = Vec::with_capacity(image_entries.len());
        let mut spine = Vec::with_capacity(image_entries.len());
        let mut toc = Vec::with_capacity(image_entries.len());

        for (idx, (name, data)) in image_entries.into_iter().enumerate() {
            let mime = if name.to_lowercase().ends_with(".png") {
                "image/png"
            } else if name.to_lowercase().ends_with(".webp") {
                "image/webp"
            } else if name.to_lowercase().ends_with(".gif") {
                "image/gif"
            } else {
                "image/jpeg"
            };

            let b64 = base64_encode(&data);
            let raw_html = format!(
                "<div style=\"text-align:center;\"><img src=\"data:{};base64,{}\" style=\"max-width:100%;height:auto;\"/></div>",
                mime, b64
            );

            let idref = format!("page_{}", idx);
            let href = format!("page_{}.html", idx);
            let plain_text = format!("[Comic Page {} - {}]", idx + 1, name);
            let plain_text_lower = plain_text.to_lowercase();
            let char_count = plain_text.chars().count();

            sections.push(Section {
                index: idx,
                idref: idref.clone(),
                href: href.clone(),
                full_path: href.clone(),
                raw_html: raw_html.clone(),
                processed_html: raw_html,
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
            media_overlays: HashMap::new(),
            render_cache: parking_lot::Mutex::new(HashMap::new()),
        };

        book.generate_locations(1000);
        Ok(book)
    }
}

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
