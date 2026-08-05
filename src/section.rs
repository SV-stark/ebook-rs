use crate::archive::{resolve_relative_path, EpubArchive};
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Loaded and processed EPUB section ready for reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub index: usize,
    pub idref: String,
    pub href: String,
    pub full_path: String,
    pub raw_html: String,
    pub processed_html: String,
    pub plain_text: String,
    pub char_count: usize,
}

impl Section {
    /// Create and process a section from archive content.
    pub fn new(
        index: usize,
        idref: String,
        href: String,
        full_path: String,
        archive: &EpubArchive,
    ) -> Result<Self, String> {
        let raw_html = archive.read_string(&full_path)?;
        let plain_text = extract_plain_text(&raw_html);
        let char_count = plain_text.chars().count();
        let processed_html = process_section_resources(&raw_html, &full_path, archive);

        Ok(Self {
            index,
            idref,
            href,
            full_path,
            raw_html,
            processed_html,
            plain_text,
            char_count,
        })
    }
}

/// Extract clean plain text from HTML content by stripping tags, styles, and scripts.
pub fn extract_plain_text(html: &str) -> String {
    let mut in_tag = false;
    let mut text = String::with_capacity(html.len());
    let mut skipping_tag: Option<&'static str> = None;

    let lower = html.to_lowercase();
    let lower_bytes = lower.as_bytes();
    let html_bytes = html.as_bytes();
    let len = html_bytes.len();

    let mut i = 0;
    while i < len {
        if !in_tag && html_bytes[i] == b'<' {
            in_tag = true;
            let slice = &lower_bytes[i..];
            if skipping_tag.is_none() {
                if slice.starts_with(b"<style") {
                    skipping_tag = Some("style");
                } else if slice.starts_with(b"<script") {
                    skipping_tag = Some("script");
                }
            } else if let Some(tag) = skipping_tag {
                if (tag == "style" && slice.starts_with(b"</style"))
                    || (tag == "script" && slice.starts_with(b"</script"))
                {
                    skipping_tag = None;
                }
            }
            text.push(' ');
            i += 1;
            continue;
        }

        if in_tag {
            if html_bytes[i] == b'>' {
                in_tag = false;
            }
            i += 1;
            continue;
        }

        if skipping_tag.is_none() {
            text.push(html_bytes[i] as char);
        }
        i += 1;
    }

    // Collapse multiple whitespace spaces into single space
    let mut result = String::with_capacity(text.len());
    let mut prev_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }

    result.trim().to_string()
}

/// Process XHTML/HTML resources: Inline images, styles, and fonts as base64 Data URIs.
pub fn process_section_resources(html: &str, section_path: &str, archive: &EpubArchive) -> String {
    let section_dir = if let Some(idx) = section_path.rfind('/') {
        &section_path[..idx]
    } else {
        ""
    };

    let mut output = html.to_string();

    // Replace image src attributes
    let img_src_regex = regex_find_attr(html, "src");
    for (orig_attr, src_val) in img_src_regex {
        if src_val.starts_with("data:")
            || src_val.starts_with("http://")
            || src_val.starts_with("https://")
        {
            continue;
        }
        let res_path = resolve_relative_path(section_dir, &src_val);
        if let Ok(bytes) = archive.read_bytes(&res_path) {
            let mime = EpubArchive::get_mime_type(&res_path);
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            let data_uri = format!("data:{};base64,{}", mime, b64);
            output = output.replace(&orig_attr, &format!("src=\"{}\"", data_uri));
        }
    }

    // Replace css href attributes
    let css_href_regex = regex_find_attr(html, "href");
    for (orig_attr, href_val) in css_href_regex {
        if href_val.ends_with(".css") {
            let res_path = resolve_relative_path(section_dir, &href_val);
            if let Ok(css_text) = archive.read_string(&res_path) {
                let processed_css = process_css_resources(&css_text, &res_path, archive);
                let b64 =
                    base64::engine::general_purpose::STANDARD.encode(processed_css.as_bytes());
                let data_uri = format!("data:text/css;base64,{}", b64);
                output = output.replace(&orig_attr, &format!("href=\"{}\"", data_uri));
            }
        }
    }

    output
}

/// Simple helper to find attributes like `src="..."` or `href="..."`.
fn regex_find_attr(html: &str, attr: &str) -> Vec<(String, String)> {
    let mut list = Vec::new();
    let pattern = format!("{}=\"", attr);
    let mut search_idx = 0;

    while let Some(start) = html[search_idx..].find(&pattern) {
        let abs_start = search_idx + start;
        let val_start = abs_start + pattern.len();
        if let Some(end) = html[val_start..].find('"') {
            let abs_end = val_start + end;
            let val = &html[val_start..abs_end];
            let orig = &html[abs_start..=abs_end];
            list.push((orig.to_string(), val.to_string()));
            search_idx = abs_end + 1;
        } else {
            break;
        }
    }

    list
}

/// Inline fonts and images inside CSS stylesheet content.
fn process_css_resources(css: &str, css_path: &str, archive: &EpubArchive) -> String {
    let css_dir = if let Some(idx) = css_path.rfind('/') {
        &css_path[..idx]
    } else {
        ""
    };

    let mut output = css.to_string();
    let mut search_idx = 0;

    while let Some(url_idx) = output[search_idx..].find("url(") {
        let abs_url = search_idx + url_idx;
        let val_start = abs_url + 4;
        if let Some(close_idx) = output[val_start..].find(')') {
            let abs_close = val_start + close_idx;
            let raw_url = output[val_start..abs_close]
                .trim()
                .trim_matches('\'')
                .trim_matches('"');
            if !raw_url.starts_with("data:") && !raw_url.starts_with("http") {
                let res_path = resolve_relative_path(css_dir, raw_url);
                if let Ok(bytes) = archive.read_bytes(&res_path) {
                    let mime = EpubArchive::get_mime_type(&res_path);
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    let data_uri = format!("data:{};base64,{}", mime, b64);
                    let target_str = &output[abs_url..=abs_close];
                    let replacement = format!("url(\"{}\")", data_uri);
                    output = output.replace(target_str, &replacement);
                }
            }
            search_idx = abs_url + 10;
        } else {
            break;
        }
    }

    output
}
