use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A node in the lightweight Ebook DOM AST tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DomNode {
    Element {
        tag_name: String,
        attributes: HashMap<String, String>,
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
        let mut root_nodes = Vec::new();
        let mut search_idx = 0;
        let bytes = html.as_bytes();

        while search_idx < bytes.len() {
            if let Some(tag_start) = html[search_idx..].find('<') {
                let abs_start = search_idx + tag_start;
                if abs_start > search_idx {
                    let text = &html[search_idx..abs_start];
                    if !text.is_empty() {
                        root_nodes.push(DomNode::Text(text.to_string()));
                    }
                }

                if let Some(tag_end) = crate::section::find_tag_end(html, abs_start) {
                    let tag_content = &html[abs_start + 1..tag_end];
                    if tag_content.starts_with("!--") {
                        root_nodes.push(DomNode::Comment(tag_content.to_string()));
                    } else if !tag_content.starts_with('/') {
                        let (tag_name, attrs) = parse_tag_parts(tag_content);
                        root_nodes.push(DomNode::Element {
                            tag_name,
                            attributes: attrs,
                            children: Vec::new(),
                        });
                    }
                    search_idx = tag_end + 1;
                } else {
                    let remainder = &html[abs_start..];
                    root_nodes.push(DomNode::Text(remainder.to_string()));
                    break;
                }
            } else {
                let remainder = &html[search_idx..];
                if !remainder.is_empty() {
                    root_nodes.push(DomNode::Text(remainder.to_string()));
                }
                break;
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
        self.root_nodes.retain(|node| match node {
            DomNode::Element { tag_name, .. } => !strip_set.contains(&tag_name.to_lowercase()),
            _ => true,
        });
    }
}

fn parse_tag_parts(content: &str) -> (String, HashMap<String, String>) {
    let trimmed = content.trim_end_matches('/').trim();
    let mut parts = trimmed.split_whitespace();
    let tag_name = parts.next().unwrap_or("").to_string();

    let mut attrs = HashMap::new();
    let mut search_idx = tag_name.len();
    while search_idx < trimmed.len() {
        let rem = &trimmed[search_idx..].trim_start();
        if rem.is_empty() {
            break;
        }
        if let Some(eq_idx) = rem.find('=') {
            let key = rem[..eq_idx].trim().to_string();
            let val_rem = rem[eq_idx + 1..].trim_start();
            if let Some(stripped) = val_rem.strip_prefix('"') {
                if let Some(end_q) = stripped.find('"') {
                    let val = &stripped[..end_q];
                    attrs.insert(key, val.to_string());
                    search_idx = trimmed.len() - stripped[end_q + 1..].len();
                } else {
                    break;
                }
            } else if let Some(stripped) = val_rem.strip_prefix('\'') {
                if let Some(end_q) = stripped.find('\'') {
                    let val = &stripped[..end_q];
                    attrs.insert(key, val.to_string());
                    search_idx = trimmed.len() - stripped[end_q + 1..].len();
                } else {
                    break;
                }
            } else {
                let val_end = val_rem.find(char::is_whitespace).unwrap_or(val_rem.len());
                attrs.insert(key, val_rem[..val_end].to_string());
                search_idx = trimmed.len() - val_rem[val_end..].len();
            }
        } else {
            let key = rem.split_whitespace().next().unwrap_or("");
            if !key.is_empty() {
                attrs.insert(key.to_string(), String::new());
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
    let bytes = xml.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'&' {
            let rest = &xml[i..];
            if rest.starts_with("&amp;")
                || rest.starts_with("&lt;")
                || rest.starts_with("&gt;")
                || rest.starts_with("&quot;")
                || rest.starts_with("&apos;")
                || (rest.starts_with("&#") && rest.find(';').map(|pos| pos < 12).unwrap_or(false))
            {
                out.push('&');
            } else {
                out.push_str("&amp;");
            }
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }

    out
}
