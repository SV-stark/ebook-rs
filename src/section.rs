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
        _strategy: &AssetDeliveryStrategy,
    ) -> Result<Self, String> {
        let raw_html = archive.read_string(&full_path)?;
        let plain_text = extract_plain_text(&raw_html);
        let plain_text_lower = plain_text.to_lowercase();
        let char_count = plain_text.chars().count();
        let (viewport_width, viewport_height) = parse_viewport_meta(&raw_html);

        Ok(Self {
            index,
            idref,
            href,
            full_path,
            processed_html: raw_html.clone(),
            raw_html,
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
    #[must_use]
    pub fn tokenize_tts_words(&self) -> Vec<TtsWordToken> {
        let mut tokens = Vec::new();
        let mut word_index = 0;
        let mut char_offset = 0;
        let mut in_word = false;
        let mut start_char = 0;
        let mut current_word = String::new();

        for ch in self.plain_text.chars() {
            if ch.is_whitespace() {
                if in_word {
                    tokens.push(TtsWordToken {
                        index: word_index,
                        word: std::mem::take(&mut current_word),
                        char_start: start_char,
                        char_end: char_offset,
                    });
                    word_index += 1;
                    in_word = false;
                }
            } else {
                if !in_word {
                    in_word = true;
                    start_char = char_offset;
                }
                current_word.push(ch);
            }
            char_offset += 1;
        }

        if in_word {
            tokens.push(TtsWordToken {
                index: word_index,
                word: current_word,
                char_start: start_char,
                char_end: char_offset,
            });
        }

        tokens
    }

    /// Wraps plain text words in the processed HTML with `<span id="tts-w-{index}" class="tts-word">` for live SpeechSynthesis word-by-word visual highlighting.
    #[must_use]
    pub fn to_tts_annotated_html(&self) -> String {
        use std::fmt::Write;

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

        let token_char_vecs: Vec<Vec<char>> =
            tokens.iter().map(|t| t.word.chars().collect()).collect();

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
                let token_chars = &token_char_vecs[token_idx];
                let t_len = token_chars.len();

                if i + t_len <= html_chars.len() && html_chars[i..i + t_len] == token_chars[..] {
                    let _ = write!(
                        annotated,
                        "<span id=\"tts-w-{}\" class=\"tts-word\" data-start=\"{}\" data-end=\"{}\">{}</span>",
                        token.index, token.char_start, token.char_end, token.word
                    );
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
                } else if find_ignore_case(slice, close_tag).is_none() {
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

    let result = result.trim();
    // The tag stripper above only removes markup. Parsers such as FB2/PDF/ODT/DOCX/RTF build their
    // section HTML by escaping text nodes (`'` → `&apos;`, `&` → `&amp;`, …), so without this
    // decode step `plain_text` — and therefore search, TTS and word counts — read
    // "&apos;Curiouser and curiouser!&apos;" verbatim.
    crate::dom::decode_html_entities(result)
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

    // Collect every replacement as a byte range plus its new attribute text, then splice them
    // all in a single forward pass. The previous implementation did
    // `output = output.replace(&needle, ..)` once per match, which reallocated and rescanned the
    // whole (and ever-growing, base64-inflated) buffer each time: O(matches x output) work, so a
    // section with 2000 images copied terabytes from a 200 KB source file.
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();

    // Replace image src attributes
    for (start, end, src_val) in regex_find_attr(html, "src") {
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
                    replacements.push((
                        start,
                        end,
                        format!("src=\"data:{};base64,{}\"", mime, b64),
                    ));
                }
            }
            AssetDeliveryStrategy::ResourceStream => {
                let stream_url = format!("resource/{}", res_path);
                replacements.push((start, end, format!("src=\"{}\"", stream_url)));
            }
        }
    }

    // Replace ONLY <link href="*.css"> stylesheet links
    for (start, end, href_val) in regex_find_link_css(html) {
        let res_path = resolve_relative_path(section_dir, &href_val);
        match strategy {
            AssetDeliveryStrategy::InlinedBase64 => {
                if let Ok(css_text) = archive.read_string(&res_path) {
                    let processed_css = process_css_resources(&css_text, &res_path, archive);
                    let b64 =
                        base64::engine::general_purpose::STANDARD.encode(processed_css.as_bytes());
                    replacements.push((
                        start,
                        end,
                        format!("href=\"data:text/css;base64,{}\"", b64),
                    ));
                }
            }
            AssetDeliveryStrategy::ResourceStream => {
                let stream_url = format!("resource/{}", res_path);
                replacements.push((start, end, format!("href=\"{}\"", stream_url)));
            }
        }
    }

    if replacements.is_empty() {
        return html.to_string();
    }

    // Splice: sort by start offset and emit untouched spans verbatim between replacements.
    replacements.sort_by_key(|(start, _, _)| *start);
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;
    for (start, end, replacement) in replacements {
        // Skip overlapping/duplicate ranges; a range must begin at or after the cursor.
        if start < cursor || end < start || end >= html.len() {
            continue;
        }
        out.push_str(&html[cursor..start]);
        out.push_str(&replacement);
        cursor = end + 1;
    }
    out.push_str(&html[cursor..]);
    out
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
/// Collect `<link href="...">` stylesheet attributes as `(start, end_inclusive, value)` byte
/// ranges into `html`, so callers can splice replacements in a single pass.
fn regex_find_link_css(html: &str) -> Vec<(usize, usize, String)> {
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
                if let Some((attr_start, attr_end, val)) = extract_attr(tag_str, "href") {
                    if find_ignore_case(&val, ".css").is_some()
                        || find_ignore_case(tag_str, "rel=\"stylesheet\"").is_some()
                    {
                        list.push((abs_link + attr_start, abs_link + attr_end, val));
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

/// Find an attribute inside a tag, returning `(attr_start, attr_end_inclusive, value)` as byte
/// offsets relative to `tag_str`.
fn extract_attr(tag_str: &str, attr: &str) -> Option<(usize, usize, String)> {
    let quotes = ['"', '\''];
    let prefixes = [' ', '<'];

    for &prefix in &prefixes {
        for &quote in &quotes {
            let pat = format!("{prefix}{attr}={quote}");
            if let Some(pos) = find_ignore_case(tag_str, &pat) {
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
                            let val = tag_str[val_start..val_end].to_string();
                            return Some((attr_start, val_end, val));
                        }
                    }
                }
            }
        }
    }
    None
}

/// Helper to find attributes like `src="..."` or `href="..."` with word boundary checks (E4 Fix).
///
/// Returns `(start, end_inclusive, value)` byte ranges into `html`.
fn regex_find_attr(html: &str, attr: &str) -> Vec<(usize, usize, String)> {
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

        // Skip if inside <pre>, <code>, <script>, or <style> blocks
        let before_slice = &html[..abs_start];
        let inside_code = ["<pre", "<code", "<script", "<style"].iter().any(|open| {
            let close = format!("</{}", &open[1..]);
            let last_o = before_slice.rfind(open);
            let last_c = before_slice.rfind(&close);
            match (last_o, last_c) {
                (Some(_o), None) => true,
                (Some(o), Some(c)) => o > c,
                _ => false,
            }
        });

        if inside_code {
            search_idx = abs_start + 1;
            continue;
        }

        let val_start = abs_start + attr.len() + 2;
        if html.is_char_boundary(val_start) {
            if let Some(end) = html[val_start..].find('"') {
                let abs_end = val_start + end;
                if html.is_char_boundary(abs_end) {
                    let val = html[val_start..abs_end].to_string();
                    list.push((abs_start, abs_end, val));
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
                if let Ok(mut bytes) = archive.read_bytes(&res_path) {
                    if let Ok(enc_xml) = archive.read_string("META-INF/encryption.xml") {
                        let deobf =
                            crate::deobfuscate::FontDeobfuscator::parse_encryption_xml(&enc_xml);
                        if deobf.is_encrypted(&res_path) {
                            let opf_path = archive
                                .get_opf_path()
                                .unwrap_or_else(|_| "OEBPS/content.opf".to_string());
                            let identifier = archive
                                .read_string(&opf_path)
                                .or_else(|_| archive.read_string("content.opf"))
                                .ok()
                                .and_then(|xml| crate::opf::parse_opf(&xml, &opf_path).ok())
                                .map(|pkg| pkg.metadata.identifier.clone().unwrap_or_default())
                                .unwrap_or_default();
                            deobf.deobfuscate(&res_path, &mut bytes, &identifier);
                        }
                    }
                    let mime = EpubArchive::get_mime_type(&res_path);
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    use std::fmt::Write;
                    let _ = write!(output, "url(\"data:{mime};base64,{b64}\")");
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
                    if let Some((_, _, content)) = extract_attr(tag, "content") {
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

    // Phase 1: Strip <script>...</script>, <iframe>, <object>, <embed> with fast memchr tag scanning
    let bytes = html.as_bytes();
    while i < len {
        if let Some(tag_offset) = memchr::memchr(b'<', &bytes[i..]) {
            let abs_tag = i + tag_offset;
            if abs_tag > i {
                output.push_str(&html[i..abs_tag]);
            }
            i = abs_tag;

            let slice = &html[i..];
            if starts_with_ignore_case(slice, "<script")
                || starts_with_ignore_case(slice, "<iframe")
                || starts_with_ignore_case(slice, "<object")
                || starts_with_ignore_case(slice, "<embed")
                || starts_with_ignore_case(slice, "<applet")
                || starts_with_ignore_case(slice, "<base")
                || starts_with_ignore_case(slice, "<form")
                || (starts_with_ignore_case(slice, "<meta") && {
                    // Scan the *whole* tag, not a fixed-width prefix: a `<meta>` padded past any
                    // byte budget with benign attributes (`name="…"`) must not be able to hide an
                    // `http-equiv=refresh` at its end. `find_tag_end` respects quoted attributes;
                    // an unterminated tag is dropped a few lines below anyway, so a bounded
                    // fallback is sufficient there.
                    let tag_scan = find_tag_end(html, i)
                        .map_or_else(|| slice.get(..4096).unwrap_or(slice), |p| &html[i..=p]);
                    find_ignore_case(tag_scan, "http-equiv").is_some()
                        || find_ignore_case(tag_scan, "refresh").is_some()
                })
            {
                if let Some(close_tag_pos) = find_tag_end(html, i) {
                    let tag_str = &html[i..=close_tag_pos];
                    let is_self_closing = tag_str.ends_with("/>")
                        || starts_with_ignore_case(slice, "<base")
                        || starts_with_ignore_case(slice, "<meta");
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
                    } else if starts_with_ignore_case(slice, "<applet") {
                        "</applet>"
                    } else if starts_with_ignore_case(slice, "<form") {
                        "</form>"
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
        } else {
            output.push_str(&html[i..]);
            break;
        }
    }

    // Phase 2 (B8 Fix): Strip inline event attributes like onload=, onclick= (only inside HTML tags)
    let mut sanitized = String::with_capacity(output.len());
    let mut idx = 0;
    let mut in_tag = false;
    let mut in_quote: Option<char> = None;
    // Set on the iteration where a quoted attribute value closes. The HTML5 tokenizer re-consumes
    // the following character in the "before attribute name" state, so `<img src="x"onerror=…>`
    // really does create a *separate* `onerror` attribute despite the missing space. Treating the
    // closing quote as an attribute-name boundary closes that bypass.
    let mut just_closed_quote = false;

    while idx < output.len() {
        if !output.is_char_boundary(idx) {
            idx += 1;
            continue;
        }
        // Consume the flag for this iteration; it is re-set below if a quote closes on `ch`.
        let quote_was_boundary = just_closed_quote;
        just_closed_quote = false;
        let slice = &output[idx..];
        if let Some(ch) = slice.chars().next() {
            if !in_tag && ch == '<' {
                in_tag = true;
                in_quote = None;
            } else if in_tag {
                if let Some(q) = in_quote {
                    if ch == q {
                        in_quote = None;
                        just_closed_quote = true;
                    }
                } else if ch == '"' || ch == '\'' {
                    in_quote = Some(ch);
                } else if ch == '>' {
                    in_tag = false;
                    in_quote = None;
                }
            }

            if in_tag && (quote_was_boundary || ch.is_whitespace() || ch == '<' || ch == '/') {
                // Where the attribute name begins, relative to `idx`. For a whitespace / `<` /
                // `/` separator that is just past `ch`; when the separator was a closing quote it
                // is `ch` itself, because the HTML5 tokenizer re-consumes that character in the
                // "before attribute name" state and the quote has already been emitted.
                let name_offset = if quote_was_boundary { 0 } else { ch.len_utf8() };
                let rest = &slice[name_offset..];
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
                        if !quote_was_boundary {
                            sanitized.push(ch);
                        }
                        sanitized.push_str(ws_bytes);
                        idx += name_offset
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
        let first_byte = slice.as_bytes().first().copied().unwrap_or(0);
        let might_be_scheme = matches!(first_byte, b'j' | b'J' | b'v' | b'V' | b'd' | b'D' | b'&');

        let match_info = if might_be_scheme {
            let mut window_len = slice.len().min(64);
            while !slice.is_char_boundary(window_len) {
                window_len -= 1;
            }
            let window = &slice[..window_len];
            // Probe the scheme on a *normalized* view of the text. Browsers (per the WHATWG URL
            // standard) strip ASCII tab/LF/CR from a URL before parsing its scheme, and HTML5
            // defines the named references `&Tab;` / `&NewLine;`. Checking only the raw text or
            // a numeric-entity-decoded copy therefore misses `java<TAB>script:`,
            // `java&#9;script:` and `java&Tab;script:`, all of which execute.
            let probe = normalize_uri_probe(window);

            if starts_with_ignore_case(&probe, "javascript:")
                || starts_with_ignore_case(&probe, "vbscript:")
                || starts_with_ignore_case(&probe, "data:text/html")
            {
                // Advance through the terminating `:` in the ORIGINAL text so the splice offset
                // always lines up with `sanitized` — see `scheme_prefix_bytes`.
                Some(scheme_prefix_bytes(window))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(advance_len) = match_info {
            final_sanitized.push_str("#disabled_uri:");
            cur_idx += advance_len.min(slice.len());
        } else if let Some(ch) = slice.chars().next() {
            final_sanitized.push(ch);
            cur_idx += ch.len_utf8();
        } else {
            break;
        }
    }

    final_sanitized
}

/// Normalize text before a URI scheme check.
///
/// Decodes HTML entities (numeric *and* the control-character named references) and then strips
/// the ASCII whitespace characters the WHATWG URL parser removes prior to scheme resolution.
/// Without the strip step, `java<TAB>script:` survives a contiguous-prefix check while still
/// executing as script in every browser.
fn normalize_uri_probe(input: &str) -> String {
    decode_html_entities_for_uri(input)
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect()
}

/// Decode HTML character references for URI scheme inspection.
///
/// Handles numeric references (`&#9;`, `&#x09;`) and the named references that matter for
/// scheme smuggling (`&Tab;`, `&NewLine;`). Control characters are dropped from decoded output,
/// except for the tab/newline names, which are emitted so [`normalize_uri_probe`] can strip them
/// the same way a browser would.
fn decode_html_entities_for_uri(input: &str) -> String {
    /// Named references usable to smuggle a scheme past a contiguous-prefix check.
    ///
    /// `colon` is required: HTML5 decodes `&colon;` inside attribute values, so
    /// `javascript&colon;alert(1)` is a live URI in every browser while a raw prefix check on
    /// `javascript:` sees nothing.
    const NAMED: &[(&str, char)] = &[
        ("Tab", '\t'),
        ("tab", '\t'),
        ("NewLine", '\n'),
        ("newline", '\n'),
        ("colon", ':'),
        ("Colon", ':'),
    ];

    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();

    'outer: while i < bytes.len() {
        if bytes[i] == b'&' && i + 1 < bytes.len() {
            // Named reference (e.g. `&Tab;`, `&NewLine`).
            for (name, ch) in NAMED {
                if let Some(after_amp) = input[i + 1..].strip_prefix(name) {
                    let len = name.len() + 1 + usize::from(after_amp.starts_with(';'));
                    out.push(*ch);
                    i += len;
                    continue 'outer;
                }
            }

            // Numeric reference (e.g. `&#9;`, `&#x09;`).
            if bytes[i + 1] == b'#' && i + 2 < bytes.len() {
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
                            continue 'outer;
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

/// Decode the single character reference at `input[i]`, returning the character and the number of
/// input bytes it consumed. `None` when `input[i]` is not `&` or does not start a reference.
///
/// Covers the same reference set as [`decode_html_entities_for_uri`], which is what a browser
/// resolves while reading an attribute value.
fn decode_uri_step(input: &str, i: usize) -> Option<(char, usize)> {
    let rest = &input[i..];
    if !rest.starts_with('&') {
        return None;
    }
    let semi = rest.find(';')?;
    if semi > 12 || i + semi + 1 > input.len() {
        return None;
    }
    let name = &rest[1..semi];
    let len = semi + 1;
    let ch = match name {
        "Tab" | "tab" => '\t',
        "NewLine" | "newline" => '\n',
        "colon" | "Colon" => ':',
        _ => {
            let digits = name.strip_prefix('#')?;
            let code = if let Some(hex) = digits
                .strip_prefix('x')
                .or_else(|| digits.strip_prefix('X'))
            {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                digits.parse::<u32>().ok()?
            };
            char::from_u32(code)?
        }
    };
    Some((ch, len))
}

/// Byte length of the leading scheme prefix of `window`, through the `:` that ends the scheme.
///
/// The terminating `:` may be spelled literally *or* as a character reference (`&colon;`,
/// `&#58;`, `&#x3a;`), so the offset cannot be recovered with a plain `find(':')`. For
/// `javascript&colon;alert(1)` there is no literal colon at all, and falling back to a fixed
/// byte count replaces only a prefix of the scheme while leaving `lon;…` behind.
fn scheme_prefix_bytes(window: &str) -> usize {
    let bytes = window.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b':' => return i + 1,
            b'&' => match decode_uri_step(window, i) {
                Some((ch, len)) => {
                    let end = i + len.min(bytes.len() - i);
                    if ch == ':' {
                        return end;
                    }
                    i = end;
                }
                // A `&` that does not begin a character reference is kept by the URL parser, so
                // the scheme cannot extend beyond it.
                None => return i,
            },
            c if c.is_ascii_alphanumeric()
                || matches!(c, b'+' | b'-' | b'.' | b'\t' | b'\r' | b'\n') =>
            {
                i += 1
            }
            _ => return i,
        }
    }
    window.len()
}

#[cfg(test)]
mod security_regression_tests {
    use super::*;

    /// A `javascript:` URI must be neutralized no matter how the scheme is obfuscated with
    /// characters a browser strips or decodes before resolving it.
    #[test]
    fn javascript_uri_scheme_smuggling_is_neutralized() {
        let payloads = [
            // Plain form (baseline).
            r#"<a href="javascript:alert(1)">x</a>"#,
            // Case variations.
            r#"<a href="JavaScript:alert(1)">x</a>"#,
            r#"<a href="JAVASCRIPT:alert(1)">x</a>"#,
            // Numeric character references for tab.
            "<a href=\"java&#9;script:alert(1)\">x</a>",
            "<a href=\"java&#x09;script:alert(1)\">x</a>",
            // HTML5 named references for tab / newline.
            "<a href=\"java&Tab;script:alert(1)\">x</a>",
            "<a href=\"java&NewLine;script:alert(1)\">x</a>",
            // Literal control characters, which browsers strip from URLs before scheme parsing.
            "<a href=\"java\tscript:alert(1)\">x</a>",
            "<a href=\"java\nscript:alert(1)\">x</a>",
            "<a href=\"java\rscript:alert(1)\">x</a>",
            // Other dangerous schemes.
            r#"<a href="vbscript:msgbox(1)">x</a>"#,
            r#"<a href="data:text/html;base64,PHNjcmlwdD4=">x</a>"#,
        ];

        for payload in payloads {
            let out = sanitize_html_scripts(payload);
            assert!(
                !out.to_ascii_lowercase().contains("javascript:")
                    && !out.to_ascii_lowercase().contains("vbscript:")
                    && !out.to_ascii_lowercase().contains("data:text/html"),
                "scheme survived sanitization for payload {payload:?} -> {out:?}"
            );
            assert!(
                out.contains("#disabled_uri:"),
                "payload was not neutralized at all: {payload:?} -> {out:?}"
            );
        }
    }

    /// A legitimate URL containing the substring "javascript" later in the path must survive.
    #[test]
    fn benign_urls_are_not_neutralized() {
        for payload in [
            r#"<a href="https://example.com/docs/javascript.html">x</a>"#,
            r#"<img src="images/photo.png"/>"#,
            r#"<link rel="stylesheet" href="style.css"/>"#,
        ] {
            let out = sanitize_html_scripts(payload);
            assert!(
                !out.contains("#disabled_uri:"),
                "benign URL was wrongly neutralized: {payload:?} -> {out:?}"
            );
        }
    }

    #[test]
    fn script_and_event_handlers_are_stripped() {
        let out = sanitize_html_scripts(
            r#"<div onclick="steal()"><script>bad()</script><p>ok</p></div>"#,
        );
        assert!(!out.contains("onclick"), "{out}");
        assert!(!out.contains("<script"), "{out}");
        assert!(out.contains("ok"), "{out}");
    }

    /// The HTML5 tokenizer re-consumes the character after a quoted attribute value in the
    /// "before attribute name" state, so `"x"onerror=` creates a *separate* `onerror`
    /// attribute even with no separator. The handler must not survive.
    #[test]
    fn event_handler_after_closing_quote_is_stripped() {
        for payload in [
            r#"<img src="x"onerror="alert(1)">"#,
            "<img src='x'onerror='alert(1)'>",
            r#"<img src="x"onerror="alert(1)" src="y">"#,
        ] {
            let out = sanitize_html_scripts(payload);
            assert!(
                !out.to_ascii_lowercase().contains("onerror"),
                "handler survived: {payload:?} -> {out:?}"
            );
        }
        // The legitimate `src` attribute must be preserved.
        let out = sanitize_html_scripts(r#"<img src="x"onerror="alert(1)">"#);
        assert!(out.contains(r#"src="x""#), "{out}");
    }

    /// `&colon;` is decoded by browsers inside attribute values, so `javascript&colon;…` is a
    /// live URI. It is also the one scheme spelling with no literal `:` in the source.
    #[test]
    fn colon_entity_scheme_is_neutralized() {
        for payload in [
            r#"<a href="javascript&colon;alert(1)">x</a>"#,
            r#"<a href="javascript&#58;alert(1)">x</a>"#,
            "<a href=\"javascript&#x3a;alert(1)\">x</a>",
        ] {
            let out = sanitize_html_scripts(payload);
            assert!(
                !out.to_ascii_lowercase().contains("javascript:"),
                "scheme survived: {payload:?} -> {out:?}"
            );
            assert!(
                out.contains("#disabled_uri:"),
                "payload was not neutralized: {payload:?} -> {out:?}"
            );
        }
    }

    /// A `<meta http-equiv=refresh>` must be found wherever it appears in the tag, not only in a
    /// fixed-width prefix. CSP does not apply here: meta refresh is navigation, not script.
    #[test]
    fn long_meta_tags_still_hide_refresh() {
        let padding = "a".repeat(4000);
        let payload = format!(
            r#"<p>keep</p><meta name="{padding}" http-equiv="refresh" content="0;url=http://evil.com/">"#
        );
        let out = sanitize_html_scripts(&payload);
        assert!(!out.contains("http-equiv"), "{out}");
        assert!(!out.contains("evil.com"), "{out}");
        assert!(out.contains("keep"), "{out}");
    }

    /// The probe offset must be computed from the decoded scheme, not from a literal colon
    /// search: `javascript&colon;` has no `:` in the source, so `find(':')` returned `None` and
    /// the old fixed fallback left the tail `lon;alert(1)` in place.
    #[test]
    fn scheme_prefix_bytes_tracks_entity_spelled_colon() {
        // The splice must cover the whole scheme name plus its terminating `:`. For
        // `data:text/html` the scheme name ends at the first `:`, which is already enough to kill
        // the URI — what remains (`text/html,…`) is left behind a `#` and is inert.
        for (window, scheme) in [
            ("javascript:alert(1)", "javascript:"),
            ("javascript&colon;alert(1)", "javascript&colon;"),
            ("javascript&#58;alert(1)", "javascript&#58;"),
            ("javascript&#x3a;alert(1)", "javascript&#x3a;"),
            ("vbscript:msgbox(1)", "vbscript:"),
            ("data:text/html,x", "data:"),
            ("JAVASCRIPT:alert(1)", "JAVASCRIPT:"),
            ("java&Tab;script:alert(1)", "java&Tab;script:"),
            ("java&#9;script:alert(1)", "java&#9;script:"),
        ] {
            assert_eq!(
                &window[..scheme_prefix_bytes(window)],
                scheme,
                "wrong scheme boundary in {window:?}"
            );
        }
        // A `&` that does not start a reference terminates the scheme, and so does any
        // non-scheme character (the scan stops at the `:` or the first byte that cannot be part
        // of a scheme name).
        assert_eq!(scheme_prefix_bytes("java& script:"), 4);
        assert_eq!(scheme_prefix_bytes("no colon here"), 2);
        assert_eq!(scheme_prefix_bytes("<a href="), 0);
    }
}
