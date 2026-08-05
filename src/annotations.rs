use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of annotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationType {
    Highlight,
    Underline,
    Bookmark,
    Note,
}

/// An annotation item associated with a CFI or CFI range.
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

/// Manager for maintaining annotations across a book session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnnotationManager {
    annotations: HashMap<String, Annotation>,
}

impl AnnotationManager {
    pub fn new() -> Self {
        Self {
            annotations: HashMap::new(),
        }
    }

    /// Add or replace an annotation.
    pub fn add(&mut self, annotation: Annotation) {
        self.annotations.insert(annotation.id.clone(), annotation);
    }

    /// Create a new highlight.
    pub fn create_highlight(
        &mut self,
        cfi_range: &str,
        color: &str,
        text: Option<&str>,
        note: Option<&str>,
    ) -> Annotation {
        let id = format!("ann-{}", uuid_simple());
        let ann = Annotation {
            id: id.clone(),
            cfi_range: cfi_range.to_string(),
            type_: AnnotationType::Highlight,
            color: color.to_string(),
            note: note.map(|s| s.to_string()),
            selected_text: text.map(|s| s.to_string()),
            created_at: current_timestamp_str(),
        };
        self.annotations.insert(id, ann.clone());
        ann
    }

    /// Create a bookmark at CFI.
    pub fn create_bookmark(&mut self, cfi: &str, note: Option<&str>) -> Annotation {
        let id = format!("bm-{}", uuid_simple());
        let ann = Annotation {
            id: id.clone(),
            cfi_range: cfi.to_string(),
            type_: AnnotationType::Bookmark,
            color: "#f59e0b".to_string(),
            note: note.map(|s| s.to_string()),
            selected_text: None,
            created_at: current_timestamp_str(),
        };
        self.annotations.insert(id, ann.clone());
        ann
    }

    /// Remove annotation by ID.
    pub fn remove(&mut self, id: &str) -> Option<Annotation> {
        self.annotations.remove(id)
    }

    /// Get annotation by ID.
    pub fn get(&self, id: &str) -> Option<&Annotation> {
        self.annotations.get(id)
    }

    /// List all annotations.
    pub fn list(&self) -> Vec<&Annotation> {
        let mut list: Vec<&Annotation> = self.annotations.values().collect();
        list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        list
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

fn current_timestamp_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}
