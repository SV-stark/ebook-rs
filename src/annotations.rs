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
        self.to_w3c_json_inner(None)
    }

    /// Export annotations as W3C Web Annotation Data Model (JSON-LD) with a `SpecificResource`
    /// `source` IRI on each target.
    pub fn to_w3c_json_with_source(&self, source_iri: &str) -> Result<String, String> {
        self.to_w3c_json_inner(Some(source_iri))
    }

    fn to_w3c_json_inner(&self, source_iri: Option<&str>) -> Result<String, String> {
        let items: Vec<serde_json::Value> = self
            .annotations
            .values()
            .map(|ann| {
                let selector = serde_json::json!({
                    "type": "FragmentSelector",
                    "conformsTo": "http://www.idpf.org/epub/linking/cfi/epub-cfi.html",
                    "value": ann.cfi_range
                });
                let target = match source_iri {
                    Some(src) => serde_json::json!({
                        "type": "SpecificResource",
                        "source": src,
                        "selector": selector
                    }),
                    None => serde_json::json!({
                        "selector": selector
                    }),
                };
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
                    "target": target,
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
        .as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let hour = rem / 3_600;
    let min = (rem % 3_600) / 60;
    let sec = rem % 60;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

/// Howard Hinnant's civil-from-days algorithm for UTC ISO-8601 formatting.
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = y + i64::from(m <= 2);
    (year, m, d)
}
