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

/// Plain Text (.txt) and Markdown (.md) document parser engine.
pub struct TxtBook;

impl TxtBook {
    /// Parse plain text (.txt) or markdown (.md) byte slice into a `Book` instance.
    pub fn parse(
        bytes: &[u8],
        title_fallback: &str,
        is_markdown: bool,
    ) -> Result<Book, EbookError> {
        let text = String::from_utf8_lossy(bytes);
        if text.trim().is_empty() {
            return Err(EbookError::InvalidFormat("Text file is empty".to_string()));
        }

        let mut title = title_fallback.to_string();
        let mut has_custom_title = title_fallback != "eBook"
            && title_fallback != "Untitled"
            && title_fallback != "TXT Book"
            && title_fallback != "MD Book"
            && !title_fallback.ends_with("Book")
            && !title_fallback.to_lowercase().contains("ignored");
        let mut creators = Vec::new();
        let mut languages = vec!["en".to_string()];

        for line in text.lines().take(15) {
            let lower = line.to_lowercase();
            if let Some(pos) = lower.find("title:") {
                let v = line[pos + 6..]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim();
                if !v.is_empty() {
                    title = v.to_string();
                    has_custom_title = true;
                }
            } else if let Some(pos) = lower.find("author:").or_else(|| lower.find("creator:")) {
                if let Some(colon) = line[pos..].find(':') {
                    let v = line[pos + colon + 1..]
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .trim();
                    if !v.is_empty() {
                        creators.push(v.to_string());
                    }
                }
            } else if let Some(pos) = lower.find("language:").or_else(|| lower.find("lang:")) {
                if let Some(colon) = line[pos..].find(':') {
                    let v = line[pos + colon + 1..]
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .trim();
                    if !v.is_empty() {
                        languages = vec![v.to_string()];
                    }
                }
            }
        }

        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();

        if is_markdown {
            let mut current_section_html = String::new();
            let mut section_index = 0;
            let mut plain_text_buf = String::new();

            let mut in_code_block = false;
            let mut code_lang = String::new();
            let mut code_buf = String::new();

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("```") {
                    if in_code_block {
                        let pre_html = format!(
                            "<pre><code class=\"language-{}\">\n{}</code></pre>",
                            if code_lang.is_empty() {
                                "text"
                            } else {
                                &code_lang
                            },
                            xml_escape(&code_buf)
                        );
                        current_section_html.push_str(&pre_html);
                        current_section_html.push('\n');
                        plain_text_buf.push_str(&code_buf);
                        plain_text_buf.push('\n');

                        in_code_block = false;
                        code_lang.clear();
                        code_buf.clear();
                    } else {
                        in_code_block = true;
                        code_lang = trimmed.trim_start_matches('`').trim().to_string();
                    }
                    continue;
                }

                if in_code_block {
                    code_buf.push_str(line);
                    code_buf.push('\n');
                    continue;
                }

                if trimmed.starts_with('#') {
                    let level = trimmed.chars().take_while(|c| *c == '#').count();
                    let heading_text = trimmed.trim_start_matches('#').trim();

                    if !has_custom_title && level == 1 {
                        title = heading_text.to_string();
                    }

                    if !plain_text_buf.is_empty() {
                        let sec_href = format!("section_{}.html", section_index);
                        let full_html = format!(
                            "<div class=\"md-content\">\n{}\n</div>",
                            current_section_html
                        );

                        let char_count = plain_text_buf.chars().count();
                        let plain_text_lower = plain_text_buf.to_lowercase();

                        sections.push(Section {
                            index: section_index,
                            idref: format!("sec_{}", section_index),
                            href: sec_href.clone(),
                            full_path: sec_href.clone(),
                            raw_html: full_html.clone(),
                            processed_html: full_html,
                            plain_text: plain_text_buf.clone(),
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
                        current_section_html.clear();
                        plain_text_buf.clear();
                    }

                    let h_tag = format!(
                        "<h{}>{}</h{}>",
                        level.min(6),
                        xml_escape(heading_text),
                        level.min(6)
                    );
                    current_section_html.push_str(&h_tag);
                    current_section_html.push('\n');
                    plain_text_buf.push_str(heading_text);
                    plain_text_buf.push('\n');

                    let href = format!("section_{}.html", section_index);
                    toc.push(NavPoint {
                        id: format!("nav_{}", toc.len() + 1),
                        label: heading_text.to_string(),
                        href: href.clone(),
                        full_path: href,
                        subitems: Vec::new(),
                    });
                } else if !trimmed.is_empty() {
                    let p_html = format!("<p>{}</p>", xml_escape(trimmed));
                    current_section_html.push_str(&p_html);
                    current_section_html.push('\n');
                    plain_text_buf.push_str(trimmed);
                    plain_text_buf.push('\n');
                }
            }

            if !plain_text_buf.is_empty() || sections.is_empty() {
                let sec_href = format!("section_{}.html", section_index);
                let full_html = format!(
                    "<div class=\"md-content\">\n{}\n</div>",
                    current_section_html
                );

                let char_count = plain_text_buf.chars().count();
                let plain_text_lower = plain_text_buf.to_lowercase();

                sections.push(Section {
                    index: section_index,
                    idref: format!("sec_{}", section_index),
                    href: sec_href.clone(),
                    full_path: sec_href.clone(),
                    raw_html: full_html.clone(),
                    processed_html: full_html,
                    plain_text: plain_text_buf.clone(),
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
        } else {
            let mut html_buf = String::new();
            let mut plain_text_buf = String::new();

            for paragraph in text.split("\n\n") {
                let p_clean = paragraph.trim();
                if !p_clean.is_empty() {
                    html_buf.push_str(&format!("<p>{}</p>\n", xml_escape(p_clean)));
                    plain_text_buf.push_str(p_clean);
                    plain_text_buf.push('\n');
                }
            }

            let sec_href = "section_0.html".to_string();
            let full_html = format!("<div class=\"txt-content\">\n{}\n</div>", html_buf);
            let char_count = plain_text_buf.chars().count();
            let plain_text_lower = plain_text_buf.to_lowercase();

            sections.push(Section {
                index: 0,
                idref: "sec_0".to_string(),
                href: sec_href.clone(),
                full_path: sec_href.clone(),
                raw_html: full_html.clone(),
                processed_html: full_html,
                plain_text: plain_text_buf,
                plain_text_lower,
                char_count,
                viewport_width: None,
                viewport_height: None,
            });

            spine.push(SpineItem {
                idref: "sec_0".to_string(),
                linear: true,
                properties: Vec::new(),
                index: 0,
                href: sec_href.clone(),
                media_type: "application/xhtml+xml".to_string(),
            });

            toc.push(NavPoint {
                id: "nav_1".to_string(),
                label: title.clone(),
                href: sec_href.clone(),
                full_path: sec_href,
                subitems: Vec::new(),
            });
        }

        let metadata = Metadata {
            title,
            creators,
            publishers: Vec::new(),
            languages,
            rights: None,
            description: Some("Text Document".to_string()),
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects: vec!["Text".to_string()],
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

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
