use crate::cfi::Cfi;
use crate::section::Section;
use serde::{Deserialize, Serialize};

/// A single search result item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub spine_index: usize,
    pub snippet: String,
    pub cfi: String,
    pub char_offset: usize,
}

/// Full-text search engine across chapter sections.
pub struct SearchEngine;

impl SearchEngine {
    /// Perform full-text search over a slice of sections.
    /// P4 Fix: Reuses pre-computed section.plain_text_lower for zero-allocation searching.
    pub fn search(sections: &[Section], query: &str, case_sensitive: bool) -> Vec<SearchResult> {
        let mut results = Vec::new();
        if query.trim().is_empty() {
            return results;
        }

        let query_cmp = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        for section in sections {
            let text_ref = if case_sensitive {
                &section.plain_text
            } else {
                &section.plain_text_lower
            };

            let mut search_idx = 0;
            while let Some(match_idx) = text_ref[search_idx..].find(&query_cmp) {
                let abs_idx = search_idx + match_idx;

                // Extract context snippet
                let snippet_start = abs_idx.saturating_sub(40);
                let snippet_end = (abs_idx + query.len() + 40).min(section.plain_text.len());

                let prefix = if snippet_start > 0 { "..." } else { "" };
                let suffix = if snippet_end < section.plain_text.len() {
                    "..."
                } else {
                    ""
                };

                let snippet = format!(
                    "{}{}{}",
                    prefix,
                    &section.plain_text[snippet_start..snippet_end],
                    suffix
                );

                // Generate target CFI for search match
                let cfi = Cfi::from_spine_index(section.index, None, abs_idx).to_string();

                results.push(SearchResult {
                    spine_index: section.index,
                    snippet,
                    cfi,
                    char_offset: abs_idx,
                });

                search_idx = abs_idx + query.len().max(1);
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search() {
        let sec = Section {
            index: 0,
            idref: "ch1".to_string(),
            href: "ch1.xhtml".to_string(),
            full_path: "OEBPS/ch1.xhtml".to_string(),
            raw_html: "<p>Hello Rust Reader</p>".to_string(),
            processed_html: "<p>Hello Rust Reader</p>".to_string(),
            plain_text: "Hello Rust Reader".to_string(),
            plain_text_lower: "hello rust reader".to_string(),
            char_count: 17,
        };

        let results = SearchEngine::search(&[sec], "Rust", false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].spine_index, 0);
        assert!(results[0].snippet.contains("Rust"));
    }
}
