use serde::{Deserialize, Serialize};

/// Location entry representing a discrete chunk of text across the EPUB spine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationEntry {
    pub location: usize,
    pub spine_index: usize,
    pub char_start: usize,
    pub char_end: usize,
}

/// Locations manager mapping character offsets to locations and progress percentage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Locations {
    pub chunk_size: usize,
    pub entries: Vec<LocationEntry>,
    pub total_locations: usize,
    pub total_characters: usize,
}

impl Default for Locations {
    fn default() -> Self {
        Self::new(1000) // Default 1000 character chunk size
    }
}

/// Readium Standard Locator Locations object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocatorLocations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
    pub progression: f64,
    #[serde(rename = "totalProgression")]
    pub total_progression: f64,
}

/// Readium Unified Locator Model conforming to W3C / Readium Specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadiumLocator {
    pub href: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub locations: LocatorLocations,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<serde_json::Value>,
}

impl Locations {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            entries: Vec::new(),
            total_locations: 0,
            total_characters: 0,
        }
    }

    /// Add a spine section's plain text and generate location entries.
    /// P5 Fix: Use pre-computed char_count directly to avoid double UTF-8 scanning.
    pub fn add_spine_section(&mut self, spine_index: usize, plain_text: &str) {
        let text_len = plain_text.chars().count();
        self.total_characters += text_len;
        if text_len == 0 {
            self.entries.push(LocationEntry {
                location: self.total_locations + 1,
                spine_index,
                char_start: 0,
                char_end: 0,
            });
            self.total_locations += 1;
            return;
        }

        let mut offset = 0;
        while offset < text_len {
            let next_offset = (offset + self.chunk_size).min(text_len);
            self.total_locations += 1;
            self.entries.push(LocationEntry {
                location: self.total_locations,
                spine_index,
                char_start: offset,
                char_end: next_offset,
            });
            offset = next_offset;
        }
    }

    pub fn finalize(&mut self) {
        if self.total_locations == 0 {
            self.total_locations = 1;
        }
    }

    /// Retrieve LocationEntry for a given CFI string.
    pub fn location_from_cfi(&self, cfi_str: &str) -> Option<LocationEntry> {
        let cfi = crate::cfi::Cfi::parse(cfi_str).ok()?;
        let spine_idx = cfi.spine_index();
        let char_off = cfi.char_offset();

        for entry in &self.entries {
            if entry.spine_index == spine_idx
                && char_off >= entry.char_start
                && char_off <= entry.char_end
            {
                return Some(entry.clone());
            }
        }
        self.entries
            .iter()
            .find(|e| e.spine_index == spine_idx)
            .cloned()
    }

    /// Map Location number to CFI string.
    pub fn cfi_from_location(&self, location: usize) -> Option<String> {
        let entry = self.entries.iter().find(|e| e.location == location)?;
        Some(
            crate::cfi::Cfi::from_spine_index(entry.spine_index, None, entry.char_start)
                .to_string(),
        )
    }

    /// Map spine index and character offset to location entry.
    pub fn location_from_char_offset(
        &self,
        spine_idx: usize,
        char_off: usize,
    ) -> Option<LocationEntry> {
        for entry in &self.entries {
            if entry.spine_index == spine_idx
                && char_off >= entry.char_start
                && char_off <= entry.char_end
            {
                return Some(entry.clone());
            }
        }
        self.entries
            .iter()
            .find(|e| e.spine_index == spine_idx)
            .cloned()
    }

    /// Convert location integer to decimal progress percentage (0.0 to 1.0).
    pub fn percentage_from_location(&self, location: usize) -> f64 {
        if self.total_locations == 0 {
            return 0.0;
        }
        (location as f64 / self.total_locations as f64).clamp(0.0, 1.0)
    }

    /// Get progress percentage (0.0 to 1.0) for a given CFI.
    pub fn percentage_from_cfi(&self, cfi_str: &str) -> f64 {
        if let Some(entry) = self.location_from_cfi(cfi_str) {
            self.percentage_from_location(entry.location)
        } else {
            0.0
        }
    }
}
