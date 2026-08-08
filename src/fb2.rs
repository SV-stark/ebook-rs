use crate::archive::EpubArchive;
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::layout::RenditionLayout;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::{Section, extract_plain_text};
use base64::Engine;
use roxmltree::Document;
use std::collections::HashMap;

/// FictionBook 2 (FB2) XML Parser.
pub struct Fb2Book;

impl Fb2Book {
    /// Parse FB2 raw XML bytes into a `Book` struct.
    pub fn parse(bytes: &[u8]) -> Result<Book, String> {
        let xml = String::from_utf8_lossy(bytes);
        let doc = Document::parse(&xml).map_err(|e| format!("FB2 XML parse error: {}", e))?;

        let root = doc.root_element();
        if !root.has_tag_name("FictionBook") && !xml.contains("<FictionBook") {
            return Err("Not a valid FictionBook 2 (FB2) document".to_string());
        }

        let mut title = "Untitled FB2 Book".to_string();
        let mut creators = Vec::new();
        let mut publisher = None;
        let mut language = None;

        // Parse <description> metadata
        for desc in root.descendants() {
            if desc.has_tag_name("title-info") {
                for child in desc.children() {
                    if !child.is_element() {
                        continue;
                    }
                    match child.tag_name().name() {
                        "book-title" => {
                            title = child.text().unwrap_or("").trim().to_string();
                        }
                        "author" => {
                            let mut name_parts = Vec::new();
                            for a in child.children() {
                                if let Some(t) = a.text() {
                                    if !t.trim().is_empty() {
                                        name_parts.push(t.trim());
                                    }
                                }
                            }
                            if !name_parts.is_empty() {
                                creators.push(name_parts.join(" "));
                            }
                        }
                        "lang" => {
                            language = child.text().map(|s| s.trim().to_string());
                        }
                        "publisher" => {
                            publisher = child.text().map(|s| s.trim().to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        if creators.is_empty() {
            creators.push("Unknown Author".to_string());
        }

        // Extract <binary> Base64 images into archive and binary_map
        let mut binary_map = HashMap::new();
        let mut archive = EpubArchive::empty();

        for node in root.descendants() {
            if node.has_tag_name("binary") {
                if let Some(id) = node.attribute("id") {
                    let text_raw = collect_descendant_text(&node);
                    let b64_clean = text_raw.replace(['\r', '\n', ' ', '\t'], "");
                    let mime = node.attribute("content-type").unwrap_or("image/jpeg");
                    let ext = match mime {
                        "image/jpeg" => "jpg",
                        "image/png" => "png",
                        "image/gif" => "gif",
                        "image/webp" => "webp",
                        "image/svg+xml" => "svg",
                        _ => "jpg",
                    };

                    let clean_id = id.trim_start_matches('#').to_string();
                    let filename = if clean_id.contains('.') {
                        clean_id.clone()
                    } else {
                        format!("{}.{}", clean_id, ext)
                    };

                    let rel_path = format!("images/{}", filename);
                    let full_archive_path = format!("OEBPS/{}", rel_path);

                    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&b64_clean) {
                        archive.insert(full_archive_path, bytes);
                        binary_map.insert(clean_id, rel_path);
                    }
                }
            }
        }

        // Extract sections from <body> tags
        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();
        let mut sec_idx = 0;

        for body in root.descendants() {
            if body.has_tag_name("body") {
                for sec in body.children() {
                    if sec.has_tag_name("section") {
                        let sec_html = convert_fb2_section_to_html(&sec, &binary_map);
                        let idref = format!("fb2_sec_{}", sec_idx);
                        let href = format!("fb2_sec_{}.html", sec_idx);
                        let plain_text = extract_plain_text(&sec_html);
                        let plain_text_lower = plain_text.to_lowercase();
                        let char_count = plain_text.chars().count();

                        sections.push(Section {
                            index: sec_idx,
                            idref: idref.clone(),
                            href: href.clone(),
                            full_path: href.clone(),
                            raw_html: sec_html.clone(),
                            processed_html: sec_html,
                            plain_text,
                            plain_text_lower,
                            char_count,
                            viewport_width: None,
                            viewport_height: None,
                        });

                        spine.push(SpineItem {
                            index: sec_idx,
                            idref,
                            href: href.clone(),
                            linear: true,
                            media_type: "application/xhtml+xml".to_string(),
                            properties: Vec::new(),
                        });

                        toc.push(NavPoint {
                            id: format!("toc_{}", sec_idx),
                            label: format!("Chapter {}", sec_idx + 1),
                            href: href.clone(),
                            full_path: href,
                            subitems: Vec::new(),
                        });

                        sec_idx += 1;
                    }
                }
            }
        }

        if sections.is_empty() {
            let full_plain = extract_plain_text(&xml);
            let plain_lower = full_plain.to_lowercase();
            let char_count = full_plain.chars().count();

            sections.push(Section {
                index: 0,
                idref: "fb2_sec_0".to_string(),
                href: "fb2_sec_0.html".to_string(),
                full_path: "fb2_sec_0.html".to_string(),
                raw_html: xml.to_string(),
                processed_html: "<p>FB2 Content</p>".to_string(),
                plain_text: full_plain,
                plain_text_lower: plain_lower,
                char_count,
                viewport_width: None,
                viewport_height: None,
            });

            spine.push(SpineItem {
                index: 0,
                idref: "fb2_sec_0".to_string(),
                href: "fb2_sec_0.html".to_string(),
                linear: true,
                media_type: "application/xhtml+xml".to_string(),
                properties: Vec::new(),
            });

            toc.push(NavPoint {
                id: "toc_0".to_string(),
                label: "Section 1".to_string(),
                href: "fb2_sec_0.html".to_string(),
                full_path: "fb2_sec_0.html".to_string(),
                subitems: Vec::new(),
            });
        }

        let metadata = Metadata {
            title,
            creators,
            publishers: publisher.map(|p| vec![p]).unwrap_or_default(),
            languages: language
                .map(|l| vec![l])
                .unwrap_or_else(|| vec!["en".to_string()]),
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

fn convert_fb2_section_to_html(
    sec: &roxmltree::Node,
    binary_map: &HashMap<String, String>,
) -> String {
    let mut html = String::from("<div>");

    for node in sec.children() {
        if !node.is_element() {
            continue;
        }
        match node.tag_name().name() {
            "title" => {
                let text = collect_descendant_text(&node);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    html.push_str(&format!("<h2>{}</h2>", xml_escape(trimmed)));
                }
            }
            "p" => {
                let text = collect_descendant_text(&node);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    html.push_str(&format!("<p>{}</p>", xml_escape(trimmed)));
                }
            }
            "image" => {
                let href_opt = node
                    .attribute("href")
                    .or_else(|| node.attribute("l:href"))
                    .or_else(|| node.attribute("xlink:href"))
                    .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")));

                if let Some(href) = href_opt {
                    let key = href.trim_start_matches('#');
                    let data_uri = binary_map
                        .get(key)
                        .or_else(|| binary_map.get(&format!("#{}", key)))
                        .or_else(|| binary_map.get(href));
                    if let Some(uri) = data_uri {
                        html.push_str(&format!("<img src=\"{}\"/>", uri));
                    }
                }
            }
            _ => {}
        }
    }

    html.push_str("</div>");
    html
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
