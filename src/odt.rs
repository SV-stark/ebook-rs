use crate::archive::EpubArchive;
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::error::EbookError;
use crate::layout::RenditionLayout;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::Section;
use ahash::AHashMap;
use roxmltree::Document;
use std::io::Read;

/// OpenDocument Text (.odt) document parser engine.
pub struct OdtBook;

impl OdtBook {
    /// Parse OpenDocument Text (.odt) archive bytes into a `Book` instance.
    pub fn parse(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
        let reader = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| EbookError::Zip(format!("Failed to open ODT ZIP archive: {}", e)))?;

        let mut content_xml = String::new();
        if let Ok(mut file) = archive.by_name("content.xml") {
            file.read_to_string(&mut content_xml)
                .map_err(|e| EbookError::Io(format!("Failed to read content.xml in ODT: {}", e)))?;
        } else {
            return Err(EbookError::InvalidFormat(
                "ODT archive missing content.xml".to_string(),
            ));
        }

        let mut title = title_fallback.to_string();
        let mut creator = String::new();

        if let Ok(mut file) = archive.by_name("meta.xml") {
            let mut meta_xml = String::new();
            if file.read_to_string(&mut meta_xml).is_ok() {
                if let Ok(doc) = Document::parse(&meta_xml) {
                    for node in doc.descendants() {
                        if node.is_element() {
                            if node.tag_name().name() == "title" {
                                if let Some(t) = node.text() {
                                    if !t.trim().is_empty() {
                                        title = t.trim().to_string();
                                    }
                                }
                            } else if node.tag_name().name() == "initial-creator"
                                || node.tag_name().name() == "creator"
                            {
                                if let Some(c) = node.text() {
                                    if !c.trim().is_empty() {
                                        creator = c.trim().to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let doc = Document::parse(&content_xml)
            .map_err(|e| format!("Failed to parse ODT content.xml XML: {}", e))?;

        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();
        let mut current_html = String::new();
        let mut current_text = String::new();
        let mut section_index = 0;
        let mut current_heading = String::new();

        fn collect_descendant_text(node: &roxmltree::Node) -> String {
            let mut buf = String::new();
            for descendant in node.descendants() {
                if descendant.is_text() {
                    if let Some(t) = descendant.text() {
                        buf.push_str(t);
                    }
                }
            }
            buf
        }

        fn xml_escape(input: &str) -> String {
            input
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&apos;")
        }

        for node in doc.descendants() {
            if node.is_element() {
                let tag_name = node.tag_name().name();
                if tag_name == "h" {
                    let text_raw = collect_descendant_text(&node);
                    let h_text = text_raw.trim();
                    if !h_text.is_empty() {
                        if title == title_fallback && current_heading.is_empty() {
                            title = h_text.to_string();
                        }

                        if !current_text.is_empty() {
                            let sec_href = format!("section_{}.html", section_index);
                            let full_html =
                                format!("<div class=\"odt-content\">\n{}\n</div>", current_html);

                            let char_count = current_text.chars().count();
                            let plain_text_lower = current_text.to_lowercase();

                            sections.push(Section {
                                index: section_index,
                                idref: format!("sec_{}", section_index),
                                href: sec_href.clone(),
                                full_path: sec_href.clone(),
                                raw_html: full_html.clone(),
                                processed_html: full_html,
                                plain_text: current_text.clone(),
                                plain_text_lower,
                                char_count,
                                viewport_width: None,
                                viewport_height: None,
                            });

                            spine.push(SpineItem {
                                idref: format!("sec_{}", section_index),
                                linear: true,
                                properties: Vec::new(),
                                index: section_index,
                                href: sec_href,
                                media_type: "application/xhtml+xml".to_string(),
                            });

                            section_index += 1;
                            current_html.clear();
                            current_text.clear();
                        }

                        current_heading = h_text.to_string();
                        current_html.push_str(&format!("<h2>{}</h2>\n", xml_escape(h_text)));
                        current_text.push_str(h_text);
                        current_text.push('\n');

                        let href = format!("section_{}.html", section_index);
                        toc.push(NavPoint {
                            id: format!("nav_{}", toc.len() + 1),
                            label: h_text.to_string(),
                            href: href.clone(),
                            full_path: href,
                            subitems: Vec::new(),
                        });
                    }
                } else if tag_name == "p" {
                    let text_raw = collect_descendant_text(&node);
                    let p_text = text_raw.trim();
                    if !p_text.is_empty() {
                        current_html.push_str(&format!("<p>{}</p>\n", xml_escape(p_text)));
                        current_text.push_str(p_text);
                        current_text.push('\n');
                    }
                }
            }
        }

        if !current_text.is_empty() || sections.is_empty() {
            let sec_href = format!("section_{}.html", section_index);
            let full_html = format!("<div class=\"odt-content\">\n{}\n</div>", current_html);

            let char_count = current_text.chars().count();
            let plain_text_lower = current_text.to_lowercase();

            sections.push(Section {
                index: section_index,
                idref: format!("sec_{}", section_index),
                href: sec_href.clone(),
                full_path: sec_href.clone(),
                raw_html: full_html.clone(),
                processed_html: full_html,
                plain_text: current_text,
                plain_text_lower,
                char_count,
                viewport_width: None,
                viewport_height: None,
            });

            spine.push(SpineItem {
                idref: format!("sec_{}", section_index),
                linear: true,
                properties: Vec::new(),
                index: section_index,
                href: sec_href,
                media_type: "application/xhtml+xml".to_string(),
            });
        }

        let mut creators = Vec::new();
        if !creator.is_empty() {
            creators.push(creator);
        }

        let metadata = Metadata {
            title,
            creators,
            publishers: Vec::new(),
            languages: vec!["en".to_string()],
            rights: None,
            description: Some("OpenDocument Text".to_string()),
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects: vec!["Document".to_string()],
            cover_id: None,
            cover_href: None,
            direction: PageProgressionDirection::Ltr,
            meta_properties: AHashMap::new(),
            accessibility: Default::default(),
        };

        let opf = OpfPackage {
            version: "3.0".to_string(),
            opf_path: "content.opf".to_string(),
            opf_dir: "".to_string(),
            metadata,
            manifest: AHashMap::new(),
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
            media_overlays: AHashMap::new(),
            render_cache: parking_lot::Mutex::new(AHashMap::new()),
        };

        book.generate_locations(1000);
        Ok(book)
    }
}
