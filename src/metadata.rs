use ahash::AHashMap;
use serde::{Deserialize, Serialize};

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
    pub meta_properties: AHashMap<String, String>,
    pub accessibility: AccessibilityMetadata,
}

impl Metadata {
    /// Return first creator (author) or empty string.
    pub fn creator(&self) -> &str {
        self.creators.first().map(|s| s.as_str()).unwrap_or("")
    }

    /// Return first publisher if present.
    pub fn publisher(&self) -> Option<&str> {
        self.publishers.first().map(|s| s.as_str())
    }

    /// Return first language or empty string.
    pub fn language(&self) -> &str {
        self.languages.first().map(|s| s.as_str()).unwrap_or("")
    }
}

/// EPUB 3 Accessibility (a11y) Metadata conforming to W3C EPUB Accessibility 1.1 & Schema.org.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessibilityMetadata {
    pub access_modes: Vec<String>,
    pub access_modes_sufficient: Vec<Vec<String>>,
    pub accessibility_features: Vec<String>,
    pub accessibility_hazards: Vec<String>,
    pub accessibility_summary: Option<String>,
    pub certified_by: Option<String>,
    pub certifier_credential: Option<String>,
    pub certifier_report: Option<String>,
    pub is_accessible: bool,
}

impl AccessibilityMetadata {
    /// Returns true if the EPUB specifies alternative text for visual assets.
    pub fn has_alternative_text(&self) -> bool {
        self.accessibility_features
            .iter()
            .any(|f| f.contains("alternativeText"))
    }

    /// Returns true if structural navigation (TOC/headings) is present.
    pub fn has_structural_navigation(&self) -> bool {
        self.accessibility_features
            .iter()
            .any(|f| f.contains("structuralNavigation") || f.contains("tableOfContents"))
    }

    /// Returns true if the book is screen-reader friendly (has text + structural nav).
    pub fn is_screen_reader_friendly(&self) -> bool {
        self.access_modes.iter().any(|m| m == "textual") && self.has_structural_navigation()
    }
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
