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

/// Microsoft Word (.docx) Office Open XML document parser engine.
pub struct DocxBook;

impl DocxBook {
    /// Parse Microsoft Word (.docx) archive bytes into a unified `Book` instance.
    pub fn parse(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
        let reader = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| EbookError::Zip(format!("Failed to open DOCX ZIP archive: {}", e)))?;

        // 1. Read word/document.xml
        let mut document_xml = String::new();
        if let Ok(mut file) = archive.by_name("word/document.xml") {
            file.read_to_string(&mut document_xml).map_err(|e| {
                EbookError::Io(format!("Failed to read word/document.xml in DOCX: {}", e))
            })?;
        } else {
            return Err(EbookError::InvalidFormat(
                "DOCX archive missing word/document.xml".to_string(),
            ));
        }

        let mut epub_archive = EpubArchive::empty();

        // 2. Extract embedded images from word/media/*
        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                let name = file.name().to_string();
                if name.starts_with("word/media/") && !name.ends_with('/') {
                    let mut img_data = Vec::new();
                    if file.read_to_end(&mut img_data).is_ok() {
                        let relative_name = name.strip_prefix("word/").unwrap_or(&name);
                        epub_archive.insert(relative_name, img_data);
                    }
                }
            }
        }

        // 3. Extract metadata from docProps/core.xml
        let mut title = title_fallback.to_string();
        let mut creators = Vec::new();
        let mut description = None;

        if let Ok(mut file) = archive.by_name("docProps/core.xml") {
            let mut core_xml = String::new();
            if file.read_to_string(&mut core_xml).is_ok() {
                if let Ok(doc) = Document::parse(&core_xml) {
                    for node in doc.descendants() {
                        if node.is_element() {
                            let tag = node.tag_name().name();
                            if tag == "title" {
                                if let Some(t) = node.text() {
                                    if !t.trim().is_empty() {
                                        title = t.trim().to_string();
                                    }
                                }
                            } else if tag == "creator" {
                                if let Some(c) = node.text() {
                                    if !c.trim().is_empty() {
                                        creators.push(c.trim().to_string());
                                    }
                                }
                            } else if tag == "description" {
                                if let Some(d) = node.text() {
                                    if !d.trim().is_empty() {
                                        description = Some(d.trim().to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Parse relationship map (word/_rels/document.xml.rels)
        let mut rels_map: AHashMap<String, String> = AHashMap::new();
        if let Ok(mut file) = archive.by_name("word/_rels/document.xml.rels") {
            let mut rels_xml = String::new();
            if file.read_to_string(&mut rels_xml).is_ok() {
                if let Ok(doc) = Document::parse(&rels_xml) {
                    for node in doc.descendants() {
                        if node.is_element() && node.tag_name().name() == "Relationship" {
                            if let (Some(id), Some(target)) =
                                (node.attribute("Id"), node.attribute("Target"))
                            {
                                rels_map.insert(id.to_string(), target.to_string());
                            }
                        }
                    }
                }
            }
        }

        // 5. Parse document.xml DOM
        let doc = Document::parse(&document_xml)
            .map_err(|e| EbookError::Xml(format!("Failed to parse DOCX document.xml: {}", e)))?;

        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();

        let mut current_html = String::new();
        let mut current_text = String::new();
        let mut section_index = 0;
        let mut current_heading = String::new();

        for node in doc.descendants() {
            if !node.is_element() {
                continue;
            }

            let tag = node.tag_name().name();

            if tag == "p" {
                // Check if paragraph is a Heading or contains page break
                let mut heading_level: Option<u32> = None;
                let mut has_page_break = false;

                for child in node.children() {
                    if child.is_element() {
                        if child.tag_name().name() == "pPr" {
                            for ppr_child in child.children() {
                                if ppr_child.is_element() && ppr_child.tag_name().name() == "pStyle"
                                {
                                    if let Some(val) = ppr_child
                                        .attribute((
                                            "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                                            "val",
                                        ))
                                        .or_else(|| ppr_child.attribute("val"))
                                    {
                                        let val_low = val.to_lowercase();
                                        if val_low == "heading1" || val_low == "heading 1" || val_low == "1" {
                                            heading_level = Some(1);
                                        } else if val_low == "heading2" || val_low == "heading 2" || val_low == "2" {
                                            heading_level = Some(2);
                                        } else if val_low == "heading3" || val_low == "heading 3" || val_low == "3" {
                                            heading_level = Some(3);
                                        }
                                    }
                                }
                            }
                        } else if child.tag_name().name() == "r" {
                            for r_child in child.children() {
                                if r_child.is_element() && r_child.tag_name().name() == "br" {
                                    if let Some(typ) = r_child.attribute((
                                        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                                        "type",
                                    )).or_else(|| r_child.attribute("type")) {
                                        if typ == "page" {
                                            has_page_break = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let p_text = extract_node_text(&node);
                let p_html = render_paragraph_html(&node, &rels_map);

                if (heading_level == Some(1) || has_page_break) && !current_text.is_empty() {
                    // Flush existing section
                    let sec_href = format!("section_{}.html", section_index);
                    let full_html =
                        format!("<div class=\"docx-section\">\n{}\n</div>", current_html);
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
                        href: sec_href.clone(),
                        media_type: "application/xhtml+xml".to_string(),
                    });

                    let label = if !current_heading.is_empty() {
                        current_heading.clone()
                    } else {
                        format!("Section {}", section_index + 1)
                    };

                    toc.push(NavPoint {
                        id: format!("nav_{}", toc.len() + 1),
                        label,
                        href: sec_href,
                        full_path: format!("section_{}.html", section_index),
                        subitems: Vec::new(),
                    });

                    section_index += 1;
                    current_html.clear();
                    current_text.clear();
                    current_heading.clear();
                }

                if let Some(level) = heading_level {
                    if level == 1 && current_heading.is_empty() {
                        current_heading = p_text.clone();
                    }
                    current_html.push_str(&format!(
                        "<h{}>{}</h{}>\n",
                        level,
                        xml_escape(&p_text),
                        level
                    ));
                } else if !p_html.is_empty() {
                    current_html.push_str(&format!("<p>{}</p>\n", p_html));
                }

                if !p_text.is_empty() {
                    current_text.push_str(&p_text);
                    current_text.push('\n');
                }
            } else if tag == "tbl" {
                let tbl_html = render_table_html(&node);
                current_html.push_str(&tbl_html);
            }
        }

        // Flush trailing section
        if !current_text.is_empty() || sections.is_empty() {
            let sec_href = format!("section_{}.html", section_index);
            let full_html = format!("<div class=\"docx-section\">\n{}\n</div>", current_html);
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
                href: sec_href.clone(),
                media_type: "application/xhtml+xml".to_string(),
            });

            let label = if !current_heading.is_empty() {
                current_heading
            } else {
                format!("Section {}", section_index + 1)
            };

            toc.push(NavPoint {
                id: format!("nav_{}", toc.len() + 1),
                label,
                href: sec_href.clone(),
                full_path: sec_href,
                subitems: Vec::new(),
            });
        }

        let metadata = Metadata {
            title,
            creators,
            publishers: Vec::new(),
            languages: vec!["en".to_string()],
            rights: None,
            description,
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
            archive: epub_archive,
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

fn extract_node_text(node: &roxmltree::Node) -> String {
    let mut buf = String::new();
    for desc in node.descendants() {
        if desc.is_element() && desc.tag_name().name() == "t" {
            if let Some(t) = desc.text() {
                buf.push_str(t);
            }
        }
    }
    buf
}

fn render_paragraph_html(node: &roxmltree::Node, rels: &AHashMap<String, String>) -> String {
    let mut html = String::new();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        let tag = child.tag_name().name();

        if tag == "r" {
            let mut is_bold = false;
            let mut is_italic = false;
            let mut is_underline = false;
            let mut is_strike = false;
            let mut text = String::new();
            let mut img_src: Option<String> = None;

            for r_child in child.children() {
                if !r_child.is_element() {
                    continue;
                }
                let r_tag = r_child.tag_name().name();

                if r_tag == "rPr" {
                    for pr in r_child.children() {
                        if !pr.is_element() {
                            continue;
                        }
                        match pr.tag_name().name() {
                            "b" => is_bold = true,
                            "i" => is_italic = true,
                            "u" => is_underline = true,
                            "strike" => is_strike = true,
                            _ => {}
                        }
                    }
                } else if r_tag == "t" {
                    if let Some(t) = r_child.text() {
                        text.push_str(t);
                    }
                } else if r_tag == "drawing" {
                    for desc in r_child.descendants() {
                        if desc.is_element() && desc.tag_name().name() == "blip" {
                            if let Some(embed_id) = desc
                                .attribute((
                                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
                                    "embed",
                                ))
                                .or_else(|| desc.attribute("r:embed"))
                            {
                                if let Some(target) = rels.get(embed_id) {
                                    img_src = Some(target.clone());
                                }
                            }
                        }
                    }
                }
            }

            if let Some(src) = img_src {
                html.push_str(&format!(
                    "<img src=\"{}\" alt=\"image\" />",
                    xml_escape(&src)
                ));
            }

            if !text.is_empty() {
                let mut escaped = xml_escape(&text);
                if is_strike {
                    escaped = format!("<s>{}</s>", escaped);
                }
                if is_underline {
                    escaped = format!("<u>{}</u>", escaped);
                }
                if is_italic {
                    escaped = format!("<em>{}</em>", escaped);
                }
                if is_bold {
                    escaped = format!("<strong>{}</strong>", escaped);
                }
                html.push_str(&escaped);
            }
        }
    }

    html
}

fn render_table_html(node: &roxmltree::Node) -> String {
    let mut out = String::from("<table>\n");

    for row in node.children() {
        if row.is_element() && row.tag_name().name() == "tr" {
            out.push_str("  <tr>\n");
            for cell in row.children() {
                if cell.is_element() && cell.tag_name().name() == "tc" {
                    let cell_text = extract_node_text(&cell);
                    out.push_str(&format!("    <td>{}</td>\n", xml_escape(&cell_text)));
                }
            }
            out.push_str("  </tr>\n");
        }
    }

    out.push_str("</table>\n");
    out
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
