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
    pub title: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub author: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<String>,
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
            title: self.metadata().title.clone(),
            author: self.metadata().creators.clone(),
            publisher: self.metadata().publishers.first().cloned(),
            language: self.metadata().languages.clone(),
            reading_progression: reading_progression.to_string(),
        };

        let links = vec![WebpubLink {
            href: "manifest.json".to_string(),
            type_: "application/webpub+json".to_string(),
            title: None,
            rel: Some("self".to_string()),
            children: Vec::new(),
        }];

        let reading_order = self
            .spine()
            .iter()
            .enumerate()
            .map(|(idx, item)| WebpubLink {
                href: item.href.clone(),
                type_: if item.media_type.is_empty() {
                    "application/xhtml+xml".to_string()
                } else {
                    item.media_type.clone()
                },
                title: Some(format!("Section {}", idx + 1)),
                rel: None,
                children: Vec::new(),
            })
            .collect();

        let toc = self
            .toc()
            .iter()
            .map(|p| WebpubLink {
                href: p.href.clone(),
                type_: "application/xhtml+xml".to_string(),
                title: Some(p.label.clone()),
                rel: None,
                children: p
                    .subitems
                    .iter()
                    .map(|sub| WebpubLink {
                        href: sub.href.clone(),
                        type_: "application/xhtml+xml".to_string(),
                        title: Some(sub.label.clone()),
                        rel: None,
                        children: Vec::new(),
                    })
                    .collect(),
            })
            .collect();

        WebpubManifest {
            context: "https://readium.org/webpub-manifest/context.jsonld".to_string(),
            metadata,
            links,
            reading_order,
            resources: Vec::new(),
            toc,
        }
    }

    /// Export the Readium Webpub Manifest as a JSON string.
    pub fn to_webpub_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.to_webpub_manifest())
            .map_err(|e| format!("Failed to serialize Webpub manifest: {}", e))
    }
}
