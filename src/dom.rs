use ahash::AHashMap;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// A node in the lightweight Ebook DOM AST tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DomNode {
    Element {
        tag_name: CompactString,
        attributes: AHashMap<CompactString, CompactString>,
        children: Vec<DomNode>,
    },
    Text(String),
    Comment(String),
}

/// Decodes a raw byte slice into a UTF-8 String using `encoding_rs`.
/// Supports legacy non-UTF-8 encodings such as "windows-1252", "shift_jis", "gbk", "iso-8859-1", etc.
pub fn decode_bytes_with_encoding(bytes: &[u8], encoding_label: Option<&str>) -> String {
    if let Some(label) = encoding_label {
        if let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) {
            let (cow, _malformed, _replacements) = encoding.decode(bytes);
            return cow.into_owned();
        }
    }
    let (cow, _malformed, _replacements) = encoding_rs::UTF_8.decode(bytes);
    cow.into_owned()
}

/// Lightweight, zero-alloc DOM AST tree parser and manipulator for eBook HTML sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbookDomTree {
    pub root_nodes: Vec<DomNode>,
}

impl EbookDomTree {
    /// Parse HTML string into lightweight DOM AST tree.
    pub fn parse(html: &str) -> Self {
        let mut root_nodes: Vec<DomNode> = Vec::new();
        let mut stack: Vec<(
            CompactString,
            AHashMap<CompactString, CompactString>,
            Vec<DomNode>,
        )> = Vec::new();
        let mut search_idx = 0;
        let bytes = html.as_bytes();

        let void_tags = [
            "img", "br", "hr", "meta", "link", "input", "area", "base", "col", "embed", "param",
            "source", "track", "wbr",
        ];

        while search_idx < bytes.len() {
            if let Some(tag_start) = html[search_idx..].find('<') {
                let abs_start = search_idx + tag_start;
                if abs_start > search_idx {
                    let text = &html[search_idx..abs_start];
                    if !text.is_empty() {
                        let node = DomNode::Text(text.to_string());
                        if let Some((_, _, children)) = stack.last_mut() {
                            children.push(node);
                        } else {
                            root_nodes.push(node);
                        }
                    }
                }

                if let Some(tag_end) = crate::section::find_tag_end(html, abs_start) {
                    let tag_content = &html[abs_start + 1..tag_end];
                    if tag_content.starts_with("!--") {
                        let node = DomNode::Comment(tag_content.to_string());
                        if let Some((_, _, children)) = stack.last_mut() {
                            children.push(node);
                        } else {
                            root_nodes.push(node);
                        }
                    } else if let Some(closing_tag) = tag_content.strip_prefix('/') {
                        let tag_name = closing_tag.trim().to_lowercase();
                        if let Some(pop_idx) = stack
                            .iter()
                            .rposition(|(name, _, _)| name.to_lowercase() == tag_name)
                        {
                            while stack.len() > pop_idx {
                                let (name, attrs, children) = stack.pop().unwrap();
                                let node = DomNode::Element {
                                    tag_name: name,
                                    attributes: attrs,
                                    children,
                                };
                                if let Some((_, _, parent_children)) = stack.last_mut() {
                                    parent_children.push(node);
                                } else {
                                    root_nodes.push(node);
                                }
                            }
                        }
                    } else {
                        let is_self_closing = tag_content.trim_end().ends_with('/');
                        let (tag_name, attrs) = parse_tag_parts(tag_content);
                        let is_void = void_tags.contains(&tag_name.to_lowercase().as_str());

                        const MAX_DOM_DEPTH: usize = 256;
                        if is_self_closing || is_void || stack.len() >= MAX_DOM_DEPTH {
                            let node = DomNode::Element {
                                tag_name,
                                attributes: attrs,
                                children: Vec::new(),
                            };
                            if let Some((_, _, children)) = stack.last_mut() {
                                children.push(node);
                            } else {
                                root_nodes.push(node);
                            }
                        } else {
                            stack.push((tag_name, attrs, Vec::new()));
                        }
                    }
                    search_idx = tag_end + 1;
                } else {
                    let remainder = &html[abs_start..];
                    let node = DomNode::Text(remainder.to_string());
                    if let Some((_, _, children)) = stack.last_mut() {
                        children.push(node);
                    } else {
                        root_nodes.push(node);
                    }
                    break;
                }
            } else {
                let remainder = &html[search_idx..];
                if !remainder.is_empty() {
                    let node = DomNode::Text(remainder.to_string());
                    if let Some((_, _, children)) = stack.last_mut() {
                        children.push(node);
                    } else {
                        root_nodes.push(node);
                    }
                }
                break;
            }
        }

        while let Some((name, attrs, children)) = stack.pop() {
            let node = DomNode::Element {
                tag_name: name,
                attributes: attrs,
                children,
            };
            if let Some((_, _, parent_children)) = stack.last_mut() {
                parent_children.push(node);
            } else {
                root_nodes.push(node);
            }
        }

        Self { root_nodes }
    }

    /// Convert DOM AST tree back to HTML string.
    pub fn to_html(&self) -> String {
        let mut out = String::new();
        for node in &self.root_nodes {
            render_node(node, &mut out);
        }
        out
    }

    /// Find all elements matching a tag name.
    pub fn find_elements_by_tag(&self, tag: &str) -> Vec<&DomNode> {
        let mut matches = Vec::new();
        let tag_lower = tag.to_lowercase();
        for node in &self.root_nodes {
            collect_matching_nodes(node, &tag_lower, &mut matches);
        }
        matches
    }

    /// Strip forbidden elements (e.g., `<script>`, `<style>`, `<iframe>`) from AST tree.
    pub fn strip_elements(&mut self, tags_to_strip: &[&str]) {
        let strip_set: Vec<String> = tags_to_strip.iter().map(|s| s.to_lowercase()).collect();
        strip_nodes(&mut self.root_nodes, &strip_set);
    }

    /// Query all elements matching a CSS selector (`tag`, `#id`, `.class`, `[attr]`, `[attr=val]`,
    /// descendant and child combinators, comma grouping).
    pub fn query_selector_all(&self, selector: &str) -> Vec<&DomNode> {
        let mut result = Vec::new();
        let selectors = parse_selectors(selector);
        for node in &self.root_nodes {
            collect_selector_matches(node, &[], &selectors, &mut result);
        }
        result
    }

    /// Resolve the inline + `<style>`-block styles applicable to a node, returned as a
    /// property -> value map (later/higher-specificity declarations win).
    pub fn style_of(&self, node: &DomNode) -> AHashMap<CompactString, CompactString> {
        // Collect <style> block CSS from the tree.
        let mut css = String::new();
        collect_style_text(&self.root_nodes, &mut css);
        // Parse rules.
        let rules = parse_css_rules(&css);
        // Find ancestors of node.
        let ancestors = find_ancestors(&self.root_nodes, node).unwrap_or_default();
        // Collect matching rules with specificity.
        let mut applied: Vec<(Specificity, &CssDeclaration)> = Vec::new();
        for rule in &rules {
            for sel in &rule.selectors {
                if matches_selector_with_ancestors(node, &ancestors, sel) {
                    for decl in &rule.decls {
                        applied.push((rule.specificity, decl));
                    }
                }
            }
        }
        // Sort by specificity ascending so later wins.
        applied.sort_by_key(|(s, _)| *s);
        let mut out = AHashMap::new();
        for (_, decl) in &applied {
            out.insert(
                CompactString::new(&decl.prop),
                CompactString::new(&decl.val),
            );
        }
        // Inline style overrides everything.
        if let DomNode::Element { attributes, .. } = node {
            if let Some(inline) = attributes.get("style") {
                for part in inline.split(';') {
                    let part = part.trim();
                    if part.is_empty() {
                        continue;
                    }
                    if let Some((k, v)) = part.split_once(':') {
                        out.insert(CompactString::new(k.trim()), CompactString::new(v.trim()));
                    }
                }
            }
        }
        out
    }
}

fn strip_nodes(nodes: &mut Vec<DomNode>, strip_set: &[String]) {
    nodes.retain_mut(|node| match node {
        DomNode::Element {
            tag_name, children, ..
        } => {
            if strip_set.contains(&tag_name.to_lowercase().to_string()) {
                false
            } else {
                strip_nodes(children, strip_set);
                true
            }
        }
        _ => true,
    });
}

fn parse_tag_parts(content: &str) -> (CompactString, AHashMap<CompactString, CompactString>) {
    let trimmed = content.trim_end_matches('/').trim();
    let mut parts = trimmed.split_whitespace();
    let tag_name = CompactString::new(parts.next().unwrap_or(""));

    let mut attrs = AHashMap::new();
    let mut search_idx = tag_name.len();
    while search_idx < trimmed.len() {
        let rem = trimmed[search_idx..].trim_start();
        if rem.is_empty() {
            break;
        }
        let name_end = rem
            .find(|c: char| c.is_whitespace() || c == '=' || c == '/')
            .unwrap_or(rem.len());
        if name_end == 0 {
            search_idx = trimmed.len() - rem[1..].len();
            continue;
        }
        let key = &rem[..name_end];
        let after_name = rem[name_end..].trim_start();

        if let Some(after_eq) = after_name.strip_prefix('=') {
            let val_rem = after_eq.trim_start();
            if let Some(stripped) = val_rem.strip_prefix('"') {
                if let Some(end_q) = stripped.find('"') {
                    let val = &stripped[..end_q];
                    attrs.insert(CompactString::new(key), CompactString::new(val));
                    search_idx = trimmed.len() - stripped[end_q + 1..].len();
                } else {
                    attrs.insert(CompactString::new(key), CompactString::new(stripped));
                    break;
                }
            } else if let Some(stripped) = val_rem.strip_prefix('\'') {
                if let Some(end_q) = stripped.find('\'') {
                    let val = &stripped[..end_q];
                    attrs.insert(CompactString::new(key), CompactString::new(val));
                    search_idx = trimmed.len() - stripped[end_q + 1..].len();
                } else {
                    attrs.insert(CompactString::new(key), CompactString::new(stripped));
                    break;
                }
            } else {
                let val_end = val_rem
                    .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
                    .unwrap_or(val_rem.len());
                attrs.insert(
                    CompactString::new(key),
                    CompactString::new(&val_rem[..val_end]),
                );
                search_idx = trimmed.len() - val_rem[val_end..].len();
            }
        } else {
            attrs.insert(CompactString::new(key), CompactString::new(""));
            search_idx = trimmed.len() - rem[name_end..].len();
        }
    }

    (tag_name, attrs)
}

fn collect_matching_nodes<'a>(node: &'a DomNode, tag_lower: &str, matches: &mut Vec<&'a DomNode>) {
    if let DomNode::Element {
        tag_name, children, ..
    } = node
    {
        if tag_name.to_lowercase() == tag_lower {
            matches.push(node);
        }
        for child in children {
            collect_matching_nodes(child, tag_lower, matches);
        }
    }
}

/// Fast XML escaping for text nodes and attribute values.
pub fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Decode HTML character references that can appear in book text nodes.
///
/// The inverse of [`xml_escape`]. Several format parsers (FB2, PDF, ODT, DOCX, RTF, CBZ) build
/// their section HTML by escaping text nodes, so any consumer reading `plain_text` — which
/// [`crate::section::extract_plain_text`] derives by stripping markup from that HTML — sees the
/// literal entities unless they are decoded back.
///
/// Performs a **single** left-to-right pass, so `&amp;amp;` correctly decodes to `&amp;` rather
/// than looping. Numeric (decimal and hex) references are decoded to `char`, and references to
/// invalid or surrogate code points are left untouched.
///
/// Only the subset of the HTML5 named-reference table relevant to book prose is decoded.
/// Attribute-value-only references (`&colon;`, `&Tab;`, …) are deliberately *not* decoded here;
/// URI scheme probing uses its own narrower decoder.
pub fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input.as_bytes()[i] != b'&' {
            let start = i;
            while i < input.len() && input.as_bytes()[i] != b'&' {
                i += 1;
            }
            out.push_str(&input[start..i]);
            continue;
        }
        let rest = &input[i..];
        // A real reference is short; bail out on an unterminated or absurdly long one instead of
        // scanning the remainder of the document.
        let Some(semi) = rest.find(';').filter(|&p| p <= 32) else {
            out.push('&');
            i += 1;
            continue;
        };
        let name = &rest[1..semi];
        if let Some(decoded) = named_html_entity(name) {
            out.push_str(decoded);
            i += semi + 1;
            continue;
        }
        if let Some(decoded) = numeric_html_entity(name) {
            out.push_str(&decoded);
            i += semi + 1;
            continue;
        }
        out.push('&');
        i += 1;
    }
    out
}

/// Named references decoded by [`decode_html_entities`].
fn named_html_entity(name: &str) -> Option<&'static str> {
    Some(match name {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => "\u{a0}",
        "shy" => "\u{ad}",
        "copy" => "\u{a9}",
        "reg" => "\u{ae}",
        "trade" => "\u{2122}",
        "hellip" => "\u{2026}",
        "mdash" => "\u{2014}",
        "ndash" => "\u{2013}",
        "lsquo" => "\u{2018}",
        "rsquo" => "\u{2019}",
        "ldquo" => "\u{201c}",
        "rdquo" => "\u{201d}",
        "laquo" => "\u{ab}",
        "raquo" => "\u{bb}",
        "bull" => "\u{2022}",
        "dagger" => "\u{2020}",
        "middot" => "\u{b7}",
        "permil" => "\u{2030}",
        "prime" => "\u{2032}",
        "Prime" => "\u{2033}",
        "deg" => "\u{b0}",
        "plusmn" => "\u{b1}",
        "times" => "\u{d7}",
        "divide" => "\u{f7}",
        "frac12" => "\u{bd}",
        "sect" => "\u{a7}",
        "para" => "\u{b6}",
        "euro" => "\u{20ac}",
        "pound" => "\u{a3}",
        "yen" => "\u{a5}",
        "cent" => "\u{a2}",
        "iexcl" => "\u{a1}",
        "iquest" => "\u{bf}",
        "ensp" => "\u{2002}",
        "emsp" => "\u{2003}",
        "thinsp" => "\u{2009}",
        "zwnj" => "\u{200c}",
        "zwj" => "\u{200d}",
        "lrm" => "\u{200e}",
        "rlm" => "\u{200f}",
        _ => return None,
    })
}

/// Decode a numeric character reference (`#38` / `#x26`) to a `char`.
fn numeric_html_entity(name: &str) -> Option<String> {
    let digits = name.strip_prefix('#')?;
    let code = if let Some(hex) = digits
        .strip_prefix('x')
        .or_else(|| digits.strip_prefix('X'))
    {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        digits.parse::<u32>().ok()?
    };
    char::from_u32(code).map(|c| c.to_string())
}

fn render_node(node: &DomNode, out: &mut String) {
    match node {
        DomNode::Text(t) => out.push_str(t),
        DomNode::Comment(c) => {
            out.push_str("<!--");
            out.push_str(c.trim_start_matches("!--").trim_end_matches("--"));
            out.push_str("-->");
        }
        DomNode::Element {
            tag_name,
            attributes,
            children,
        } => {
            out.push('<');
            out.push_str(tag_name);
            for (k, v) in attributes {
                out.push(' ');
                out.push_str(k);
                if !v.is_empty() {
                    out.push_str("=\"");
                    out.push_str(v);
                    out.push('"');
                }
            }
            let tag_lower = tag_name.to_lowercase();
            let is_void = matches!(
                tag_lower.as_str(),
                "img"
                    | "br"
                    | "hr"
                    | "meta"
                    | "link"
                    | "input"
                    | "area"
                    | "base"
                    | "col"
                    | "embed"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            );
            if children.is_empty() && is_void {
                out.push_str(" />");
            } else if children.is_empty() {
                out.push('>');
                out.push_str("</");
                out.push_str(tag_name);
                out.push('>');
            } else {
                out.push('>');
                for child in children {
                    render_node(child, out);
                }
                out.push_str("</");
                out.push_str(tag_name);
                out.push('>');
            }
        }
    }
}

/// Fast SIMD/memchr accelerated XML / HTML recovery sanitizer that repairs unescaped ampersands (`&`), unclosed tags, and invalid entities.
pub fn sanitize_and_repair_xml(xml: &str) -> String {
    let bytes = xml.as_bytes();
    let mut out = String::with_capacity(xml.len() + 32);
    let mut search_idx = 0;

    while search_idx < bytes.len() {
        if let Some(amp_pos) = memchr::memchr(b'&', &bytes[search_idx..]) {
            let abs_amp = search_idx + amp_pos;
            if abs_amp > search_idx {
                out.push_str(&xml[search_idx..abs_amp]);
            }

            let rest = &xml[abs_amp..];
            if let Some(entity) = ["&amp;", "&lt;", "&gt;", "&quot;", "&apos;"]
                .iter()
                .find(|e| rest.starts_with(*e))
            {
                out.push_str(entity);
                search_idx = abs_amp + entity.len();
            } else if rest.starts_with("&#")
                && let Some(semi_pos) = rest.find(';').filter(|&pos| pos < 12)
            {
                out.push_str(&rest[..=semi_pos]);
                search_idx = abs_amp + semi_pos + 1;
            } else {
                out.push_str("&amp;");
                search_idx = abs_amp + 1;
            }
        } else {
            out.push_str(&xml[search_idx..]);
            break;
        }
    }

    out
}

/// Maximum element nesting depth accepted in any XML document.
///
/// `roxmltree` recurses while parsing nested elements and imposes no depth limit, so a document
/// nested a few hundred levels deep already exhausts a default thread stack. A stack overflow is a
/// `SIGSEGV`: `catch_unwind` cannot intercept it and it takes the whole process down. Because the
/// recursion is inside the parser, guarding our own walkers is not sufficient — the input must be
/// rejected *before* it reaches `roxmltree`.
pub const MAX_XML_NESTING_DEPTH: usize = 256;

/// Iteratively compute the maximum element nesting depth of an XML/HTML string.
///
/// Returns `Err` with the observed depth if it exceeds `max_depth`. This scan is deliberately
/// non-recursive so it is safe to run on adversarial input.
pub fn xml_nesting_depth(input: &str) -> usize {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    let mut depth = 0usize;
    let mut max_depth = 0usize;

    while i < len {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }

        // Skip over constructs that contain no elements.
        if bytes.get(i + 1) == Some(&b'!') {
            if input[i..].starts_with("<!--") {
                i = input[i..].find("-->").map_or(len, |p| i + p + 3);
            } else if input[i..].starts_with("<![CDATA[") {
                i = input[i..].find("]]>").map_or(len, |p| i + p + 3);
            } else {
                // DOCTYPE or other declaration: skip to the next `>`.
                i = input[i..].find('>').map_or(len, |p| i + p + 1);
            }
            continue;
        }
        if bytes.get(i + 1) == Some(&b'?') {
            i = input[i..].find("?>").map_or(len, |p| i + p + 2);
            continue;
        }

        let is_close = bytes.get(i + 1) == Some(&b'/');
        let name_start = if is_close { i + 2 } else { i + 1 };
        // `NameStartChar` in XML 1.0 includes non-ASCII letters, and every non-ASCII UTF-8 lead
        // byte is >= 0x80. Restricting this test to ASCII names made the entire guard skippable by
        // naming an element `<日本語>`: the tag would be skipped as "not a tag" instead of counted,
        // so a document nested thousands of levels deep measured as depth 1 and was handed to
        // `roxmltree` unchanged.
        if !matches!(bytes.get(name_start), Some(c) if c.is_ascii_alphabetic() || *c == b'_' || *c == b':' || *c >= 0x80)
        {
            i += 1;
            continue;
        }

        // Find the end of this tag, respecting quoted attribute values.
        let mut j = name_start;
        let mut quote: Option<u8> = None;
        let mut self_closing = false;
        while j < len {
            let c = bytes[j];
            match quote {
                Some(q) => {
                    if c == q {
                        quote = None;
                    }
                }
                None => match c {
                    b'"' | b'\'' => quote = Some(c),
                    b'>' => {
                        self_closing = j > 0 && bytes[j - 1] == b'/';
                        break;
                    }
                    _ => {}
                },
            }
            j += 1;
        }
        if j >= len {
            // Unterminated tag: nothing more to count.
            break;
        }

        if is_close {
            depth = depth.saturating_sub(1);
        } else if !self_closing {
            depth += 1;
            max_depth = max_depth.max(depth);
        }
        i = j + 1;
    }

    max_depth
}

/// Reject XML whose element nesting exceeds [`MAX_XML_NESTING_DEPTH`].
///
/// Must be called before `roxmltree` sees the input; see that constant for why.
pub fn check_xml_nesting_depth(input: &str) -> Result<(), String> {
    let depth = xml_nesting_depth(input);
    if depth > MAX_XML_NESTING_DEPTH {
        return Err(format!(
            "XML element nesting depth {} exceeds the safety limit of {}",
            depth, MAX_XML_NESTING_DEPTH
        ));
    }
    Ok(())
}

/// Parse XML with a node-count limit and a pre-scan nesting-depth guard.
///
/// This is the single entry point every roxmltree call site in this crate should use.
pub fn parse_xml_document(input: &str) -> Result<roxmltree::Document<'_>, String> {
    check_xml_nesting_depth(input)?;
    let opts = roxmltree::ParsingOptions {
        nodes_limit: MAX_XML_NODES,
        ..roxmltree::ParsingOptions::default()
    };
    roxmltree::Document::parse_with_options(input, opts).map_err(|e| e.to_string())
}

/// Maximum XML node count accepted in any document.
pub const MAX_XML_NODES: u32 = 4_000_000;

#[cfg(test)]
mod entity_tests {
    use super::*;

    #[test]
    fn decodes_common_entities() {
        assert_eq!(decode_html_entities("a &amp; b"), "a & b");
        assert_eq!(decode_html_entities("&lt;img&gt;"), "<img>");
        assert_eq!(decode_html_entities("&quot;q&quot;"), "\"q\"");
        assert_eq!(decode_html_entities("it&apos;s"), "it's");
        assert_eq!(decode_html_entities("&nbsp;"), "\u{a0}");
        assert_eq!(decode_html_entities("plain text"), "plain text");
    }

    /// A single left-to-right pass: `&amp;amp;` decodes to `&amp;`, not to `&`.
    #[test]
    fn decoding_is_not_recursive() {
        assert_eq!(decode_html_entities("&amp;amp;"), "&amp;");
        assert_eq!(decode_html_entities("&amp;lt;"), "&lt;");
    }

    #[test]
    fn decodes_numeric_references() {
        assert_eq!(decode_html_entities("&#65;"), "A");
        assert_eq!(decode_html_entities("&#x41;"), "A");
        assert_eq!(decode_html_entities("&#X41;"), "A");
        assert_eq!(decode_html_entities("&#233;"), "\u{e9}");
        assert_eq!(decode_html_entities("&#x2014;"), "\u{2014}");
    }

    /// Malformed references are left verbatim rather than dropped or panicking.
    #[test]
    fn malformed_references_are_left_alone() {
        assert_eq!(decode_html_entities("a & b"), "a & b");
        assert_eq!(decode_html_entities("5 &lt 6"), "5 &lt 6");
        assert_eq!(decode_html_entities("&unknown;"), "&unknown;");
        // No terminating semicolon within a sane distance.
        assert_eq!(
            decode_html_entities("&amp1234567890123456789012345;"),
            "&amp1234567890123456789012345;"
        );
        // Surrogate / out-of-range code points decode to nothing, not to a panic.
        assert_eq!(decode_html_entities("&#xD800;"), "&#xD800;");
        assert_eq!(decode_html_entities("&#1114112;"), "&#1114112;");
    }

    /// Attribute-value-only references must NOT be decoded in text nodes.
    #[test]
    fn attribute_only_references_are_not_decoded() {
        assert_eq!(
            decode_html_entities("javascript&colon;x"),
            "javascript&colon;x"
        );
    }

    /// Char-boundary safety: the scanner must never slice inside a multi-byte character.
    #[test]
    fn decoding_is_char_boundary_safe() {
        assert_eq!(decode_html_entities("日本語&amp;中文"), "日本語&中文");
        assert_eq!(decode_html_entities("&日本語;"), "&日本語;");
        assert_eq!(decode_html_entities("a&日b"), "a&日b");
    }
}

#[cfg(test)]
mod nesting_tests {
    use super::*;

    #[test]
    fn depth_scan_counts_nesting() {
        assert_eq!(xml_nesting_depth("<a><b><c/></b></a>"), 2);
        assert_eq!(xml_nesting_depth("<a/>"), 0);
        assert_eq!(xml_nesting_depth("plain text"), 0);
        // Comments/CDATA/PIs must not count as elements.
        // Comments/CDATA/PIs must not count as elements, and self-closing tags must not
        // increase the depth.
        assert_eq!(xml_nesting_depth("<!-- <a><b> --><c/>"), 0);
        assert_eq!(xml_nesting_depth("<a><![CDATA[<b><c>]]></a>"), 1);
        assert_eq!(xml_nesting_depth("<?xml version=\"1.0\"?><a/>"), 0);
        assert_eq!(xml_nesting_depth("<?xml version=\"1.0\"?><a><b/></a>"), 1);
        // `>` inside a quoted attribute must not terminate the tag early: if it did, the trailing
        // `">` would be treated as a bogus tag and `<b/>` would be miscounted.
        assert_eq!(xml_nesting_depth(r#"<a t=">"><b/></a>"#), 1);
        assert_eq!(xml_nesting_depth(r#"<a t=">"><b><c/></b></a>"#), 2);
    }

    #[test]
    fn guard_rejects_deep_documents() {
        let deep = format!("{}{}{}", "<a>".repeat(1000), "</a>".repeat(1000), "");
        assert!(check_xml_nesting_depth(&deep).is_err());
        let shallow = "<a><b><c/></b></a>";
        assert!(check_xml_nesting_depth(shallow).is_ok());
    }

    #[test]
    fn parsing_deep_document_does_not_overflow() {
        let depth = 20_000;
        let deep = format!(
            "<FictionBook>{}{}</FictionBook>",
            "<a>".repeat(depth),
            "</a>".repeat(depth)
        );
        // Must be rejected by the guard rather than overflowing the stack.
        assert!(parse_xml_document(&deep).is_err());
    }

    /// The name-start test in `xml_nesting_depth` must accept non-ASCII letters. XML 1.0
    /// `NameStartChar` includes them, and every non-ASCII UTF-8 lead byte is >= 0x80, so an
    /// element named `<日本語>` used to be skipped as "not a tag" instead of counted — making the
    /// guard skippable by naming a hostile element with one non-ASCII character.
    #[test]
    fn depth_scan_counts_non_ascii_element_names() {
        assert_eq!(xml_nesting_depth("<日本語><中文/></日本語>"), 1);
        // A hostile document built from non-ASCII names must be measured as deep, not shallow.
        let deep = format!(
            "<日本語>{}{}</日本語>",
            "<中文>".repeat(5000),
            "</中文>".repeat(5000)
        );
        assert!(
            xml_nesting_depth(&deep) > MAX_XML_NESTING_DEPTH,
            "non-ASCII nesting was not counted: {}",
            xml_nesting_depth(&deep)
        );
        assert!(check_xml_nesting_depth(&deep).is_err());
    }
}

// ---- CSS selector engine + style resolution (blocker #3) ----

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Specificity(pub u32, pub u32, pub u32);

#[derive(Clone, Debug)]
struct SimpleSel {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attrs: Vec<(String, Option<String>)>,
    universal: bool,
    pseudo: Vec<String>,
}

#[derive(Clone, Debug)]
struct Compound {
    simples: Vec<SimpleSel>,
}

#[derive(Clone, Debug)]
enum Combinator {
    Descendant,
    Child,
}

#[derive(Clone, Debug)]
struct Selector {
    // list of (combinator_before, compound); first has None combinator
    parts: Vec<(Option<Combinator>, Compound)>,
}

#[derive(Clone, Debug)]
struct CssDeclaration {
    prop: String,
    val: String,
}

#[derive(Clone, Debug)]
struct CssRule {
    selectors: Vec<Selector>,
    decls: Vec<CssDeclaration>,
    specificity: Specificity,
}

fn parse_simple(s: &str) -> SimpleSel {
    let s = s.trim();
    let mut sel = SimpleSel {
        tag: None,
        id: None,
        classes: vec![],
        attrs: vec![],
        universal: false,
        pseudo: vec![],
    };
    let mut i = 0;
    // tag name at start (if not starting with # . [ * :)
    let mut buf = String::new();
    // parse leading tag
    if !s.is_empty() && !matches!(s.chars().next().unwrap(), '#' | '.' | '[' | '*' | ':') {
        for c in s.chars() {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                buf.push(c);
                i += c.len_utf8();
            } else {
                break;
            }
        }
        if !buf.is_empty() {
            sel.tag = Some(buf.to_lowercase());
            buf.clear();
        }
    }
    // universal
    if s.starts_with('*') {
        sel.universal = true;
        i = 1;
    }
    // parse rest
    let rest = &s[i..];
    let mut j = 0;
    let chars: Vec<char> = rest.chars().collect();
    while j < chars.len() {
        match chars[j] {
            '#' => {
                j += 1;
                let mut name = String::new();
                while j < chars.len()
                    && (chars[j].is_alphanumeric() || chars[j] == '-' || chars[j] == '_')
                {
                    name.push(chars[j]);
                    j += 1;
                }
                sel.id = Some(name);
            }
            '.' => {
                j += 1;
                let mut name = String::new();
                while j < chars.len()
                    && (chars[j].is_alphanumeric() || chars[j] == '-' || chars[j] == '_')
                {
                    name.push(chars[j]);
                    j += 1;
                }
                sel.classes.push(name);
            }
            '[' => {
                // find closing ]
                let mut depth = 0;
                let mut k = j;
                while k < chars.len() {
                    if chars[k] == '[' {
                        depth += 1;
                    }
                    if chars[k] == ']' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    k += 1;
                }
                let content: String = chars[j + 1..k].iter().collect();
                j = k + 1;
                // parse attr and optional = value
                let content = content.trim();
                if let Some(eq) = content.find('=') {
                    let attr = content[..eq].trim().to_lowercase();
                    let mut val = content[eq + 1..].trim();
                    val = val.trim_matches('"').trim_matches('\'');
                    sel.attrs.push((attr, Some(val.to_string())));
                } else {
                    sel.attrs.push((content.to_lowercase(), None));
                }
            }
            ':' => {
                // pseudo-class: read name and optional (...)
                j += 1;
                let mut name = String::new();
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '-') {
                    name.push(chars[j]);
                    j += 1;
                }
                if j < chars.len() && chars[j] == '(' {
                    // skip until )
                    while j < chars.len() && chars[j] != ')' {
                        j += 1;
                    }
                    j += 1;
                }
                sel.pseudo.push(name);
            }
            _ => {
                j += 1;
            }
        }
    }
    sel
}

fn parse_compound(s: &str) -> Compound {
    // split into simple selectors: tag#id.class[attr]:pseudo chained (no spaces)
    // A compound is a sequence of simple selectors with no combinator; but our SimpleSel
    // already captures tag+id+class+attr+pseudo as one compound. Multiple simples chained
    // like "div.foo" are ONE simple sel. Combinators split compounds.
    // So a compound string with no spaces/ >/+ /~ is a single SimpleSel.
    Compound {
        simples: vec![parse_simple(s.trim())],
    }
}

fn parse_selectors(selector: &str) -> Vec<Selector> {
    let mut out = Vec::new();
    for group in selector.split(',') {
        let group = group.trim();
        if group.is_empty() {
            continue;
        }
        // tokenize combinators: split by whitespace and >
        // We'll parse by scanning
        let mut parts: Vec<(Option<Combinator>, Compound)> = Vec::new();
        // normalize: insert spaces around >
        let normalized = group.replace('>', " > ");
        let tokens: Vec<&str> = normalized.split_whitespace().collect();
        // tokens may be: compound, ">", compound, compound (descendant)...
        // Reconstruct: a combinator appears as ">" token
        let mut pending_comb: Option<Combinator> = None;
        // first part
        let mut first = true;
        let mut buf: Vec<&str> = Vec::new();
        // We'll iterate tokens; when we hit ">", flush buf as a compound with pending comb
        for tok in &tokens {
            if *tok == ">" {
                if !buf.is_empty() {
                    let compound_str = buf.join("");
                    let c = parse_compound(&compound_str);
                    // Actually for first, comb None; for subsequent without explicit >, descendant
                    let comb = if first {
                        None
                    } else if pending_comb.is_some() {
                        pending_comb.take()
                    } else {
                        Some(Combinator::Descendant)
                    };
                    parts.push((comb, c));
                    first = false;
                    buf.clear();
                }
                pending_comb = Some(Combinator::Child);
            } else {
                buf.push(tok);
            }
        }
        if !buf.is_empty() {
            let compound_str = buf.join("");
            let c = parse_compound(&compound_str);
            let comb = if first {
                None
            } else if pending_comb.is_some() {
                pending_comb.take()
            } else {
                Some(Combinator::Descendant)
            };
            parts.push((comb, c));
        }
        if !parts.is_empty() {
            out.push(Selector { parts });
        }
    }
    out
}

fn simple_matches(node: &DomNode, simple: &SimpleSel) -> bool {
    let DomNode::Element {
        tag_name,
        attributes,
        ..
    } = node
    else {
        return false;
    };
    if simple.universal {
    } else if let Some(t) = &simple.tag {
        if tag_name.to_lowercase() != *t {
            return false;
        }
    }
    if let Some(id) = &simple.id {
        match attributes.get("id") {
            Some(v) if v == id => {}
            _ => return false,
        }
    }
    for cls in &simple.classes {
        let class_attr = attributes.get("class").cloned().unwrap_or_default();
        let classes: Vec<&str> = class_attr.split_whitespace().collect();
        if !classes.contains(&cls.as_str()) {
            return false;
        }
    }
    for (attr, val) in &simple.attrs {
        match attributes.get(attr.as_str()) {
            Some(v) => {
                if let Some(expected) = val {
                    if v != expected {
                        return false;
                    }
                }
            }
            None => return false,
        }
    }
    for p in &simple.pseudo {
        // basic pseudo-classes handled in caller context; here just accept :first-child etc. as no-op-ish
        let _ = p;
    }
    true
}

fn compound_matches(node: &DomNode, compound: &Compound) -> bool {
    compound.simples.iter().all(|s| simple_matches(node, s))
}

fn matches_selector_with_ancestors(
    node: &DomNode,
    ancestors: &[&DomNode],
    selector: &Selector,
) -> bool {
    // parts: last must match node; walk backwards
    let n = selector.parts.len();
    if n == 0 {
        return false;
    }
    // last part
    let (_, last_compound) = &selector.parts[n - 1];
    if !compound_matches(node, last_compound) {
        return false;
    }
    if n == 1 {
        return true;
    }
    // walk previous parts
    let mut current_ancestors = ancestors;
    for idx in (0..n - 1).rev() {
        let (comb, compound) = &selector.parts[idx];
        let is_child = matches!(comb, Some(Combinator::Child));
        // find an ancestor matching compound
        if is_child {
            // immediate parent
            if current_ancestors.is_empty() {
                return false;
            }
            let parent = current_ancestors[current_ancestors.len() - 1];
            if !compound_matches(parent, compound) {
                return false;
            }
            // move up one
            current_ancestors = &current_ancestors[..current_ancestors.len().saturating_sub(1)];
        } else {
            // descendant: any ancestor
            let mut found = false;
            for i in (0..current_ancestors.len()).rev() {
                if compound_matches(current_ancestors[i], compound) {
                    current_ancestors = &current_ancestors[..i];
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
    }
    true
}

fn collect_selector_matches<'a>(
    node: &'a DomNode,
    ancestors: &[&'a DomNode],
    selectors: &[Selector],
    out: &mut Vec<&'a DomNode>,
) {
    let mut new_anc = ancestors.to_vec();
    new_anc.push(node);
    if let DomNode::Element { children, .. } = node {
        // test this node
        for sel in selectors {
            if matches_selector_with_ancestors(node, ancestors, sel) {
                out.push(node);
                break;
            }
        }
        for c in children {
            collect_selector_matches(c, &new_anc, selectors, out);
        }
    }
}

fn collect_style_text(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        if let DomNode::Element {
            tag_name, children, ..
        } = node
        {
            if tag_name.eq_ignore_ascii_case("style") {
                // gather text children
                for c in children {
                    if let DomNode::Text(t) = c {
                        out.push_str(t);
                        out.push('\n');
                    }
                }
            } else {
                collect_style_text(children, out);
            }
        }
    }
}

fn find_ancestors<'a>(nodes: &'a [DomNode], target: &'a DomNode) -> Option<Vec<&'a DomNode>> {
    // returns ancestors (excluding target) from root
    fn walk<'a>(nodes: &'a [DomNode], target: *const DomNode, path: &mut Vec<&'a DomNode>) -> bool {
        for node in nodes {
            if std::ptr::eq(node as *const DomNode, target) {
                return true;
            }
            if let DomNode::Element { children, .. } = node {
                path.push(node);
                if walk(children, target, path) {
                    return true;
                }
                path.pop();
            }
        }
        false
    }
    let mut path = Vec::new();
    if walk(nodes, target as *const DomNode, &mut path) {
        Some(path)
    } else {
        None
    }
}

fn parse_css_rules(css: &str) -> Vec<CssRule> {
    let mut rules = Vec::new();
    // strip comments
    let css = strip_css_comments(css);
    // split into rules by }
    let mut depth = 0;
    let mut buf = String::new();
    let mut chunks: Vec<String> = Vec::new();
    for c in css.chars() {
        buf.push(c);
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                chunks.push(buf.clone());
                buf.clear();
            }
        }
    }
    for chunk in chunks {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        // split selector { decls }
        let Some(brace) = chunk.find('{') else {
            continue;
        };
        let sel_part = chunk[..brace].trim();
        let decl_part = chunk[brace + 1..].trim().trim_end_matches('}').trim();
        if sel_part.is_empty() {
            continue;
        }
        let selectors = parse_selectors(sel_part);
        // specificity: sum of first selector (approx)
        let spec = selectors
            .first()
            .map(selector_specificity)
            .unwrap_or(Specificity(0, 0, 0));
        let mut decls = Vec::new();
        for d in decl_part.split(';') {
            let d = d.trim();
            if d.is_empty() {
                continue;
            }
            if let Some((k, v)) = d.split_once(':') {
                decls.push(CssDeclaration {
                    prop: k.trim().to_string(),
                    val: v.trim().to_string(),
                });
            }
        }
        if decls.is_empty() {
            continue;
        }
        rules.push(CssRule {
            selectors,
            decls,
            specificity: spec,
        });
    }
    rules
}

fn selector_specificity(sel: &Selector) -> Specificity {
    let mut a = 0u32;
    let mut b = 0u32;
    let mut c = 0u32;
    for (_, compound) in &sel.parts {
        for simple in &compound.simples {
            if simple.id.is_some() {
                a += 1;
            }
            b += simple.classes.len() as u32
                + simple.attrs.len() as u32
                + simple.pseudo.len() as u32;
            if simple.tag.is_some() {
                c += 1;
            }
            if simple.universal { /* no specificity */ }
        }
    }
    Specificity(a, b, c)
}

fn strip_css_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // find */
            let mut j = i + 2;
            while j + 1 < bytes.len() && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
                j += 1;
            }
            i = j + 2;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}
