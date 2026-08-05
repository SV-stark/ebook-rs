use crate::archive::resolve_relative_path;
use roxmltree::Document;
use serde::{Deserialize, Serialize};

/// A node in the Table of Contents tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavPoint {
    pub id: String,
    pub label: String,
    pub href: String,
    pub full_path: String,
    pub subitems: Vec<NavPoint>,
}

/// EPUB 3 Landmark reference (e.g. cover, titlepage, toc, bodymatter).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Landmark {
    pub epub_type: String,
    pub label: String,
    pub href: String,
}

/// EPUB 3 Page List item (mapping physical print page numbers to EPUB target hrefs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageListItem {
    pub page: String,
    pub label: String,
    pub href: String,
}

/// Parse EPUB 2 NCX document (`toc.ncx`).
pub fn parse_ncx(xml_content: &str, ncx_path: &str) -> Result<Vec<NavPoint>, String> {
    let doc = Document::parse(xml_content).map_err(|e| format!("NCX XML parse error: {}", e))?;
    let ncx_dir = if let Some(idx) = ncx_path.rfind('/') {
        &ncx_path[..idx]
    } else {
        ""
    };

    let mut points = Vec::new();

    for node in doc.descendants() {
        if node.has_tag_name("navMap") {
            for child in node.children() {
                if child.has_tag_name("navPoint") {
                    if let Some(nav_pt) = parse_ncx_navpoint(&child, ncx_dir) {
                        points.push(nav_pt);
                    }
                }
            }
        }
    }

    Ok(points)
}

fn parse_ncx_navpoint(node: &roxmltree::Node, base_dir: &str) -> Option<NavPoint> {
    let id = node.attribute("id").unwrap_or("").to_string();
    let mut label = String::new();
    let mut href = String::new();
    let mut subitems = Vec::new();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "navLabel" => {
                for label_child in child.children() {
                    if label_child.has_tag_name("text") {
                        label = label_child.text().unwrap_or("").trim().to_string();
                    }
                }
            }
            "content" => {
                if let Some(src) = child.attribute("src") {
                    href = src.to_string();
                }
            }
            "navPoint" => {
                if let Some(sub) = parse_ncx_navpoint(&child, base_dir) {
                    subitems.push(sub);
                }
            }
            _ => {}
        }
    }

    if href.is_empty() {
        return None;
    }

    let full_path = resolve_relative_path(base_dir, &href);

    Some(NavPoint {
        id,
        label: if label.is_empty() {
            "Untitled".to_string()
        } else {
            label
        },
        href,
        full_path,
        subitems,
    })
}

/// Parse EPUB 3 Navigation Document (`nav.xhtml`).
pub fn parse_nav_xhtml(html_content: &str, nav_path: &str) -> Result<Vec<NavPoint>, String> {
    let doc = Document::parse(html_content).map_err(|e| format!("NAV XHTML parse error: {}", e))?;
    let nav_dir = if let Some(idx) = nav_path.rfind('/') {
        &nav_path[..idx]
    } else {
        ""
    };

    let mut points = Vec::new();

    for node in doc.descendants() {
        if node.has_tag_name("nav") {
            // B4 Fix: Strict TOC nav element matching without points.is_empty() fallback
            let is_toc = get_attr_val(&node, "type")
                .map(|t| t.contains("toc"))
                .unwrap_or(false)
                || node.attribute("id").map(|i| i == "toc").unwrap_or(false);

            if is_toc {
                for child in node.children() {
                    if child.has_tag_name("ol") || child.has_tag_name("ul") {
                        points = parse_nav_list(&child, nav_dir);
                        if !points.is_empty() {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(points)
}

/// Parse EPUB 3 Landmarks navigation (`<nav epub:type="landmarks">`).
pub fn parse_landmarks(html_content: &str) -> Vec<Landmark> {
    let mut list = Vec::new();
    if let Ok(doc) = Document::parse(html_content) {
        for node in doc.descendants() {
            if node.has_tag_name("nav") {
                let is_landmarks = get_attr_val(&node, "type")
                    .map(|t| t.contains("landmarks"))
                    .unwrap_or(false);

                if is_landmarks {
                    for a in node.descendants() {
                        if a.has_tag_name("a") {
                            let epub_type = get_attr_val(&a, "type").unwrap_or_default();
                            let href = a.attribute("href").unwrap_or("").to_string();
                            let label = a.text().unwrap_or("").trim().to_string();
                            if !href.is_empty() {
                                list.push(Landmark {
                                    epub_type,
                                    label,
                                    href,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    list
}

/// Parse EPUB 3 Page List navigation (`<nav epub:type="page-list">`).
pub fn parse_page_list(html_content: &str) -> Vec<PageListItem> {
    let mut list = Vec::new();
    if let Ok(doc) = Document::parse(html_content) {
        for node in doc.descendants() {
            if node.has_tag_name("nav") {
                let is_pagelist = get_attr_val(&node, "type")
                    .map(|t| t.contains("page-list"))
                    .unwrap_or(false);

                if is_pagelist {
                    for a in node.descendants() {
                        if a.has_tag_name("a") {
                            let href = a.attribute("href").unwrap_or("").to_string();
                            let label = a.text().unwrap_or("").trim().to_string();
                            if !href.is_empty() {
                                list.push(PageListItem {
                                    page: label.clone(),
                                    label,
                                    href,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    list
}

fn get_attr_val(node: &roxmltree::Node, name: &str) -> Option<String> {
    for attr in node.attributes() {
        if attr.name() == name || attr.name().ends_with(&format!(":{}", name)) {
            return Some(attr.value().to_string());
        }
    }
    None
}

fn parse_nav_list(ol_node: &roxmltree::Node, base_dir: &str) -> Vec<NavPoint> {
    let mut points = Vec::new();

    for li in ol_node.children() {
        if li.has_tag_name("li") {
            let mut label = String::new();
            let mut href = String::new();
            let mut id = String::new();
            let mut subitems = Vec::new();

            for child in li.children() {
                if !child.is_element() {
                    continue;
                }
                match child.tag_name().name() {
                    "a" => {
                        if let Some(h) = child.attribute("href") {
                            href = h.to_string();
                        }
                        if let Some(i) = child.attribute("id") {
                            id = i.to_string();
                        }
                        label = child.text().unwrap_or("").trim().to_string();
                        if label.is_empty() {
                            let mut texts = Vec::new();
                            for sub in child.descendants() {
                                if let Some(t) = sub.text() {
                                    if !t.trim().is_empty() {
                                        texts.push(t.trim());
                                    }
                                }
                            }
                            label = texts.join(" ");
                        }
                    }
                    "span" => {
                        if label.is_empty() {
                            label = child.text().unwrap_or("").trim().to_string();
                        }
                    }
                    "ol" | "ul" => {
                        subitems = parse_nav_list(&child, base_dir);
                    }
                    _ => {}
                }
            }

            if !href.is_empty() {
                let full_path = resolve_relative_path(base_dir, &href);
                points.push(NavPoint {
                    id,
                    label: if label.is_empty() {
                        "Untitled".to_string()
                    } else {
                        label
                    },
                    href,
                    full_path,
                    subitems,
                });
            }
        }
    }

    points
}
