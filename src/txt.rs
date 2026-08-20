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

/// Plain Text (.txt) and Markdown (.md) document parser engine with YAML/TOML frontmatter,
/// Obsidian-style wikilinks (`[[wikilink]]`), and GFM/Obsidian callout block support (`> [!NOTE]`).
pub struct TxtBook;

impl TxtBook {
    /// Parse plain text (.txt) or markdown (.md) byte slice into a `Book` instance.
    pub fn parse(
        bytes: &[u8],
        title_fallback: &str,
        is_markdown: bool,
    ) -> Result<Book, EbookError> {
        let raw_text = String::from_utf8_lossy(bytes);
        if raw_text.trim().is_empty() {
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
        let mut publishers = Vec::new();
        let mut languages = vec!["en".to_string()];
        let mut description = Some("Markdown Document".to_string());
        let mut subjects = vec!["Text".to_string()];
        let mut identifier = None;

        let content_text = if is_markdown {
            let (extracted_meta, body) = extract_frontmatter(&raw_text);
            if let Some(t) = extracted_meta.title {
                title = t;
                has_custom_title = true;
            }
            if !extracted_meta.creators.is_empty() {
                creators = extracted_meta.creators;
            }
            if !extracted_meta.languages.is_empty() {
                languages = extracted_meta.languages;
            }
            if let Some(d) = extracted_meta.description {
                description = Some(d);
            }
            if !extracted_meta.publishers.is_empty() {
                publishers = extracted_meta.publishers;
            }
            if !extracted_meta.subjects.is_empty() {
                subjects = extracted_meta.subjects;
            }
            if let Some(id) = extracted_meta.identifier {
                identifier = Some(id);
            }
            body
        } else {
            // Legacy inline metadata sniffing for plain text files
            for line in raw_text.lines().take(15) {
                if let Some((key, val)) = line.split_once(':') {
                    let key_clean = key.trim().to_lowercase();
                    let v = val.trim().trim_matches('"').trim_matches('\'').trim();
                    if !v.is_empty() {
                        if key_clean == "title" {
                            title = v.to_string();
                            has_custom_title = true;
                        } else if key_clean == "author" || key_clean == "creator" {
                            creators.push(v.to_string());
                        } else if key_clean == "language" || key_clean == "lang" {
                            languages = vec![v.to_string()];
                        }
                    }
                }
            }
            raw_text.to_string()
        };

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

            let mut in_callout = false;
            let mut callout_type = String::new();
            let mut callout_title = String::new();
            let mut callout_lines = Vec::new();

            let flush_callout = |current_html: &mut String,
                                 plain_buf: &mut String,
                                 c_type: &str,
                                 c_title: &str,
                                 c_lines: &mut Vec<String>| {
                let icon = match c_type {
                    "tip" => "💡",
                    "important" => "❗",
                    "warning" => "⚠️",
                    "caution" => "🛑",
                    _ => "ℹ️",
                };
                let display_title = if c_title.is_empty() {
                    c_type.to_uppercase()
                } else {
                    c_title.to_string()
                };

                let mut inner_html = String::new();
                for cl in c_lines.iter() {
                    inner_html.push_str(&format!("<p>{}</p>\n", parse_inline_markdown(cl)));
                    plain_buf.push_str(cl);
                    plain_buf.push('\n');
                }

                let safe_c_type: String = c_type
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                let callout_block = format!(
                    "<div class=\"callout callout-{}\">\n<div class=\"callout-title\"><span class=\"callout-icon\">{}</span> {}</div>\n<div class=\"callout-content\">\n{}</div>\n</div>\n",
                    safe_c_type,
                    icon,
                    xml_escape(&display_title),
                    inner_html
                );
                current_html.push_str(&callout_block);
                c_lines.clear();
            };

            for line in content_text.lines() {
                let trimmed = line.trim();

                // Code block handling
                if trimmed.starts_with("```") {
                    if in_callout {
                        flush_callout(
                            &mut current_section_html,
                            &mut plain_text_buf,
                            &callout_type,
                            &callout_title,
                            &mut callout_lines,
                        );
                        in_callout = false;
                    }

                    if in_code_block {
                        let pre_html = format!(
                            "<pre><code class=\"language-{}\">\n{}</code></pre>\n",
                            if code_lang.is_empty() {
                                "text"
                            } else {
                                &code_lang
                            },
                            xml_escape(&code_buf)
                        );
                        current_section_html.push_str(&pre_html);
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

                // Callout start detection (`> [!NOTE] Title`)
                if trimmed.starts_with('>') {
                    let quote_content = trimmed.trim_start_matches('>').trim();
                    if quote_content.starts_with("[!") && quote_content.contains(']') {
                        if in_callout {
                            flush_callout(
                                &mut current_section_html,
                                &mut plain_text_buf,
                                &callout_type,
                                &callout_title,
                                &mut callout_lines,
                            );
                        }

                        let end_bracket = quote_content.find(']').unwrap();
                        let tag_type = quote_content[2..end_bracket].to_lowercase();
                        let custom_title = quote_content[end_bracket + 1..].trim();

                        in_callout = true;
                        callout_type = tag_type;
                        callout_title = custom_title.to_string();
                        continue;
                    } else if in_callout {
                        callout_lines.push(quote_content.to_string());
                        continue;
                    }
                } else if in_callout {
                    flush_callout(
                        &mut current_section_html,
                        &mut plain_text_buf,
                        &callout_type,
                        &callout_title,
                        &mut callout_lines,
                    );
                    in_callout = false;
                }

                // Headings
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

                    let parsed_heading = parse_inline_markdown(heading_text);
                    let h_tag = format!(
                        "<h{}>{}</h{}>\n",
                        level.min(6),
                        parsed_heading,
                        level.min(6)
                    );
                    current_section_html.push_str(&h_tag);
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
                    let inline_parsed = parse_inline_markdown(trimmed);
                    let p_html = format!("<p>{}</p>\n", inline_parsed);
                    current_section_html.push_str(&p_html);
                    plain_text_buf.push_str(trimmed);
                    plain_text_buf.push('\n');
                }
            }

            if in_callout {
                flush_callout(
                    &mut current_section_html,
                    &mut plain_text_buf,
                    &callout_type,
                    &callout_title,
                    &mut callout_lines,
                );
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
            // Plain text parsing
            let mut html_buf = String::new();
            let mut plain_text_buf = String::new();

            for paragraph in content_text.split("\n\n") {
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
            publishers,
            languages,
            rights: None,
            description,
            identifier,
            pub_date: None,
            modified_date: None,
            subjects,
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

/// Parsed metadata container from YAML / TOML frontmatter.
#[derive(Debug, Default)]
struct FrontmatterMeta {
    pub title: Option<String>,
    pub creators: Vec<String>,
    pub publishers: Vec<String>,
    pub languages: Vec<String>,
    pub description: Option<String>,
    pub subjects: Vec<String>,
    pub identifier: Option<String>,
}

/// Extract YAML (`--- ... ---`) or TOML (`+++ ... +++`) frontmatter from Markdown source.
fn extract_frontmatter(text: &str) -> (FrontmatterMeta, String) {
    let mut meta = FrontmatterMeta::default();
    let (delimiter, rest) = if let Some(after) = text.strip_prefix("---\n") {
        ("---", after)
    } else if let Some(after) = text.strip_prefix("---\r\n") {
        ("---", after)
    } else if let Some(after) = text.strip_prefix("+++\n") {
        ("+++", after)
    } else if let Some(after) = text.strip_prefix("+++\r\n") {
        ("+++", after)
    } else {
        return (meta, text.to_string());
    };

    let mut frontmatter_content = None;
    let mut body_content = None;

    let mut current_offset = 0;
    for line in rest.split_inclusive('\n') {
        let trimmed_line = line.trim_end_matches(&['\r', '\n'][..]).trim();
        if trimmed_line == delimiter {
            frontmatter_content = Some(&rest[..current_offset]);
            body_content = Some(&rest[current_offset + line.len()..]);
            break;
        }
        current_offset += line.len();
    }

    if let (Some(fm_content), Some(body)) = (frontmatter_content, body_content) {
        for line in fm_content.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() || line_trimmed.starts_with('#') {
                continue;
            }
            if let Some(colon_pos) = line_trimmed.find(':').or_else(|| line_trimmed.find('=')) {
                let key = line_trimmed[..colon_pos].trim().to_lowercase();
                let val = line_trimmed[colon_pos + 1..]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim();

                match key.as_str() {
                    "title" => meta.title = Some(val.to_string()),
                    "author" | "creator" => meta.creators.push(val.to_string()),
                    "authors" | "creators" => {
                        let cleaned = val.trim_matches('[').trim_matches(']');
                        for a in cleaned.split(',') {
                            let clean_a = a.trim().trim_matches('"').trim_matches('\'').trim();
                            if !clean_a.is_empty() {
                                meta.creators.push(clean_a.to_string());
                            }
                        }
                    }
                    "language" | "lang" => meta.languages = vec![val.to_string()],
                    "description" | "summary" => meta.description = Some(val.to_string()),
                    "publisher" => meta.publishers.push(val.to_string()),
                    "isbn" | "identifier" | "id" => meta.identifier = Some(val.to_string()),
                    "tags" | "subjects" => {
                        let cleaned = val.trim_matches('[').trim_matches(']');
                        for tag in cleaned.split(',') {
                            let clean_tag = tag.trim().trim_matches('"').trim_matches('\'').trim();
                            if !clean_tag.is_empty() {
                                meta.subjects.push(clean_tag.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        (meta, body.trim_start().to_string())
    } else {
        (meta, text.to_string())
    }
}

/// Convert inline Markdown formatting, Obsidian wikilinks `[[Link|Label]]`, bold, italic, and code.
fn parse_inline_markdown(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();

    while i < chars.len() {
        // 1. Obsidian-style wikilinks `[[target|label]]` or `[[target]]`
        if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
            if let Some(end_pos) = chars[i + 2..].windows(2).position(|w| w == [']', ']']) {
                let abs_end = i + 2 + end_pos;
                let link_content: String = chars[i + 2..abs_end].iter().collect();
                let parts: Vec<&str> = link_content.split('|').collect();

                let (target, label) = match parts.len() {
                    1 => {
                        let t = parts[0].trim();
                        let l = if t.starts_with('#') {
                            t.trim_start_matches('#')
                        } else {
                            t
                        };
                        (t, l)
                    }
                    _ => (parts[0].trim(), parts[1].trim()),
                };

                let href = if target.starts_with('#') {
                    target.to_string()
                } else {
                    format!("#{}", target.to_lowercase().replace(' ', "-"))
                };

                out.push_str(&format!(
                    "<a href=\"{}\" class=\"wikilink\">{}</a>",
                    xml_escape(&href),
                    xml_escape(label)
                ));
                i = abs_end + 2;
                continue;
            }
        }

        // 2. Standard Markdown links `[label](url)`
        if chars[i] == '[' {
            if let Some(close_bracket) = chars[i + 1..].iter().position(|c| *c == ']') {
                let abs_close = i + 1 + close_bracket;
                if abs_close + 1 < chars.len() && chars[abs_close + 1] == '(' {
                    if let Some(close_paren) = chars[abs_close + 2..].iter().position(|c| *c == ')')
                    {
                        let abs_paren = abs_close + 2 + close_paren;
                        let label: String = chars[i + 1..abs_close].iter().collect();
                        let url: String = chars[abs_close + 2..abs_paren].iter().collect();

                        out.push_str(&format!(
                            "<a href=\"{}\">{}</a>",
                            xml_escape(&url),
                            xml_escape(&label)
                        ));
                        i = abs_paren + 1;
                        continue;
                    }
                }
            }
        }

        // 3. Inline code `` `code` ``
        if chars[i] == '`' {
            if let Some(close_tick) = chars[i + 1..].iter().position(|c| *c == '`') {
                let abs_tick = i + 1 + close_tick;
                let code_content: String = chars[i + 1..abs_tick].iter().collect();
                out.push_str(&format!("<code>{}</code>", xml_escape(&code_content)));
                i = abs_tick + 1;
                continue;
            }
        }

        // 4. Bold `**text**`
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(close_bold) = chars[i + 2..].windows(2).position(|w| w == ['*', '*']) {
                let abs_bold = i + 2 + close_bold;
                let bold_content: String = chars[i + 2..abs_bold].iter().collect();
                out.push_str(&format!("<strong>{}</strong>", xml_escape(&bold_content)));
                i = abs_bold + 2;
                continue;
            }
        }

        // 5. Italic `*text*`
        if chars[i] == '*' {
            if let Some(close_italic) = chars[i + 1..].iter().position(|c| *c == '*') {
                let abs_italic = i + 1 + close_italic;
                let italic_content: String = chars[i + 1..abs_italic].iter().collect();
                out.push_str(&format!("<em>{}</em>", xml_escape(&italic_content)));
                i = abs_italic + 1;
                continue;
            }
        }

        // Escape HTML special characters
        match chars[i] {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
        i += 1;
    }

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
