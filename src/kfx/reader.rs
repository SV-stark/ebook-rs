use crate::book::Book;
use crate::kfx::container::KfxContainer;
use crate::metadata::{Metadata, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::Section;
use ahash::AHashMap;
use std::collections::HashMap;

/// Struct representing a parsed Amazon KFX (.kfx, .azw8) eBook file.
#[derive(Debug, Clone)]
pub struct KfxBook {
    pub metadata: Metadata,
    pub spine: Vec<SpineItem>,
    pub toc: Vec<NavPoint>,
    pub sections: Vec<Section>,
    pub resources: HashMap<String, Vec<u8>>,
}

impl KfxBook {
    /// Detect if a byte slice starts with valid Amazon KFX container header magic bytes.
    pub fn is_kfx(bytes: &[u8]) -> bool {
        KfxContainer::is_kfx(bytes)
    }

    /// Parse an Amazon KFX container from raw byte slice into structured metadata, spine, TOC, and HTML sections.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let container = KfxContainer::parse(bytes)?;
        let mut metadata = Metadata::default();
        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();
        let resources = HashMap::new();

        let mut text_fragments = Vec::new();
        let mut title_found = false;

        // Process Payload and Entities
        let text_scan = String::from_utf8_lossy(&container.payload);
        for line in text_scan.lines() {
            let line_trim = line.trim();
            if line_trim.contains("title") || line_trim.contains("Title") {
                if let Some(val) = extract_kv(line_trim) {
                    metadata.title = val;
                    title_found = true;
                }
            } else if line_trim.contains("author") || line_trim.contains("creator") {
                if let Some(val) = extract_kv(line_trim) {
                    if !metadata.creators.contains(&val) {
                        metadata.creators.push(val);
                    }
                }
            } else if line_trim.contains("publisher") {
                if let Some(val) = extract_kv(line_trim) {
                    if !metadata.publishers.contains(&val) {
                        metadata.publishers.push(val);
                    }
                }
            } else if line_trim.contains("language") {
                if let Some(val) = extract_kv(line_trim) {
                    metadata.languages = vec![val];
                }
            } else if line_trim.len() > 30
                && !line_trim.starts_with('{')
                && !line_trim.starts_with('[')
            {
                text_fragments.push(line_trim.to_string());
            }
        }

        if !title_found || metadata.title.is_empty() {
            metadata.title = "Amazon KFX Publication".to_string();
        }
        if metadata.languages.is_empty() {
            metadata.languages.push("en".to_string());
        }

        if text_fragments.is_empty() {
            let clean_text = crate::dom::sanitize_and_repair_xml(&text_scan);
            let sec_id = "kfx_sec_0".to_string();
            let raw_html = format!(
                "<div class=\"kfx-content\"><h1>{}</h1><p>{}</p></div>",
                metadata.title, clean_text
            );

            let section = Section {
                index: 0,
                idref: sec_id.clone(),
                href: "sec_0.xhtml".to_string(),
                full_path: "OEBPS/sec_0.xhtml".to_string(),
                raw_html: raw_html.clone(),
                processed_html: raw_html,
                plain_text: clean_text.clone(),
                plain_text_lower: clean_text.to_lowercase(),
                char_count: clean_text.chars().count(),
                viewport_width: None,
                viewport_height: None,
            };
            sections.push(section);

            spine.push(SpineItem {
                idref: sec_id,
                linear: true,
                properties: Vec::new(),
                index: 0,
                href: "sec_0.xhtml".to_string(),
                media_type: "application/xhtml+xml".to_string(),
            });

            toc.push(NavPoint {
                id: "toc_0".to_string(),
                label: metadata.title.clone(),
                href: "sec_0.xhtml".to_string(),
                full_path: "OEBPS/sec_0.xhtml".to_string(),
                subitems: Vec::new(),
            });
        } else {
            for (idx, frag) in text_fragments.into_iter().enumerate() {
                let sec_id = format!("kfx_sec_{}", idx);
                let href = format!("sec_{}.xhtml", idx);
                let full_path = format!("OEBPS/sec_{}.xhtml", idx);
                let raw_html = format!(
                    "<div class=\"kfx-section\"><h2>Section {}</h2><div>{}</div></div>",
                    idx + 1,
                    crate::dom::sanitize_and_repair_xml(&frag)
                );
                let plain_text = frag;

                let section = Section {
                    index: idx,
                    idref: sec_id.clone(),
                    href: href.clone(),
                    full_path: full_path.clone(),
                    raw_html: raw_html.clone(),
                    processed_html: raw_html,
                    plain_text: plain_text.clone(),
                    plain_text_lower: plain_text.to_lowercase(),
                    char_count: plain_text.chars().count(),
                    viewport_width: None,
                    viewport_height: None,
                };
                sections.push(section);

                spine.push(SpineItem {
                    idref: sec_id,
                    linear: true,
                    properties: Vec::new(),
                    index: idx,
                    href: href.clone(),
                    media_type: "application/xhtml+xml".to_string(),
                });

                toc.push(NavPoint {
                    id: format!("toc_{}", idx),
                    label: format!("Section {}", idx + 1),
                    href,
                    full_path,
                    subitems: Vec::new(),
                });
            }
        }

        Ok(Self {
            metadata,
            spine,
            toc,
            sections,
            resources,
        })
    }

    /// Parse Amazon KFX bytes into a standard `Book` instance.
    pub fn parse(bytes: &[u8]) -> Result<Book, String> {
        let kfx = Self::from_bytes(bytes)?;

        let archive = crate::archive::EpubArchive::empty();
        let opf = OpfPackage {
            version: "3.0".to_string(),
            opf_path: "OEBPS/content.opf".to_string(),
            opf_dir: "OEBPS".to_string(),
            metadata: kfx.metadata,
            manifest: AHashMap::new(),
            spine: kfx.spine,
            guide: Vec::new(),
            toc_item_id: None,
            nav_item_id: None,
        };

        let mut book = Book {
            archive,
            opf,
            toc: kfx.toc,
            landmarks: Vec::new(),
            page_list: Vec::new(),
            sections: kfx.sections,
            locations: crate::locations::Locations::default(),
            annotations: crate::annotations::AnnotationManager::default(),
            layout: crate::layout::RenditionLayout::default(),
            font_deobfuscator: crate::deobfuscate::FontDeobfuscator::default(),
            before_display_hooks: Vec::new(),
            media_overlays: std::collections::HashMap::new(),
            render_cache: parking_lot::Mutex::new(std::collections::HashMap::new()),
        };

        let mut locations = crate::locations::Locations::new(1000);
        for (idx, sec) in book.sections.iter().enumerate() {
            locations.add_spine_section(idx, &sec.plain_text);
        }
        book.locations = locations;

        Ok(book)
    }
}

fn extract_kv(line: &str) -> Option<String> {
    if let Some(pos) = line.find(':') {
        let val = line[pos + 1..]
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .trim_matches(',');
        if !val.is_empty() {
            return Some(val.to_string());
        }
    }
    None
}
