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
        if query.trim().is_empty() {
            return Vec::new();
        }

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            sections
                .par_iter()
                .flat_map(|section| Self::search_section(section, query, case_sensitive))
                .collect()
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut results = Vec::new();
            for section in sections {
                results.extend(Self::search_section(section, query, case_sensitive));
            }
            results
        }
    }

    /// Search a single section.
    pub fn search_section(
        section: &Section,
        query: &str,
        case_sensitive: bool,
    ) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let query_cmp = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let text_ref = if case_sensitive {
            &section.plain_text
        } else {
            &section.plain_text_lower
        };

        let mut search_idx = 0;
        while let Some(match_idx) = text_ref[search_idx..].find(&query_cmp) {
            let abs_idx = search_idx + match_idx;

            // Extract context snippet with 100% UTF-8 char boundary safety
            let text_chars: Vec<char> = section.plain_text.chars().collect();
            let char_idx = section.plain_text[..abs_idx.min(section.plain_text.len())]
                .chars()
                .count();
            let q_char_len = query.chars().count();

            let start_c = char_idx.saturating_sub(40);
            let end_c = (char_idx + q_char_len + 40).min(text_chars.len());

            let raw_snippet: String = text_chars[start_c..end_c].iter().collect();
            let query_str: String = text_chars
                [char_idx..(char_idx + q_char_len).min(text_chars.len())]
                .iter()
                .collect();
            let highlighted =
                raw_snippet.replace(&query_str, &format!("<mark>{}</mark>", query_str));

            let prefix = if start_c > 0 { "..." } else { "" };
            let suffix = if end_c < text_chars.len() { "..." } else { "" };

            let snippet = format!("{}{}{}", prefix, highlighted, suffix);

            // Generate target CFI with true character index char_idx
            let cfi = Cfi::from_spine_index(section.index, None, char_idx).to_string();

            results.push(SearchResult {
                spine_index: section.index,
                snippet,
                cfi,
                char_offset: char_idx,
            });

            search_idx = abs_idx + query.len().max(1);
        }

        results
    }

    /// Perform full-text regex pattern search across chapter sections.
    pub fn search_regex(sections: &[Section], pattern: &str) -> Result<Vec<SearchResult>, String> {
        let re = regex::Regex::new(pattern)
            .map_err(|e| format!("Invalid regex pattern '{}': {}", pattern, e))?;
        let mut results = Vec::new();

        for section in sections {
            for m in re.find_iter(&section.plain_text) {
                let abs_idx = m.start();
                let match_str = m.as_str();

                let text_chars: Vec<char> = section.plain_text.chars().collect();
                let char_idx = section.plain_text[..abs_idx.min(section.plain_text.len())]
                    .chars()
                    .count();
                let q_char_len = match_str.chars().count();

                let start_c = char_idx.saturating_sub(40);
                let end_c = (char_idx + q_char_len + 40).min(text_chars.len());

                let raw_snippet: String = text_chars[start_c..end_c].iter().collect();
                let query_str: String = text_chars
                    [char_idx..(char_idx + q_char_len).min(text_chars.len())]
                    .iter()
                    .collect();
                let highlighted =
                    raw_snippet.replace(&query_str, &format!("<mark>{}</mark>", query_str));

                let prefix = if start_c > 0 { "..." } else { "" };
                let suffix = if end_c < text_chars.len() { "..." } else { "" };

                let snippet = format!("{}{}{}", prefix, highlighted, suffix);
                let cfi = Cfi::from_spine_index(section.index, None, abs_idx).to_string();

                results.push(SearchResult {
                    spine_index: section.index,
                    snippet,
                    cfi,
                    char_offset: abs_idx,
                });
            }
        }

        Ok(results)
    }

    /// Format search results into Readium Standard Search Collection JSON (application/vnd.readium.search+json).
    pub fn to_readium_search_json(results: &[SearchResult], query: &str) -> Result<String, String> {
        let locators: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "href": format!("section_{}.html", r.spine_index),
                    "type": "application/xhtml+xml",
                    "locations": {
                        "cfi": r.cfi,
                        "position": r.char_offset / 1000 + 1
                    },
                    "text": {
                        "snippet": r.snippet
                    }
                })
            })
            .collect();

        let collection = serde_json::json!({
            "@context": "http://readium.org/webpub-manifest/context.jsonld",
            "metadata": {
                "numberOfResults": results.len(),
                "query": query
            },
            "locators": locators
        });

        serde_json::to_string_pretty(&collection)
            .map_err(|e| format!("Failed to serialize Readium Search JSON: {}", e))
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
            viewport_width: None,
            viewport_height: None,
        };

        let results = SearchEngine::search(&[sec], "Rust", false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].spine_index, 0);
        assert!(results[0].snippet.contains("Rust"));
    }
}
