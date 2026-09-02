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

/// Rich Text Format (.rtf) document parser engine.
pub struct RtfBook;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RtfState {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    is_destination: bool,
    dest_name: String,
    uc: usize,
}

impl Default for RtfState {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strike: false,
            is_destination: false,
            dest_name: String::new(),
            uc: 1,
        }
    }
}

impl RtfBook {
    /// Parse Rich Text Format (.rtf) byte slice into a unified `Book` instance.
    pub fn parse(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
        let text = match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(bytes).to_string(),
        };

        if !text.starts_with("{\\rtf") && !text.contains("{\\rtf") {
            return Err(EbookError::InvalidFormat(
                "Not a valid RTF document (missing {\\rtf header)".to_string(),
            ));
        }

        let mut archive = EpubArchive::empty();
        let mut title = title_fallback.to_string();
        let mut creators = Vec::new();
        let mut description = None;

        let mut sections: Vec<Section> = Vec::new();
        let mut spine: Vec<SpineItem> = Vec::new();
        let mut toc: Vec<NavPoint> = Vec::new();

        let mut current_html = String::new();
        let mut current_text = String::new();
        let mut cur_run = String::new();
        let mut section_index = 0;
        let mut img_counter = 0;

        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut i = 0;

        let mut state_stack: Vec<RtfState> = vec![RtfState::default()];
        let mut cur_state = RtfState::default();

        let mut dest_buffer = String::new();
        let mut in_pict = false;
        let mut pict_hex = String::new();
        let mut pict_type = "png";

        let mut in_table = false;
        let mut cell_start_idx = 0;
        let mut table_cells: Vec<String> = Vec::new();

        let flush_run =
            |html: &mut String, text: &mut String, run: &mut String, state: &RtfState| {
                if run.is_empty() {
                    return;
                }
                let mut formatted = xml_escape(run);
                if state.strike {
                    formatted = format!("<s>{}</s>", formatted);
                }
                if state.underline {
                    formatted = format!("<u>{}</u>", formatted);
                }
                if state.italic {
                    formatted = format!("<em>{}</em>", formatted);
                }
                if state.bold {
                    formatted = format!("<strong>{}</strong>", formatted);
                }
                html.push_str(&formatted);
                text.push_str(run);
                run.clear();
            };

        let flush_section = |sec_idx: usize,
                             cur_html: &str,
                             cur_txt: &str,
                             sections: &mut Vec<Section>,
                             spine: &mut Vec<SpineItem>,
                             toc: &mut Vec<NavPoint>| {
            if cur_txt.trim().is_empty() && !sections.is_empty() {
                return;
            }
            let href = format!("section_{}.html", sec_idx);
            let full_html = format!("<div class=\"rtf-section\">\n{}\n</div>", cur_html);
            let char_count = cur_txt.chars().count();
            let plain_text_lower = cur_txt.to_lowercase();

            sections.push(Section {
                index: sec_idx,
                idref: format!("sec_{}", sec_idx),
                href: href.clone(),
                full_path: href.clone(),
                raw_html: full_html.clone(),
                processed_html: full_html,
                plain_text: cur_txt.to_string(),
                plain_text_lower,
                char_count,
                viewport_width: None,
                viewport_height: None,
            });

            spine.push(SpineItem {
                idref: format!("sec_{}", sec_idx),
                linear: true,
                properties: Vec::new(),
                index: sec_idx,
                href: href.clone(),
                media_type: "application/xhtml+xml".to_string(),
            });

            toc.push(NavPoint {
                id: format!("nav_{}", toc.len() + 1),
                label: format!("Section {}", sec_idx + 1),
                href: href.clone(),
                full_path: href,
                subitems: Vec::new(),
            });
        };

        while i < len {
            match chars[i] {
                '{' => {
                    flush_run(
                        &mut current_html,
                        &mut current_text,
                        &mut cur_run,
                        &cur_state,
                    );
                    state_stack.push(cur_state.clone());
                    cur_state.is_destination = false;
                    cur_state.dest_name.clear();
                    i += 1;
                }
                '}' => {
                    flush_run(
                        &mut current_html,
                        &mut current_text,
                        &mut cur_run,
                        &cur_state,
                    );
                    if in_pict {
                        if !pict_hex.is_empty() {
                            if let Ok(bin) = hex_to_bytes(&pict_hex) {
                                img_counter += 1;
                                let img_name = format!("images/img_{}.{}", img_counter, pict_type);
                                archive.insert(&img_name, bin);
                                current_html.push_str(&format!(
                                    "<p><img src=\"{}\" alt=\"image\" /></p>\n",
                                    img_name
                                ));
                            }
                        }
                        in_pict = false;
                        pict_hex.clear();
                    }

                    if cur_state.is_destination {
                        let dest = cur_state.dest_name.as_str();
                        let buf_clean = dest_buffer.trim().to_string();
                        if !buf_clean.is_empty() {
                            match dest {
                                "title" => title = buf_clean,
                                "author" => creators.push(buf_clean),
                                "doccomm" | "subject" => description = Some(buf_clean),
                                _ => {}
                            }
                        }
                        dest_buffer.clear();
                    }

                    if let Some(prev) = state_stack.pop() {
                        cur_state = prev;
                    }
                    i += 1;
                }
                '\\' => {
                    i += 1;
                    if i >= len {
                        break;
                    }

                    // Special escaped characters
                    match chars[i] {
                        '{' | '}' | '\\' => {
                            let c = chars[i];
                            if cur_state.is_destination {
                                dest_buffer.push(c);
                            } else if in_pict {
                                pict_hex.push(c);
                            } else {
                                cur_run.push(c);
                            }
                            i += 1;
                            continue;
                        }
                        '\'' => {
                            // Hex escape \'XX
                            if i + 2 < len {
                                let hex_str: String = chars[i + 1..=i + 2].iter().collect();
                                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                                    let s = if byte < 0x80 {
                                        (byte as char).to_string()
                                    } else {
                                        let byte_arr = [byte];
                                        let (cow, _) = encoding_rs::WINDOWS_1252
                                            .decode_without_bom_handling(&byte_arr);
                                        cow.to_string()
                                    };
                                    if cur_state.is_destination {
                                        dest_buffer.push_str(&s);
                                    } else {
                                        cur_run.push_str(&s);
                                    }
                                }
                                i += 3;
                                continue;
                            }
                        }
                        '*' => {
                            // Ignorable destination marker
                            cur_state.is_destination = true;
                            i += 1;
                            continue;
                        }
                        _ => {}
                    }

                    // Read control word [a-zA-Z]+
                    let mut word = String::new();
                    while i < len && chars[i].is_ascii_alphabetic() {
                        word.push(chars[i]);
                        i += 1;
                    }

                    // Read optional numeric parameter
                    let mut is_neg = false;
                    if i < len && chars[i] == '-' {
                        is_neg = true;
                        i += 1;
                    }
                    let mut param_str = String::new();
                    while i < len && chars[i].is_ascii_digit() {
                        param_str.push(chars[i]);
                        i += 1;
                    }
                    let param: Option<i32> = if !param_str.is_empty() {
                        let p = param_str.parse::<i32>().unwrap_or(0);
                        Some(if is_neg { -p } else { p })
                    } else {
                        None
                    };

                    // Optional trailing space delimiter after control word
                    if i < len && chars[i] == ' ' {
                        i += 1;
                    }

                    match word.as_str() {
                        "b" => {
                            let next_bold = param != Some(0);
                            if next_bold != cur_state.bold {
                                flush_run(
                                    &mut current_html,
                                    &mut current_text,
                                    &mut cur_run,
                                    &cur_state,
                                );
                                cur_state.bold = next_bold;
                            }
                        }
                        "i" => {
                            let next_italic = param != Some(0);
                            if next_italic != cur_state.italic {
                                flush_run(
                                    &mut current_html,
                                    &mut current_text,
                                    &mut cur_run,
                                    &cur_state,
                                );
                                cur_state.italic = next_italic;
                            }
                        }
                        "ul" => {
                            let next_ul = param != Some(0);
                            if next_ul != cur_state.underline {
                                flush_run(
                                    &mut current_html,
                                    &mut current_text,
                                    &mut cur_run,
                                    &cur_state,
                                );
                                cur_state.underline = next_ul;
                            }
                        }
                        "ulnone" => {
                            if cur_state.underline {
                                flush_run(
                                    &mut current_html,
                                    &mut current_text,
                                    &mut cur_run,
                                    &cur_state,
                                );
                                cur_state.underline = false;
                            }
                        }
                        "strike" => {
                            let next_strike = param != Some(0);
                            if next_strike != cur_state.strike {
                                flush_run(
                                    &mut current_html,
                                    &mut current_text,
                                    &mut cur_run,
                                    &cur_state,
                                );
                                cur_state.strike = next_strike;
                            }
                        }
                        "par" => {
                            flush_run(
                                &mut current_html,
                                &mut current_text,
                                &mut cur_run,
                                &cur_state,
                            );
                            if !cur_state.is_destination && !in_pict {
                                current_html.push_str("<br/>\n");
                                current_text.push('\n');
                            }
                        }
                        "line" => {
                            flush_run(
                                &mut current_html,
                                &mut current_text,
                                &mut cur_run,
                                &cur_state,
                            );
                            if !cur_state.is_destination && !in_pict {
                                current_html.push_str("<br/>");
                                current_text.push('\n');
                            }
                        }
                        "page" => {
                            flush_run(
                                &mut current_html,
                                &mut current_text,
                                &mut cur_run,
                                &cur_state,
                            );
                            if !cur_state.is_destination && !in_pict && !current_text.is_empty() {
                                flush_section(
                                    section_index,
                                    &current_html,
                                    &current_text,
                                    &mut sections,
                                    &mut spine,
                                    &mut toc,
                                );
                                section_index += 1;
                                current_html.clear();
                                current_text.clear();
                            }
                        }
                        "tab" => {
                            flush_run(
                                &mut current_html,
                                &mut current_text,
                                &mut cur_run,
                                &cur_state,
                            );
                            if !cur_state.is_destination && !in_pict {
                                current_html.push_str("&emsp;");
                                current_text.push('\t');
                            }
                        }
                        "uc" => {
                            if let Some(n) = param {
                                cur_state.uc = (n.max(0)) as usize;
                            }
                        }
                        "u" => {
                            if let Some(code) = param {
                                let unsigned_code = if code < 0 {
                                    (code + 65536) as u32
                                } else {
                                    code as u32
                                };
                                if let Some(ch) = char::from_u32(unsigned_code) {
                                    if cur_state.is_destination {
                                        dest_buffer.push(ch);
                                    } else {
                                        cur_run.push(ch);
                                    }
                                }
                                // Skip optional fallback chars according to \ucN (including \'hh hex escapes)
                                let mut skipped = 0;
                                while i < len && skipped < cur_state.uc {
                                    if chars[i] == '{' || chars[i] == '}' {
                                        break;
                                    }
                                    if chars[i] == '\\' {
                                        if i + 3 < len && chars[i + 1] == '\'' {
                                            i += 4;
                                            skipped += 1;
                                            continue;
                                        } else {
                                            break;
                                        }
                                    }
                                    i += 1;
                                    skipped += 1;
                                }
                            }
                        }
                        "info" | "title" | "author" | "subject" | "doccomm" | "keywords"
                        | "fonttbl" | "colortbl" | "stylesheet" => {
                            flush_run(
                                &mut current_html,
                                &mut current_text,
                                &mut cur_run,
                                &cur_state,
                            );
                            cur_state.is_destination = true;
                            cur_state.dest_name = word;
                        }
                        "pict" => {
                            flush_run(
                                &mut current_html,
                                &mut current_text,
                                &mut cur_run,
                                &cur_state,
                            );
                            in_pict = true;
                            pict_hex.clear();
                        }
                        "pngblip" => pict_type = "png",
                        "jpegblip" => pict_type = "jpg",
                        "trowd" => {
                            flush_run(
                                &mut current_html,
                                &mut current_text,
                                &mut cur_run,
                                &cur_state,
                            );
                            in_table = true;
                            table_cells.clear();
                            cell_start_idx = current_html.len();
                        }
                        "cell" => {
                            flush_run(
                                &mut current_html,
                                &mut current_text,
                                &mut cur_run,
                                &cur_state,
                            );
                            if in_table && current_html.len() >= cell_start_idx {
                                let cell_content = current_html.split_off(cell_start_idx);
                                table_cells.push(cell_content);
                                cell_start_idx = current_html.len();
                            }
                        }
                        "row" => {
                            flush_run(
                                &mut current_html,
                                &mut current_text,
                                &mut cur_run,
                                &cur_state,
                            );
                            if in_table {
                                if current_html.len() > cell_start_idx {
                                    let cell_content = current_html.split_off(cell_start_idx);
                                    table_cells.push(cell_content);
                                }
                                current_html.push_str("<table>\n<tr>\n");
                                for cell in &table_cells {
                                    current_html.push_str(&format!("  <td>{}</td>\n", cell));
                                }
                                current_html.push_str("</tr>\n</table>\n");
                                table_cells.clear();
                                in_table = false;
                            }
                        }
                        _ => {}
                    }
                }
                c => {
                    if in_pict {
                        if c.is_ascii_hexdigit() {
                            pict_hex.push(c);
                        }
                    } else if cur_state.is_destination {
                        dest_buffer.push(c);
                    } else if c != '\r' && c != '\n' {
                        cur_run.push(c);
                    }
                    i += 1;
                }
            }
        }

        // Flush remaining run and trailing section
        flush_run(
            &mut current_html,
            &mut current_text,
            &mut cur_run,
            &cur_state,
        );
        flush_section(
            section_index,
            &current_html,
            &current_text,
            &mut sections,
            &mut spine,
            &mut toc,
        );

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

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() % 2 != 0 {
        return Err("Invalid hex length".to_string());
    }
    let mut bytes = Vec::with_capacity(clean.len() / 2);
    let chars: Vec<char> = clean.chars().collect();
    for chunk in chars.chunks(2) {
        let pair: String = chunk.iter().collect();
        let b = u8::from_str_radix(&pair, 16).map_err(|e| e.to_string())?;
        bytes.push(b);
    }
    Ok(bytes)
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
