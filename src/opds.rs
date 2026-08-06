use serde::{Deserialize, Serialize};

/// OPDS Link entry (download link, acquisition link, image thumbnail link).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsLink {
    pub rel: String,
    pub href: String,
    pub media_type: String,
    pub title: Option<String>,
}

/// OPDS Book/Catalog Entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsEntry {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
    pub links: Vec<OpdsLink>,
}

impl OpdsEntry {
    /// Find acquisition download link for EPUB or CBZ format.
    pub fn download_link(&self, target_mime: Option<&str>) -> Option<&OpdsLink> {
        let mime = target_mime.unwrap_or("application/epub+zip");
        self.links
            .iter()
            .find(|link| link.media_type == mime || link.rel.contains("acquisition"))
    }
}

/// OPDS 1.2 / 2.0 Library Catalog Feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsFeed {
    pub id: String,
    pub title: String,
    pub updated: Option<String>,
    pub icon: Option<String>,
    pub links: Vec<OpdsLink>,
    pub entries: Vec<OpdsEntry>,
}

impl OpdsFeed {
    /// Parse OPDS 1.2 Atom XML Feed.
    pub fn parse_atom_xml(xml: &str) -> Result<Self, String> {
        let doc = roxmltree::Document::parse(xml)
            .map_err(|e| format!("Failed to parse OPDS Atom XML feed: {}", e))?;

        let root = doc.root_element();
        let feed_title = root
            .children()
            .find(|n| n.has_tag_name("title"))
            .map(|n| n.text().unwrap_or("").to_string())
            .unwrap_or_else(|| "OPDS Catalog".to_string());

        let feed_id = root
            .children()
            .find(|n| n.has_tag_name("id"))
            .map(|n| n.text().unwrap_or("").to_string())
            .unwrap_or_else(|| "urn:opds:feed".to_string());

        let updated = root
            .children()
            .find(|n| n.has_tag_name("updated"))
            .map(|n| n.text().unwrap_or("").to_string());

        let icon = root
            .children()
            .find(|n| n.has_tag_name("icon") || n.has_tag_name("logo"))
            .map(|n| n.text().unwrap_or("").to_string());

        let feed_links = parse_xml_links(&root);
        let mut entries = Vec::new();

        for entry_node in root.children().filter(|n| n.has_tag_name("entry")) {
            let id = entry_node
                .children()
                .find(|n| n.has_tag_name("id"))
                .map(|n| n.text().unwrap_or("").to_string())
                .unwrap_or_default();

            let title = entry_node
                .children()
                .find(|n| n.has_tag_name("title"))
                .map(|n| n.text().unwrap_or("").to_string())
                .unwrap_or_else(|| "Untitled".to_string());

            let mut authors = Vec::new();
            for author_node in entry_node.children().filter(|n| n.has_tag_name("author")) {
                if let Some(name_node) = author_node.children().find(|n| n.has_tag_name("name")) {
                    if let Some(text) = name_node.text() {
                        authors.push(text.to_string());
                    }
                }
            }

            let summary = entry_node
                .children()
                .find(|n| n.has_tag_name("summary"))
                .map(|n| n.text().unwrap_or("").to_string());

            let content = entry_node
                .children()
                .find(|n| n.has_tag_name("content"))
                .map(|n| n.text().unwrap_or("").to_string());

            let published = entry_node
                .children()
                .find(|n| n.has_tag_name("published"))
                .map(|n| n.text().unwrap_or("").to_string());

            let entry_updated = entry_node
                .children()
                .find(|n| n.has_tag_name("updated"))
                .map(|n| n.text().unwrap_or("").to_string());

            let links = parse_xml_links(&entry_node);

            entries.push(OpdsEntry {
                id,
                title,
                authors,
                summary,
                content,
                published,
                updated: entry_updated,
                links,
            });
        }

        Ok(Self {
            id: feed_id,
            title: feed_title,
            updated,
            icon,
            links: feed_links,
            entries,
        })
    }

    /// Parse OPDS 2.0 JSON Feed.
    pub fn parse_json(json_str: &str) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse OPDS 2.0 JSON: {}", e))?;

        let title = v["metadata"]["title"]
            .as_str()
            .unwrap_or("OPDS JSON Catalog")
            .to_string();

        let id = v["metadata"]["id"]
            .as_str()
            .unwrap_or("urn:opds:json:feed")
            .to_string();

        let updated = v["metadata"]["updated"].as_str().map(|s| s.to_string());

        let mut feed_links = Vec::new();
        if let Some(links_arr) = v["links"].as_array() {
            for l in links_arr {
                if let (Some(href), Some(type_val)) = (l["href"].as_str(), l["type"].as_str()) {
                    feed_links.push(OpdsLink {
                        rel: l["rel"].as_str().unwrap_or("").to_string(),
                        href: href.to_string(),
                        media_type: type_val.to_string(),
                        title: l["title"].as_str().map(|s| s.to_string()),
                    });
                }
            }
        }

        let mut entries = Vec::new();
        if let Some(pub_arr) = v["publications"].as_array() {
            for pub_item in pub_arr {
                let entry_title = pub_item["metadata"]["title"]
                    .as_str()
                    .unwrap_or("Untitled")
                    .to_string();

                let entry_id = pub_item["metadata"]["identifier"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let mut authors = Vec::new();
                if let Some(author_val) = pub_item["metadata"]["author"].as_array() {
                    for a in author_val {
                        if let Some(name) = a["name"].as_str() {
                            authors.push(name.to_string());
                        }
                    }
                } else if let Some(name) = pub_item["metadata"]["author"]["name"].as_str() {
                    authors.push(name.to_string());
                }

                let summary = pub_item["metadata"]["description"]
                    .as_str()
                    .map(|s| s.to_string());

                let mut entry_links = Vec::new();
                if let Some(links_arr) = pub_item["links"].as_array() {
                    for l in links_arr {
                        if let (Some(href), Some(type_val)) =
                            (l["href"].as_str(), l["type"].as_str())
                        {
                            entry_links.push(OpdsLink {
                                rel: l["rel"].as_str().unwrap_or("acquisition").to_string(),
                                href: href.to_string(),
                                media_type: type_val.to_string(),
                                title: l["title"].as_str().map(|s| s.to_string()),
                            });
                        }
                    }
                }

                entries.push(OpdsEntry {
                    id: entry_id,
                    title: entry_title,
                    authors,
                    summary,
                    content: None,
                    published: pub_item["metadata"]["published"]
                        .as_str()
                        .map(|s| s.to_string()),
                    updated: pub_item["metadata"]["modified"]
                        .as_str()
                        .map(|s| s.to_string()),
                    links: entry_links,
                });
            }
        }

        Ok(Self {
            id,
            title,
            updated,
            icon: None,
            links: feed_links,
            entries,
        })
    }
}

fn parse_xml_links<'a>(node: &roxmltree::Node<'a, 'a>) -> Vec<OpdsLink> {
    let mut links = Vec::new();
    for link_node in node.children().filter(|n| n.has_tag_name("link")) {
        let rel = link_node.attribute("rel").unwrap_or("").to_string();
        let href = link_node.attribute("href").unwrap_or("").to_string();
        let media_type = link_node.attribute("type").unwrap_or("").to_string();
        let title = link_node.attribute("title").map(|s| s.to_string());

        if !href.is_empty() {
            links.push(OpdsLink {
                rel,
                href,
                media_type,
                title,
            });
        }
    }
    links
}
