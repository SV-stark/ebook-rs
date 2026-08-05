use crate::cfi::Cfi;
use serde::{Deserialize, Serialize};

/// A single location entry generated across the EPUB spine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationEntry {
    pub location: usize,
    pub spine_index: usize,
    pub char_offset: usize,
    pub cfi: String,
    pub percentage: f32,
}

/// Location Manager for calculating page/location progress and CFI mapping.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Locations {
    pub chunk_size: usize,
    pub total_locations: usize,
    pub total_characters: usize,
    pub entries: Vec<LocationEntry>,
}

impl Locations {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            total_locations: 0,
            total_characters: 0,
            entries: Vec::new(),
        }
    }

    /// Add spine section text to generate locations.
    pub fn add_spine_section(&mut self, spine_index: usize, plain_text: &str) {
        let char_count = plain_text.chars().count();
        if char_count == 0 {
            let loc_num = self.entries.len() + 1;
            let cfi = Cfi::from_spine_index(spine_index, None, 0).to_string();
            self.entries.push(LocationEntry {
                location: loc_num,
                spine_index,
                char_offset: 0,
                cfi,
                percentage: 0.0,
            });
            return;
        }

        let mut offset = 0;
        while offset < char_count {
            let loc_num = self.entries.len() + 1;
            let cfi = Cfi::from_spine_index(spine_index, None, offset).to_string();
            self.entries.push(LocationEntry {
                location: loc_num,
                spine_index,
                char_offset: offset,
                cfi,
                percentage: 0.0,
            });
            offset += self.chunk_size;
        }

        self.total_characters += char_count;
    }

    /// Finalize location percentages after all sections have been added.
    pub fn finalize(&mut self) {
        self.total_locations = self.entries.len();
        let total = self.total_locations as f32;
        if total > 0.0 {
            for (idx, entry) in self.entries.iter_mut().enumerate() {
                entry.percentage = (idx as f32) / (total - 1.0).max(1.0);
            }
        }
    }

    /// Find location entry closest to given CFI string.
    pub fn location_from_cfi(&self, cfi_str: &str) -> Option<&LocationEntry> {
        let target_cfi = Cfi::parse(cfi_str).ok()?;
        let mut best = None;
        let mut min_diff = usize::MAX;

        for entry in &self.entries {
            if entry.spine_index == target_cfi.spine_index() {
                let diff = (entry.char_offset as isize - target_cfi.char_offset() as isize).abs() as usize;
                if diff < min_diff {
                    min_diff = diff;
                    best = Some(entry);
                }
            }
        }

        best.or_else(|| self.entries.first())
    }

    /// Find CFI string closest to given location number.
    pub fn cfi_from_location(&self, location: usize) -> Option<String> {
        if location == 0 || self.entries.is_empty() {
            return self.entries.first().map(|e| e.cfi.clone());
        }
        if location > self.entries.len() {
            return self.entries.last().map(|e| e.cfi.clone());
        }
        self.entries.get(location - 1).map(|e| e.cfi.clone())
    }

    /// Find CFI string from percentage (0.0 to 1.0).
    pub fn cfi_from_percentage(&self, percentage: f32) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        let clamped = percentage.clamp(0.0, 1.0);
        let target_idx = ((self.entries.len() - 1) as f32 * clamped).round() as usize;
        self.entries.get(target_idx).map(|e| e.cfi.clone())
    }

    /// Find percentage from CFI string.
    pub fn percentage_from_cfi(&self, cfi_str: &str) -> f32 {
        if let Some(entry) = self.location_from_cfi(cfi_str) {
            entry.percentage
        } else {
            0.0
        }
    }
}
