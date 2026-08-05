use crate::cfi::Cfi;
use crate::section::Section;
use serde::{Deserialize, Serialize};

/// Search match result item with CFI and snippet context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub spine_index: usize,
    pub href: String,
    pub section_title: String,
    pub cfi: String,
    pub snippet: String,
    pub match_start: usize,
    pub match_end: usize,
}

/// Search engine for searching full-text content across EPUB sections.
pub struct SearchEngine;

impl SearchEngine {
    /// Perform full-text search across sections.
    pub fn search(sections: &[Section], query: &str, case_sensitive: bool) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return results;
        }

        let query_cmp = if case_sensitive {
            trimmed.to_string()
        } else {
            trimmed.to_lowercase()
        };

        for section in sections {
            let text = &section.plain_text;
            let text_cmp = if case_sensitive {
                text.clone()
            } else {
                text.to_lowercase()
            };

            let mut search_idx = 0;
            while let Some(pos) = text_cmp[search_idx..].find(&query_cmp) {
                let abs_start = search_idx + pos;
                let abs_end = abs_start + query_cmp.len();

                // Build context snippet (~40 chars before and after)
                let snippet_start = abs_start.saturating_sub(40);
                let snippet_end = (abs_end + 40).min(text.len());

                let prefix = if snippet_start > 0 { "..." } else { "" };
                let suffix = if snippet_end < text.len() { "..." } else { "" };

                let raw_snippet = &text[snippet_start..snippet_end];
                let snippet = format!("{}{}{}", prefix, raw_snippet, suffix);

                // Generate exact CFI for match start position
                let cfi = Cfi::from_spine_index(section.index, None, abs_start).to_string();

                results.push(SearchResult {
                    spine_index: section.index,
                    href: section.href.clone(),
                    section_title: format!("Section {}", section.index + 1),
                    cfi,
                    snippet,
                    match_start: abs_start,
                    match_end: abs_end,
                });

                search_idx = abs_end;
            }
        }

        results
    }
}
