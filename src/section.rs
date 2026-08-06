use crate::archive::{EpubArchive, resolve_relative_path};
use crate::layout::AssetDeliveryStrategy;
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
    pub plain_text_lower: String, // P4: Pre-computed lowercased text for zero-alloc search
    pub char_count: usize,
    pub viewport_width: Option<f64>,  // FXL viewport target width
    pub viewport_height: Option<f64>, // FXL viewport target height
}

impl Section {
    /// Create and process a section from archive content using default Base64 inlining.
    pub fn new(
        index: usize,
        idref: String,
        href: String,
        full_path: String,
        archive: &EpubArchive,
    ) -> Result<Self, String> {
        Self::new_with_strategy(
            index,
            idref,
            href,
            full_path,
            archive,
            &AssetDeliveryStrategy::InlinedBase64,
        )
    }

    /// Create and process a section using a specific asset delivery strategy (Base64 vs Resource Stream).
    pub fn new_with_strategy(
        index: usize,
        idref: String,
        href: String,
        full_path: String,
        archive: &EpubArchive,
        strategy: &AssetDeliveryStrategy,
    ) -> Result<Self, String> {
        let raw_html = archive.read_string(&full_path)?;
        let plain_text = extract_plain_text(&raw_html);
        let plain_text_lower = plain_text.to_lowercase();
        let char_count = plain_text.chars().count();
        let processed_html =
            process_section_resources_with_strategy(&raw_html, &full_path, archive, strategy);
        let (viewport_width, viewport_height) = parse_viewport_meta(&raw_html);

        Ok(Self {
            index,
            idref,
            href,
            full_path,
            raw_html,
            processed_html,
            plain_text,
            plain_text_lower,
            char_count,
            viewport_width,
            viewport_height,
        })
    }

    /// Strip embedded <script> tags, inline event attributes (on*="..."), and javascript: URIs.
    pub fn strip_script_content(&mut self) {
        self.processed_html = sanitize_html_scripts(&self.processed_html);
    }

    /// Extract footnotes and endnotes for popup previewing.
    pub fn extract_footnotes(&self) -> Vec<crate::footnote::Footnote> {
        crate::footnote::parse_footnotes_from_html(&self.raw_html)
    }

    /// Calculate structural NLP Reading Analytics (word count, WPM reading time, difficulty score, keywords).
    pub fn analytics(&self) -> crate::analytics::ReadingAnalytics {
        crate::analytics::ReadingAnalytics::analyze_text(&self.plain_text)
    }

    /// Calculate virtual reflow page breaks for this section using default or custom paginator bounds.
    pub fn paginate(
        &self,
        paginator: Option<&crate::paginator::ReflowPaginator>,
    ) -> crate::paginator::SectionPageMap {
        let default_paginator = crate::paginator::ReflowPaginator::default();
        let active_paginator = paginator.unwrap_or(&default_paginator);
        active_paginator.paginate_section(self)
    }
}

/// Extract clean plain text from HTML content by stripping tags, styles, and scripts.
/// Quote-aware HTML tag boundary finder that ignores `>` characters inside attribute quotes.
pub fn find_tag_end(html: &str, start_idx: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut in_quote: Option<u8> = None;
    let mut i = start_idx;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_quote {
            if b == q {
                in_quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            in_quote = Some(b);
        } else if b == b'>' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Extract clean plain text from HTML content by stripping tags, styles, and scripts.
/// B2 Fix: Multibyte UTF-8 and quote-aware extraction.
pub fn extract_plain_text(html: &str) -> String {
    let mut in_tag = false;
    let mut in_quote: Option<u8> = None;
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
            in_quote = None;
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
                    || slice.starts_with(b"<p")
                    || slice.starts_with(b"<div")
                    || slice.starts_with(b"<body")
                    || slice.starts_with(b"<h")
                    || slice.starts_with(b"<section")
                {
                    skipping_tag = None;
                }
            }
            text.push(' ');
            i += 1;
            continue;
        }

        if in_tag {
            let b = html_bytes[i];
            if let Some(q) = in_quote {
                if b == q {
                    in_quote = None;
                }
            } else if b == b'"' || b == b'\'' {
                in_quote = Some(b);
            } else if b == b'>' {
                in_tag = false;
                in_quote = None;
            }
            i += 1;
            continue;
        }

        if skipping_tag.is_none() {
            if let Some(ch) = html[i..].chars().next() {
                text.push(ch);
                i += ch.len_utf8();
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
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
    process_section_resources_with_strategy(
        html,
        section_path,
        archive,
        &AssetDeliveryStrategy::InlinedBase64,
    )
}

/// Process XHTML/HTML resources according to specified delivery strategy (Base64 vs ResourceStream).
pub fn process_section_resources_with_strategy(
    html: &str,
    section_path: &str,
    archive: &EpubArchive,
    strategy: &AssetDeliveryStrategy,
) -> String {
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

        match strategy {
            AssetDeliveryStrategy::InlinedBase64 => {
                if let Ok(bytes) = archive.read_bytes(&res_path) {
                    let mime = EpubArchive::get_mime_type(&res_path);
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    let data_uri = format!("data:{};base64,{}", mime, b64);
                    output = output.replace(&orig_attr, &format!("src=\"{}\"", data_uri));
                }
            }
            AssetDeliveryStrategy::ResourceStream => {
                let stream_url = format!("resource/{}", res_path);
                output = output.replace(&orig_attr, &format!("src=\"{}\"", stream_url));
            }
        }
    }

    // Replace ONLY <link href="*.css"> stylesheet links
    let css_href_regex = regex_find_link_css(html);
    for (orig_attr, href_val) in css_href_regex {
        let res_path = resolve_relative_path(section_dir, &href_val);
        match strategy {
            AssetDeliveryStrategy::InlinedBase64 => {
                if let Ok(css_text) = archive.read_string(&res_path) {
                    let processed_css = process_css_resources(&css_text, &res_path, archive);
                    let b64 =
                        base64::engine::general_purpose::STANDARD.encode(processed_css.as_bytes());
                    let data_uri = format!("data:text/css;base64,{}", b64);
                    output = output.replace(&orig_attr, &format!("href=\"{}\"", data_uri));
                }
            }
            AssetDeliveryStrategy::ResourceStream => {
                let stream_url = format!("resource/{}", res_path);
                output = output.replace(&orig_attr, &format!("href=\"{}\"", stream_url));
            }
        }
    }

    output
}

/// Helper to find ONLY `<link ... href="...">` tags for CSS inlining (E5 Fix).
fn regex_find_link_css(html: &str) -> Vec<(String, String)> {
    let mut list = Vec::new();
    let lower = html.to_lowercase();
    let mut search_idx = 0;

    while let Some(link_idx) = lower[search_idx..].find("<link") {
        let abs_link = search_idx + link_idx;
        if let Some(abs_close) = find_tag_end(html, abs_link) {
            let tag_str = &html[abs_link..=abs_close];
            if let Some((orig_href, val)) = extract_attr(tag_str, "href") {
                let val_lower = val.to_lowercase();
                if val_lower.contains(".css")
                    || tag_str.to_lowercase().contains("rel=\"stylesheet\"")
                {
                    list.push((orig_href, val));
                }
            }
            search_idx = abs_close + 1;
        } else {
            break;
        }
    }

    list
}

fn extract_attr(tag_str: &str, attr: &str) -> Option<(String, String)> {
    let lower_tag = tag_str.to_lowercase();
    let attr_lower = attr.to_lowercase();
    let pat1 = format!(" {}=\"", attr_lower);
    let pat2 = format!("<{}=\"", attr_lower);
    let pat3 = format!(" {}='", attr_lower);
    let pat4 = format!("<{}='", attr_lower);

    for pat in &[pat1, pat2, pat3, pat4] {
        if let Some(pos) = lower_tag.find(pat) {
            let quote = pat.chars().last().unwrap();
            let attr_start = pos + 1;
            let val_start = pos + pat.len();

            // B2 Fix: Ensure char boundaries before slicing non-ASCII / CJK attribute strings
            if tag_str.is_char_boundary(val_start) {
                if let Some(quote_idx) =
                    memchr::memchr(quote as u8, &tag_str.as_bytes()[val_start..])
                {
                    let val_end = val_start + quote_idx;
                    if tag_str.is_char_boundary(attr_start)
                        && tag_str.is_char_boundary(val_end)
                        && val_end < tag_str.len()
                    {
                        let val = &tag_str[val_start..val_end];
                        let orig = &tag_str[attr_start..=val_end];
                        return Some((orig.to_string(), val.to_string()));
                    }
                }
            }
        }
    }
    None
}

/// Helper to find attributes like `src="..."` or `href="..."` with word boundary checks (E4 Fix).
fn regex_find_attr(html: &str, attr: &str) -> Vec<(String, String)> {
    let mut list = Vec::new();
    let lower = html.to_lowercase();
    let pattern1 = format!(" {}=\"", attr);
    let pattern2 = format!("<{}=\"", attr);
    let mut search_idx = 0;

    while search_idx < html.len() {
        let p1_match = lower[search_idx..]
            .find(&pattern1)
            .map(|s| search_idx + s + 1);
        let p2_match = lower[search_idx..]
            .find(&pattern2)
            .map(|s| search_idx + s + 1);

        let abs_start = match (p1_match, p2_match) {
            (Some(m1), Some(m2)) => m1.min(m2),
            (Some(m1), None) => m1,
            (None, Some(m2)) => m2,
            (None, None) => break,
        };

        let val_start = abs_start + attr.len() + 2;
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

    let mut replacements = Vec::new();
    let mut search_idx = 0;

    while let Some(url_idx) = css[search_idx..].find("url(") {
        let abs_url = search_idx + url_idx;
        let val_start = abs_url + 4;
        if let Some(close_idx) = css[val_start..].find(')') {
            let abs_close = val_start + close_idx;
            let raw_url = css[val_start..abs_close]
                .trim()
                .trim_matches('\'')
                .trim_matches('"');
            if !raw_url.starts_with("data:") && !raw_url.starts_with("http") {
                let res_path = resolve_relative_path(css_dir, raw_url);
                if let Ok(bytes) = archive.read_bytes(&res_path) {
                    let mime = EpubArchive::get_mime_type(&res_path);
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    let data_uri = format!("data:{};base64,{}", mime, b64);
                    let target_str = css[abs_url..=abs_close].to_string();
                    let replacement = format!("url(\"{}\")", data_uri);
                    replacements.push((target_str, replacement));
                }
            }
            search_idx = abs_close + 1;
        } else {
            break;
        }
    }

    let mut output = css.to_string();
    for (target, repl) in replacements {
        output = output.replace(&target, &repl);
    }
    output
}

/// Parse `<meta name="viewport" content="width=..., height=...">` from section HTML.
pub fn parse_viewport_meta(html: &str) -> (Option<f64>, Option<f64>) {
    let lower = html.to_lowercase();
    let mut search_idx = 0;

    while let Some(idx) = lower[search_idx..].find("<meta") {
        let abs_idx = search_idx + idx;
        if let Some(abs_close) = find_tag_end(html, abs_idx) {
            let tag = &html[abs_idx..=abs_close];
            if tag.to_lowercase().contains("viewport") {
                if let Some((_, content)) = extract_attr(tag, "content") {
                    let mut width = None;
                    let mut height = None;

                    for pair in content.split(',') {
                        let parts: Vec<&str> = pair.split('=').map(|s| s.trim()).collect();
                        if parts.len() == 2 {
                            if parts[0].eq_ignore_ascii_case("width") {
                                width = parts[1].parse::<f64>().ok();
                            } else if parts[0].eq_ignore_ascii_case("height") {
                                height = parts[1].parse::<f64>().ok();
                            }
                        }
                    }
                    if width.is_some() || height.is_some() {
                        return (width, height);
                    }
                }
            }
            search_idx = abs_close + 1;
        } else {
            break;
        }
    }

    (None, None)
}

/// Sanitize HTML content by stripping <script> blocks, inline event handlers, and javascript: links (B1 & B2 Fix).
pub fn sanitize_html_scripts(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let lower = html.to_lowercase();
    let lower_bytes = lower.as_bytes();
    let len = html.len();

    let mut i = 0;
    let mut in_script = false;

    while i < len {
        if !in_script && lower_bytes[i..].starts_with(b"<script") {
            in_script = true;
            i += 7;
            continue;
        }

        if in_script {
            if lower_bytes[i..].starts_with(b"</script>") {
                in_script = false;
                i += 9;
            } else {
                i += 1;
            }
            continue;
        }

        // B1 Fix: Safely push UTF-8 character instead of casting u8 as char
        if let Some(ch) = html[i..].chars().next() {
            output.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    // B2 Fix: Strip inline event attributes like onload=, onclick= with char-boundary safety
    let mut sanitized = String::with_capacity(output.len());
    let mut search_idx = 0;
    let out_lower = output.to_lowercase();

    while search_idx < output.len() {
        if let Some(on_idx) = out_lower[search_idx..].find(" on") {
            let abs_on = search_idx + on_idx;
            sanitized.push_str(&output[search_idx..abs_on]);

            if let Some(eq_idx) = out_lower[abs_on..].find('=') {
                let attr_name = out_lower[abs_on + 1..abs_on + eq_idx].trim();
                if attr_name.starts_with("on") {
                    let val_start = abs_on + eq_idx + 1;
                    if val_start < output.len() && output.as_bytes()[val_start] == b'"' {
                        if let Some(end_quote) = output[val_start + 1..].find('"') {
                            search_idx = val_start + 1 + end_quote + 1;
                            continue;
                        }
                    }
                }
            }
            sanitized.push_str(" on");
            search_idx = abs_on + 3;
        } else {
            sanitized.push_str(&output[search_idx..]);
            break;
        }
    }

    // Strip href="javascript:..."
    sanitized.replace("href=\"javascript:", "href=\"#disabled_js:")
}
