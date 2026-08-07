use crate::section::extract_plain_text;
use serde::{Deserialize, Serialize};

/// Structured representation of an eBook footnote or endnote for popup previewing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Footnote {
    pub id: String,
    pub href: String,
    pub target_id: String,
    pub label: String,
    pub html_content: String,
    pub plain_text: String,
}

/// Extract footnotes and endnotes from section HTML content.
pub fn parse_footnotes_from_html(html: &str) -> Vec<Footnote> {
    let mut list = Vec::new();
    let lower = html.to_lowercase();
    let mut search_idx = 0;

    // 1. Scan for footnote reference links (<a href="#..." epub:type="noteref"> or class="footnote">)
    while let Some(a_idx) = lower[search_idx..].find("<a ") {
        let abs_a = search_idx + a_idx;
        if let Some(close_idx) = html[abs_a..].find('>') {
            let tag = &html[abs_a..=abs_a + close_idx];
            let tag_lower = tag.to_lowercase();

            let is_noteref = tag_lower.contains("noteref")
                || tag_lower.contains("footnote")
                || tag_lower.contains("endnote");

            if is_noteref {
                if let Some((_, href_val)) = extract_attr(tag, "href") {
                    if let Some(target_id) = href_val.split('#').nth(1) {
                        let label = if let Some(end_a) = lower[abs_a..].find("</a>") {
                            extract_plain_text(&html[abs_a + close_idx + 1..abs_a + end_a])
                        } else {
                            target_id.to_string()
                        };

                        if let Some((target_html, target_text)) =
                            extract_footnote_target(html, target_id)
                        {
                            let id_val = extract_attr(tag, "id")
                                .map(|(_, id)| id)
                                .unwrap_or_else(|| format!("fn_ref_{}", list.len() + 1));

                            list.push(Footnote {
                                id: id_val,
                                href: href_val.to_string(),
                                target_id: target_id.to_string(),
                                label: label.trim().to_string(),
                                html_content: target_html,
                                plain_text: target_text,
                            });
                        }
                    }
                }
            }
            search_idx = abs_a + close_idx + 1;
        } else {
            break;
        }
    }

    // 2. Fallback: Scan for standalone <aside epub:type="footnote" id="..."> elements
    search_idx = 0;
    while let Some(aside_idx) = lower[search_idx..].find("<aside") {
        let abs_aside = search_idx + aside_idx;
        if let Some(close_idx) = html[abs_aside..].find('>') {
            let tag = &html[abs_aside..=abs_aside + close_idx];
            if tag.to_lowercase().contains("footnote") {
                if let Some((_, id_val)) = extract_attr(tag, "id") {
                    if !list.iter().any(|fn_item| fn_item.target_id == id_val) {
                        if let Some((target_html, target_text)) =
                            extract_footnote_target(html, &id_val)
                        {
                            list.push(Footnote {
                                id: format!("fn_{}", id_val),
                                href: format!("#{}", id_val),
                                target_id: id_val.clone(),
                                label: id_val,
                                html_content: target_html,
                                plain_text: target_text,
                            });
                        }
                    }
                }
            }
            search_idx = abs_aside + close_idx + 1;
        } else {
            break;
        }
    }

    list
}

/// Helper to extract footnote target HTML and plain text by element ID.
fn find_ignore_case(s: &str, pat: &str) -> Option<usize> {
    if pat.is_empty() || s.len() < pat.len() {
        return None;
    }
    for (i, _) in s.char_indices() {
        let end = i + pat.len();
        if end <= s.len() && s.is_char_boundary(end) {
            if s[i..end].eq_ignore_ascii_case(pat) {
                return Some(i);
            }
        }
    }
    None
}

fn extract_footnote_target(html: &str, target_id: &str) -> Option<(String, String)> {
    let pattern1 = format!("id=\"{}\"", target_id);
    let pattern2 = format!("id='{}'", target_id);

    let target_idx =
        find_ignore_case(html, &pattern1).or_else(|| find_ignore_case(html, &pattern2))?;

    // Find opening tag start '<'
    let tag_start = html[..target_idx].rfind('<')?;

    // Determine target tag type (e.g. <aside>, <li>, <div>, <p>, <section>)
    let tag_name_end = html[tag_start + 1..]
        .find(|ch: char| ch.is_whitespace() || ch == '>')
        .unwrap_or(5);
    let tag_name = &html[tag_start + 1..tag_start + 1 + tag_name_end];
    let close_tag = format!("</{}>", tag_name);

    if let Some(close_idx) = find_ignore_case(&html[tag_start..], &close_tag) {
        let full_html = html[tag_start..tag_start + close_idx + close_tag.len()].to_string();
        let plain_text = extract_plain_text(&full_html);
        Some((full_html, plain_text))
    } else {
        // Fallback: extract single tag slice
        let tag_close = html[tag_start..].find('>')?;
        let full_html = html[tag_start..=tag_start + tag_close].to_string();
        let plain_text = extract_plain_text(&full_html);
        Some((full_html, plain_text))
    }
}

fn extract_attr(tag_str: &str, attr: &str) -> Option<(String, String)> {
    let pattern = format!("{}=\"", attr);
    if let Some(start) = find_ignore_case(tag_str, &pattern) {
        let val_start = start + pattern.len();
        if tag_str.is_char_boundary(val_start) {
            if let Some(end) = tag_str[val_start..].find('"') {
                let val = &tag_str[val_start..val_start + end];
                let orig = &tag_str[start..=val_start + end];
                return Some((orig.to_string(), val.to_string()));
            }
        }
    }
    None
}
