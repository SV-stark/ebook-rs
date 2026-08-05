use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Page progression direction (LTR, RTL, Default).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageProgressionDirection {
    #[default]
    Ltr,
    Rtl,
    Default,
}

/// Metadata extracted from the EPUB package document (.opf).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub title: String,
    pub creators: Vec<String>,
    pub publishers: Vec<String>,
    pub languages: Vec<String>,
    pub rights: Option<String>,
    pub description: Option<String>,
    pub identifier: Option<String>,
    pub pub_date: Option<String>,
    pub modified_date: Option<String>,
    pub subjects: Vec<String>,
    pub cover_id: Option<String>,
    pub cover_href: Option<String>,
    pub direction: PageProgressionDirection,
    pub meta_properties: HashMap<String, String>,
}

/// A manifest item representing a resource inside the EPUB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestItem {
    pub id: String,
    pub href: String,
    pub full_path: String,
    pub media_type: String,
    pub properties: Vec<String>,
    pub fallback: Option<String>,
    pub media_overlay: Option<String>,
}

/// A spine item representing a reader page section in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpineItem {
    pub idref: String,
    pub linear: bool,
    pub properties: Vec<String>,
    pub index: usize,
    pub href: String,
    pub media_type: String,
}

/// Guide or Landmark reference entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideItem {
    pub type_: String,
    pub title: String,
    pub href: String,
    pub full_path: String,
}
