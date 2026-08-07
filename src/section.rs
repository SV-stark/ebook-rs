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

/// Tokenized word entry with character range offsets for SpeechSynthesis TTS word synchronization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtsWordToken {
    pub index: usize,
    pub word: String,
    pub char_start: usize,
    pub char_end: usize,
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

    /// Detects the primary language of this section's plain text content using `whatlang`.
    pub fn detect_language(&self) -> Option<String> {
        if self.plain_text.len() >= 30 {
            whatlang::detect(&self.plain_text).map(|info| info.lang().code().to_string())
        } else {
            None
        }
    }

    /// Tokenizes plain text into word tokens with exact character start/end offsets for SpeechSynthesis TTS synchronization.
    pub fn tokenize_tts_words(&self) -> Vec<TtsWordToken> {
        let mut tokens = Vec::new();
        let mut word_index = 0;
        let mut char_offset = 0;
        let plain_chars: Vec<char> = self.plain_text.chars().collect();

        let mut i = 0;
        while i < plain_chars.len() {
            if plain_chars[i].is_whitespace() {
                i += 1;
                char_offset += 1;
                continue;
            }

            let start_char = char_offset;
            let mut word = String::new();
            while i < plain_chars.len() && !plain_chars[i].is_whitespace() {
                word.push(plain_chars[i]);
                i += 1;
                char_offset += 1;
            }

            tokens.push(TtsWordToken {
                index: word_index,
                word,
                char_start: start_char,
                char_end: char_offset,
            });
            word_index += 1;
        }

        tokens
    }

    /// Wraps plain text words in the processed HTML with `<span id="tts-w-{index}" class="tts-word">` for live SpeechSynthesis word-by-word visual highlighting.
    pub fn to_tts_annotated_html(&self) -> String {
        let tokens = self.tokenize_tts_words();
        if tokens.is_empty() {
            return self.processed_html.clone();
        }

        let mut annotated = String::with_capacity(self.processed_html.len() + tokens.len() * 50);
        let mut token_idx = 0;
        let mut in_tag = false;
        let mut in_quote: Option<char> = None;

        let html_chars: Vec<char> = self.processed_html.chars().collect();
        let mut i = 0;

        while i < html_chars.len() {
            let ch = html_chars[i];
            if !in_tag && ch == '<' {
                in_tag = true;
                in_quote = None;
                annotated.push(ch);
                i += 1;
                continue;
            }

            if in_tag {
                annotated.push(ch);
                if let Some(q) = in_quote {
                    if ch == q {
                        in_quote = None;
                    }
                } else if ch == '"' || ch == '\'' {
                    in_quote = Some(ch);
                } else if ch == '>' {
                    in_tag = false;
                }
                i += 1;
                continue;
            }

            if token_idx < tokens.len() {
                let token = &tokens[token_idx];
                let token_chars: Vec<char> = token.word.chars().collect();
                let t_len = token_chars.len();

                if i + t_len <= html_chars.len() && html_chars[i..i + t_len] == token_chars[..] {
                    annotated.push_str(&format!(
                        "<span id=\"tts-w-{}\" class=\"tts-word\" data-start=\"{}\" data-end=\"{}\">{}</span>",
                        token.index, token.char_start, token.char_end, token.word
                    ));
                    i += t_len;
                    token_idx += 1;
                    continue;
                }
            }

            annotated.push(ch);
            i += 1;
        }

        annotated
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

fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    if let Some(sub) = s.get(..prefix.len()) {
        sub.eq_ignore_ascii_case(prefix)
    } else {
        false
    }
}

/// Extract clean plain text from HTML content by stripping tags, styles, and scripts.
/// UTF-8 char boundary safe and quote-aware extraction.
pub fn extract_plain_text(html: &str) -> String {
    let mut in_tag = false;
    let mut in_quote: Option<u8> = None;
    let mut text = String::with_capacity(html.len());
    let mut skipping_tag: Option<&'static str> = None;

    let len = html.len();
    let mut i = 0;

    while i < len {
        if !in_tag && html.as_bytes()[i] == b'<' {
            in_tag = true;
            in_quote = None;
            let slice = &html[i..];
            if skipping_tag.is_none() {
                if starts_with_ignore_case(slice, "<style") {
                    skipping_tag = Some("style");
                } else if starts_with_ignore_case(slice, "<script") {
                    skipping_tag = Some("script");
                }
            } else if let Some(tag) = skipping_tag {
                let close_tag = if tag == "style" {
                    "</style"
                } else {
                    "</script"
                };
                if starts_with_ignore_case(slice, close_tag) {
                    skipping_tag = None;
                } else if find_ignore_case(html, close_tag).is_none() {
                    // Unclosed style/script recovery: if no closing tag exists anywhere in document,
                    // recover when encountering structural HTML block tags (<p, <div, <body, <h, <section)
                    if starts_with_ignore_case(slice, "<p")
                        || starts_with_ignore_case(slice, "<div")
                        || starts_with_ignore_case(slice, "<body")
                        || starts_with_ignore_case(slice, "<h")
                        || starts_with_ignore_case(slice, "<section")
                    {
                        skipping_tag = None;
                    }
                }
            }
            text.push(' ');
            i += 1;
            continue;
        }

        if in_tag {
            let b = html.as_bytes()[i];
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

/// Helper to find ONLY `<link ... href="...">` tags for CSS inlining (E5 Fix).
fn regex_find_link_css(html: &str) -> Vec<(String, String)> {
    let mut list = Vec::new();
    let mut search_idx = 0;

    while search_idx < html.len() {
        if !html.is_char_boundary(search_idx) {
            search_idx += 1;
            continue;
        }
        if let Some(link_idx) = find_ignore_case(&html[search_idx..], "<link") {
            let abs_link = search_idx + link_idx;
            if let Some(abs_close) = find_tag_end(html, abs_link) {
                let tag_str = &html[abs_link..=abs_close];
                if let Some((orig_href, val)) = extract_attr(tag_str, "href") {
                    if find_ignore_case(&val, ".css").is_some()
                        || find_ignore_case(tag_str, "rel=\"stylesheet\"").is_some()
                    {
                        list.push((orig_href, val));
                    }
                }
                search_idx = abs_close + 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    list
}

fn extract_attr(tag_str: &str, attr: &str) -> Option<(String, String)> {
    let attr_lower = attr.to_lowercase();
    let pat1 = format!(" {}=\"", attr_lower);
    let pat2 = format!("<{}=\"", attr_lower);
    let pat3 = format!(" {}='", attr_lower);
    let pat4 = format!("<{}='", attr_lower);

    for pat in &[pat1, pat2, pat3, pat4] {
        if let Some(pos) = find_ignore_case(tag_str, pat) {
            let quote = pat.chars().last().unwrap();
            let attr_start = pos + 1;
            let val_start = pos + pat.len();

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
    let pattern1 = format!(" {}=\"", attr);
    let pattern2 = format!("<{}=\"", attr);
    let mut search_idx = 0;

    while search_idx < html.len() {
        if !html.is_char_boundary(search_idx) {
            search_idx += 1;
            continue;
        }
        let slice = &html[search_idx..];
        let p1_match = find_ignore_case(slice, &pattern1).map(|s| search_idx + s + 1);
        let p2_match = find_ignore_case(slice, &pattern2).map(|s| search_idx + s + 1);

        let abs_start = match (p1_match, p2_match) {
            (Some(m1), Some(m2)) => m1.min(m2),
            (Some(m1), None) => m1,
            (None, Some(m2)) => m2,
            (None, None) => break,
        };

        let val_start = abs_start + attr.len() + 2;
        if html.is_char_boundary(val_start) {
            if let Some(end) = html[val_start..].find('"') {
                let abs_end = val_start + end;
                if html.is_char_boundary(abs_end) {
                    let val = &html[val_start..abs_end];
                    let orig = &html[abs_start..=abs_end];
                    list.push((orig.to_string(), val.to_string()));
                    search_idx = abs_end + 1;
                    continue;
                }
            }
        }
        search_idx = abs_start + 1;
    }

    list
}

/// Inline fonts and images inside CSS stylesheet content (P7 Fix: Single-pass streaming builder).
fn process_css_resources(css: &str, css_path: &str, archive: &EpubArchive) -> String {
    let css_dir = if let Some(idx) = css_path.rfind('/') {
        &css_path[..idx]
    } else {
        ""
    };

    let mut output = String::with_capacity(css.len());
    let mut search_idx = 0;

    while let Some(url_idx) = css[search_idx..].find("url(") {
        let abs_url = search_idx + url_idx;
        output.push_str(&css[search_idx..abs_url]);

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
                    output.push_str(&format!("url(\"data:{};base64,{}\")", mime, b64));
                    search_idx = abs_close + 1;
                    continue;
                }
            }
            output.push_str(&css[abs_url..=abs_close]);
            search_idx = abs_close + 1;
        } else {
            output.push_str(&css[abs_url..]);
            search_idx = css.len();
            break;
        }
    }

    if search_idx < css.len() {
        output.push_str(&css[search_idx..]);
    }

    output
}

/// Parse `<meta name="viewport" content="width=..., height=...">` from section HTML.
pub fn parse_viewport_meta(html: &str) -> (Option<f64>, Option<f64>) {
    let mut search_idx = 0;

    while search_idx < html.len() {
        if !html.is_char_boundary(search_idx) {
            search_idx += 1;
            continue;
        }
        if let Some(idx) = find_ignore_case(&html[search_idx..], "<meta") {
            let abs_idx = search_idx + idx;
            if let Some(abs_close) = find_tag_end(html, abs_idx) {
                let tag = &html[abs_idx..=abs_close];
                if find_ignore_case(tag, "viewport").is_some() {
                    if let Some((_, content)) = extract_attr(tag, "content") {
                        let mut width = None;
                        let mut height = None;

                        for pair in content.split(',') {
                            if let Some((k, v)) = pair.split_once('=') {
                                let k = k.trim();
                                let v = v.trim();
                                if k.eq_ignore_ascii_case("width") {
                                    width = v.parse::<f64>().ok();
                                } else if k.eq_ignore_ascii_case("height") {
                                    height = v.parse::<f64>().ok();
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
        } else {
            break;
        }
    }

    (None, None)
}

/// Sanitize HTML content by stripping <script> blocks, inline event handlers, and javascript: links.
pub fn sanitize_html_scripts(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let len = html.len();
    let mut i = 0;

    // Phase 1: Strip <script>...</script>, <iframe>, <object>, <embed> without swallowing unclosed tags
    while i < len {
        if !html.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let slice = &html[i..];
        if starts_with_ignore_case(slice, "<script")
            || starts_with_ignore_case(slice, "<iframe")
            || starts_with_ignore_case(slice, "<object")
            || starts_with_ignore_case(slice, "<embed")
        {
            if let Some(close_tag_pos) = find_tag_end(html, i) {
                let tag_str = &html[i..=close_tag_pos];
                let is_self_closing = tag_str.ends_with("/>");
                if is_self_closing {
                    i = close_tag_pos + 1;
                    continue;
                }

                let tag_name = if starts_with_ignore_case(slice, "<script") {
                    "</script>"
                } else if starts_with_ignore_case(slice, "<iframe") {
                    "</iframe>"
                } else if starts_with_ignore_case(slice, "<object") {
                    "</object>"
                } else {
                    "</embed>"
                };

                if let Some(end_idx) = find_ignore_case(&html[close_tag_pos + 1..], tag_name) {
                    let end_pos = close_tag_pos + 1 + end_idx + tag_name.len();
                    i = end_pos;
                    continue;
                } else {
                    // Unclosed tag: strip just opening tag element
                    i = close_tag_pos + 1;
                    continue;
                }
            } else {
                i += 1;
                continue;
            }
        }

        if let Some(ch) = slice.chars().next() {
            output.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    // Phase 2 (B8 Fix): Strip inline event attributes like onload=, onclick= (only inside HTML tags)
    let mut sanitized = String::with_capacity(output.len());
    let mut idx = 0;
    let mut in_tag = false;
    let mut in_quote: Option<char> = None;

    while idx < output.len() {
        if !output.is_char_boundary(idx) {
            idx += 1;
            continue;
        }
        let slice = &output[idx..];
        if let Some(ch) = slice.chars().next() {
            if !in_tag && ch == '<' {
                in_tag = true;
                in_quote = None;
            } else if in_tag {
                if let Some(q) = in_quote {
                    if ch == q {
                        in_quote = None;
                    }
                } else if ch == '"' || ch == '\'' {
                    in_quote = Some(ch);
                } else if ch == '>' {
                    in_tag = false;
                    in_quote = None;
                }
            }

            if in_tag && (ch.is_whitespace() || ch == '<' || ch == '/') {
                let rest = &slice[ch.len_utf8()..];
                let trimmed_rest = rest.trim_start();
                let ws_bytes = &rest[..rest.len() - trimmed_rest.len()];

                if starts_with_ignore_case(trimmed_rest, "on") {
                    let mut attr_len = 2;
                    while attr_len < trimmed_rest.len()
                        && trimmed_rest.as_bytes()[attr_len].is_ascii_alphanumeric()
                    {
                        attr_len += 1;
                    }
                    let after_attr = trimmed_rest[attr_len..].trim_start();
                    if let Some(_stripped) = after_attr.strip_prefix('=') {
                        sanitized.push(ch);
                        sanitized.push_str(ws_bytes);
                        idx += ch.len_utf8()
                            + ws_bytes.len()
                            + (trimmed_rest.len() - after_attr.len())
                            + 1;

                        if idx < output.len() {
                            let val_slice = &output[idx..];
                            let trimmed_val = val_slice.trim_start();
                            idx += val_slice.len() - trimmed_val.len();

                            if let Some(quote_ch) = trimmed_val.chars().next() {
                                if quote_ch == '"' || quote_ch == '\'' {
                                    idx += quote_ch.len_utf8();
                                    if let Some(end_q) = output[idx..].find(quote_ch) {
                                        idx += end_q + quote_ch.len_utf8();
                                    } else {
                                        idx = output.len();
                                    }
                                } else {
                                    while idx < output.len() {
                                        let c = output[idx..].chars().next().unwrap_or(' ');
                                        if c.is_whitespace() || c == '>' || c == '/' {
                                            if c == '>' {
                                                in_tag = false;
                                            }
                                            break;
                                        }
                                        idx += c.len_utf8();
                                    }
                                }
                            }
                        }
                        continue;
                    }
                }
            }

            sanitized.push(ch);
            idx += ch.len_utf8();
        } else {
            break;
        }
    }

    // Phase 3 (B9 Fix): Neutralize javascript:, vbscript:, data:text/html URIs (decoding all HTML entities first)
    let mut final_sanitized = String::with_capacity(sanitized.len());
    let mut cur_idx = 0;
    while cur_idx < sanitized.len() {
        if !sanitized.is_char_boundary(cur_idx) {
            cur_idx += 1;
            continue;
        }
        let slice = &sanitized[cur_idx..];
        let decoded_slice = decode_html_entities_for_uri(slice);
        if starts_with_ignore_case(&decoded_slice, "javascript:")
            || starts_with_ignore_case(slice, "javascript:")
            || starts_with_ignore_case(&decoded_slice, "vbscript:")
            || starts_with_ignore_case(slice, "vbscript:")
            || starts_with_ignore_case(&decoded_slice, "data:text/html")
            || starts_with_ignore_case(slice, "data:text/html")
        {
            final_sanitized.push_str("#disabled_uri:");
            cur_idx += "javascript:".len().min(slice.len());
        } else if let Some(ch) = slice.chars().next() {
            final_sanitized.push(ch);
            cur_idx += ch.len_utf8();
        } else {
            break;
        }
    }

    final_sanitized
}

fn decode_html_entities_for_uri(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'&' && i + 2 < bytes.len() {
            if bytes[i + 1] == b'#' {
                let is_hex = bytes[i + 2] == b'x' || bytes[i + 2] == b'X';
                let start = if is_hex { i + 3 } else { i + 2 };
                let mut end = start;
                while end < bytes.len() && bytes[end] != b';' && (end - start) < 8 {
                    end += 1;
                }
                if end < bytes.len() && bytes[end] == b';' {
                    let num_str = &input[start..end];
                    let parsed = if is_hex {
                        u32::from_str_radix(num_str, 16).ok()
                    } else {
                        num_str.parse::<u32>().ok()
                    };
                    if let Some(code) = parsed {
                        if let Some(ch) = char::from_u32(code) {
                            if !ch.is_control() {
                                out.push(ch);
                            }
                            i = end + 1;
                            continue;
                        }
                    }
                }
            }
        }
        if let Some(ch) = input[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }
    out
}
