use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ATOMIC_ANN_ID: AtomicU64 = AtomicU64::new(1);

/// Type of CFI-anchored user annotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationType {
    Highlight,
    Underline,
    Bookmark,
    Note,
}

/// A user annotation entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub cfi_range: String,
    pub type_: AnnotationType,
    pub color: String,
    pub note: Option<String>,
    pub selected_text: Option<String>,
    pub created_at: String,
}

/// Manager for annotations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnnotationManager {
    annotations: AHashMap<String, Annotation>,
}

impl AnnotationManager {
    pub fn new() -> Self {
        Self {
            annotations: AHashMap::new(),
        }
    }

    /// Add an annotation entry.
    pub fn add(&mut self, annotation: Annotation) {
        self.annotations.insert(annotation.id.clone(), annotation);
    }

    /// Create a highlight annotation.
    pub fn create_highlight(
        &mut self,
        cfi_range: &str,
        color: &str,
        selected_text: Option<&str>,
        note: Option<&str>,
    ) -> Annotation {
        let ann = Annotation {
            id: generate_unique_id("hl"),
            cfi_range: cfi_range.to_string(),
            type_: AnnotationType::Highlight,
            color: color.to_string(),
            note: note.map(|s| s.to_string()),
            selected_text: selected_text.map(|s| s.to_string()),
            created_at: current_timestamp_str(),
        };
        self.add(ann.clone());
        ann
    }

    /// Create a bookmark annotation.
    pub fn create_bookmark(&mut self, cfi: &str, note: Option<&str>) -> Annotation {
        let ann = Annotation {
            id: generate_unique_id("bm"),
            cfi_range: cfi.to_string(),
            type_: AnnotationType::Bookmark,
            color: "#f59e0b".to_string(),
            note: note.map(|s| s.to_string()),
            selected_text: None,
            created_at: current_timestamp_str(),
        };
        self.add(ann.clone());
        ann
    }

    /// Get annotation by ID.
    pub fn get(&self, id: &str) -> Option<&Annotation> {
        self.annotations.get(id)
    }

    /// Remove an annotation by ID.
    pub fn remove(&mut self, id: &str) -> bool {
        self.annotations.remove(id).is_some()
    }

    /// List all annotations.
    pub fn list(&self) -> Vec<Annotation> {
        self.annotations.values().cloned().collect()
    }

    /// Export annotations as W3C Web Annotation Data Model (JSON-LD) format (F10 Fix).
    pub fn to_w3c_json(&self) -> Result<String, String> {
        let items: Vec<serde_json::Value> = self
            .annotations
            .values()
            .map(|ann| {
                serde_json::json!({
                    "@context": "http://www.w3.org/ns/anno.jsonld",
                    "id": format!("urn:annotation:{}", ann.id),
                    "type": "Annotation",
                    "motivation": match ann.type_ {
                        AnnotationType::Highlight => "highlighting",
                        AnnotationType::Bookmark => "bookmarking",
                        AnnotationType::Underline => "underlining",
                        AnnotationType::Note => "commenting",
                    },
                    "target": {
                        "selector": {
                            "type": "FragmentSelector",
                            "conformsTo": "http://www.idpf.org/epub/linking/cfi/epub-cfi.html",
                            "value": ann.cfi_range
                        }
                    },
                    "body": {
                        "type": "TextualBody",
                        "value": ann.note.as_deref().unwrap_or(""),
                        "format": "text/plain"
                    },
                    "created": ann.created_at
                })
            })
            .collect();

        serde_json::to_string(&items).map_err(|e| e.to_string())
    }
}

/// B6 Fix: Generate 100% collision-free unique IDs using atomic sequence counters + timestamp.
fn generate_unique_id(prefix: &str) -> String {
    let seq = ATOMIC_ANN_ID.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{:x}-{:x}", prefix, nanos, seq)
}

fn current_timestamp_str() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    secs.to_string()
}
