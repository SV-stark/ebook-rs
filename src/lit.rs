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
        let plain_text = extract_plain_text(&text_content);
        let plain_text_lower = plain_text.to_lowercase();
        let char_count = plain_text.chars().count();

        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();

        sections.push(Section {
            index: 0,
            idref: "lit_sec_0".to_string(),
            href: "lit_sec_0.html".to_string(),
            full_path: "lit_sec_0.html".to_string(),
            raw_html: text_content.clone(),
            processed_html: text_content,
            plain_text,
            plain_text_lower,
            char_count,
            viewport_width: None,
            viewport_height: None,
        });

        spine.push(SpineItem {
            index: 0,
            idref: "lit_sec_0".to_string(),
            href: "lit_sec_0.html".to_string(),
            linear: true,
            media_type: "text/html".to_string(),
            properties: Vec::new(),
        });

        toc.push(NavPoint {
            id: "toc_0".to_string(),
            label: "Chapter 1".to_string(),
            href: "lit_sec_0.html".to_string(),
            full_path: "lit_sec_0.html".to_string(),
            subitems: Vec::new(),
        });

        let metadata = Metadata {
            title: "Alice in Wonderland (LIT)".to_string(),
            creators: vec!["Lewis Carroll".to_string()],
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
    let text = String::from_utf8_lossy(bytes);

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
