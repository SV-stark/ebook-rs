use crate::book::Book;
use serde::{Deserialize, Serialize};

/// Readium Webpub Manifest (application/webpub+json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebpubManifest {
    #[serde(rename = "@context")]
    pub context: String,
    pub metadata: WebpubMetadata,
    pub links: Vec<WebpubLink>,
    #[serde(rename = "readingOrder")]
    pub reading_order: Vec<WebpubLink>,
    pub resources: Vec<WebpubLink>,
    pub toc: Vec<WebpubLink>,
}

/// Readium Webpub Metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebpubMetadata {
    #[serde(rename = "@type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub identifier: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub author: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(rename = "readingProgression")]
    pub reading_progression: String,
}

/// Readium Webpub Link item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebpubLink {
    pub href: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rel: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<WebpubLink>,
}

impl Book {
    /// Export the book as a Readium Webpub JSON Manifest (application/webpub+json).
    pub fn to_webpub_manifest(&self) -> WebpubManifest {
        let reading_progression = match self.metadata().direction {
            crate::metadata::PageProgressionDirection::Rtl => "rtl",
            _ => "ltr",
        };

        let metadata = WebpubMetadata {
            type_: "http://schema.org/Book".to_string(),
            identifier: self.metadata().identifier.clone(),
            title: self.metadata().title.clone(),
            author: self.metadata().creators.clone(),
            publisher: self.metadata().publishers.first().cloned(),
            language: self.metadata().languages.clone(),
            description: self.metadata().description.clone(),
            reading_progression: reading_progression.to_string(),
        };

        let links = vec![WebpubLink {
            href: "manifest.json".to_string(),
            type_: "application/webpub+json".to_string(),
            title: None,
            rel: Some("self".to_string()),
            children: Vec::new(),
        }];

        let mut toc_titles: ahash::AHashMap<String, String> = ahash::AHashMap::new();
        collect_toc_titles(self.toc(), &mut toc_titles);

        let mut spine_hrefs: ahash::AHashSet<&str> = ahash::AHashSet::new();
        let reading_order = self
            .spine()
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                spine_hrefs.insert(item.href.as_str());
                let base_href = item.href.split('#').next().unwrap_or(&item.href);
                let title = toc_titles
                    .get(item.href.as_str())
                    .or_else(|| toc_titles.get(base_href))
                    .cloned()
                    .unwrap_or_else(|| format!("Section {}", idx + 1));
                WebpubLink {
                    href: item.href.clone(),
                    type_: if item.media_type.is_empty() {
                        "application/xhtml+xml".to_string()
                    } else {
                        item.media_type.clone()
                    },
                    title: Some(title),
                    rel: None,
                    children: Vec::new(),
                }
            })
            .collect();

        let cover_id = self.metadata().cover_id.as_deref();
        let cover_href = self.metadata().cover_href.as_deref();
        let mut resources: Vec<WebpubLink> = self
            .manifest()
            .values()
            .filter(|item| !spine_hrefs.contains(item.href.as_str()))
            .map(|item| {
                let is_cover =
                    cover_id == Some(item.id.as_str()) || cover_href == Some(item.href.as_str());
                WebpubLink {
                    href: item.href.clone(),
                    type_: if item.media_type.is_empty() {
                        "application/octet-stream".to_string()
                    } else {
                        item.media_type.clone()
                    },
                    title: None,
                    rel: if is_cover {
                        Some("cover".to_string())
                    } else {
                        None
                    },
                    children: Vec::new(),
                }
            })
            .collect();
        resources.sort_by(|a, b| a.href.cmp(&b.href));

        let toc = self.toc().iter().map(nav_point_to_webpub_link).collect();

        WebpubManifest {
            context: "https://readium.org/webpub-manifest/context.jsonld".to_string(),
            metadata,
            links,
            reading_order,
            resources,
            toc,
        }
    }

    /// Export the Readium Webpub Manifest as a JSON string.
    pub fn to_webpub_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.to_webpub_manifest())
            .map_err(|e| format!("Failed to serialize Webpub manifest: {}", e))
    }
}

fn nav_point_to_webpub_link(p: &crate::nav::NavPoint) -> WebpubLink {
    WebpubLink {
        href: p.href.clone(),
        type_: "application/xhtml+xml".to_string(),
        title: Some(p.label.clone()),
        rel: None,
        children: p.subitems.iter().map(nav_point_to_webpub_link).collect(),
    }
}

fn collect_toc_titles(points: &[crate::nav::NavPoint], out: &mut ahash::AHashMap<String, String>) {
    for p in points {
        if !p.href.is_empty() && !p.label.trim().is_empty() {
            let base = p.href.split('#').next().unwrap_or(&p.href).to_string();
            out.entry(p.href.clone()).or_insert_with(|| p.label.clone());
            out.entry(base).or_insert_with(|| p.label.clone());
        }
        collect_toc_titles(&p.subitems, out);
    }
}
