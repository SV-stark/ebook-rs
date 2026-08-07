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

                        if is_self_closing || is_void {
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
        let rem = &trimmed[search_idx..].trim_start();
        if rem.is_empty() {
            break;
        }
        if let Some(eq_idx) = rem.find('=') {
            let key = CompactString::new(rem[..eq_idx].trim());
            let val_rem = rem[eq_idx + 1..].trim_start();
            if let Some(stripped) = val_rem.strip_prefix('"') {
                if let Some(end_q) = stripped.find('"') {
                    let val = &stripped[..end_q];
                    attrs.insert(key, CompactString::new(val));
                    search_idx = trimmed.len() - stripped[end_q + 1..].len();
                } else {
                    break;
                }
            } else if let Some(stripped) = val_rem.strip_prefix('\'') {
                if let Some(end_q) = stripped.find('\'') {
                    let val = &stripped[..end_q];
                    attrs.insert(key, CompactString::new(val));
                    search_idx = trimmed.len() - stripped[end_q + 1..].len();
                } else {
                    break;
                }
            } else {
                let val_end = val_rem.find(char::is_whitespace).unwrap_or(val_rem.len());
                attrs.insert(key, CompactString::new(&val_rem[..val_end]));
                search_idx = trimmed.len() - val_rem[val_end..].len();
            }
        } else {
            let key = rem.split_whitespace().next().unwrap_or("");
            if !key.is_empty() {
                attrs.insert(CompactString::new(key), CompactString::new(""));
            }
            break;
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

fn render_node(node: &DomNode, out: &mut String) {
    match node {
        DomNode::Text(t) => out.push_str(t),
        DomNode::Comment(c) => {
            out.push('<');
            out.push_str(c);
            out.push('>');
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
            if children.is_empty() {
                out.push_str(" />");
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

/// Lenient XML / HTML recovery sanitizer that repairs unescaped ampersands (`&`), unclosed tags, and invalid entities.
pub fn sanitize_and_repair_xml(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len() + 32);
    let mut i = 0;

    while i < xml.len() {
        if xml.as_bytes()[i] == b'&' {
            let rest = &xml[i..];
            if let Some(entity) = ["&amp;", "&lt;", "&gt;", "&quot;", "&apos;"]
                .iter()
                .find(|e| rest.starts_with(*e))
            {
                out.push_str(entity);
                i += entity.len();
            } else if rest.starts_with("&#") && rest.find(';').map(|pos| pos < 12).unwrap_or(false)
            {
                let semi_pos = rest.find(';').unwrap();
                out.push_str(&rest[..=semi_pos]);
                i += semi_pos + 1;
            } else {
                out.push_str("&amp;");
                i += 1;
            }
        } else if let Some(ch) = xml[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    out
}
