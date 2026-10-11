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

/// Default cap on returned matches. Callers may request a different limit via
/// [`SearchEngine::search_limited`].
pub const DEFAULT_MAX_RESULTS: usize = 500;
/// Absolute ceiling on returned matches, regardless of what a caller asks for.
pub const ABSOLUTE_MAX_RESULTS: usize = 5000;

impl SearchEngine {
    /// Perform full-text search over a slice of sections using the default result cap.
    pub fn search(sections: &[Section], query: &str, case_sensitive: bool) -> Vec<SearchResult> {
        Self::search_limited(sections, query, case_sensitive, None)
    }

    /// Perform full-text search, stopping once `max_results` matches have been collected.
    ///
    /// The cap is applied *while* scanning. Materializing every match and truncating afterwards
    /// is itself the denial of service: a single-character query against a large book produces
    /// millions of heap-allocated `SearchResult`s.
    ///
    /// `max_results` of `None` uses [`DEFAULT_MAX_RESULTS`]. Values above
    /// [`ABSOLUTE_MAX_RESULTS`] are clamped.
    pub fn search_limited(
        sections: &[Section],
        query: &str,
        case_sensitive: bool,
        max_results: Option<usize>,
    ) -> Vec<SearchResult> {
        let limit = max_results
            .unwrap_or(DEFAULT_MAX_RESULTS)
            .min(ABSOLUTE_MAX_RESULTS);
        if limit == 0 || query.trim().is_empty() {
            return Vec::new();
        }

        let query_pattern = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let precompiled_re = if !case_sensitive && !query.is_ascii() {
            regex::Regex::new(&format!("(?i){}", regex::escape(query))).ok()
        } else {
            None
        };

        // Sequential accumulation with an early exit. Deliberately not `par_iter().collect()`:
        // that form has to build the whole result set before anything can be truncated.
        //
        // The cap is threaded *into* the section scan, not applied between sections: a book whose
        // single spine section is 10 MB used to materialize every match in that section before
        // truncating, so the documented "do not materialize the full set" guarantee did not hold.
        let mut results: Vec<SearchResult> = Vec::new();
        for section in sections {
            if results.len() >= limit {
                break;
            }
            let remaining = limit.saturating_sub(results.len());
            let found = Self::search_section_prepared(
                section,
                &query_pattern,
                case_sensitive,
                precompiled_re.as_ref(),
                Some(remaining),
            );
            results.extend(found);
        }
        results
    }

    /// Search a single section safely without cross-string offset mismatch panics.
    pub fn search_section(
        section: &Section,
        query: &str,
        case_sensitive: bool,
    ) -> Vec<SearchResult> {
        let query_pattern = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        let precompiled_re = if !case_sensitive && !query.is_ascii() {
            regex::Regex::new(&format!("(?i){}", regex::escape(query))).ok()
        } else {
            None
        };
        Self::search_section_prepared_inner(
            section,
            &query_pattern,
            case_sensitive,
            precompiled_re.as_ref(),
            None,
        )
    }

    fn search_section_prepared(
        section: &Section,
        query: &str,
        case_sensitive: bool,
        precompiled_re: Option<&regex::Regex>,
        limit: Option<usize>,
    ) -> Vec<SearchResult> {
        let remaining = match limit {
            Some(0) => return Vec::new(),
            Some(n) => n,
            None => DEFAULT_MAX_RESULTS,
        };
        Self::search_section_prepared_inner(
            section,
            query,
            case_sensitive,
            precompiled_re,
            Some(remaining),
        )
    }

    fn search_section_prepared_inner(
        section: &Section,
        query: &str,
        case_sensitive: bool,
        precompiled_re: Option<&regex::Regex>,
        limit: Option<usize>,
    ) -> Vec<SearchResult> {
        let cap = limit.unwrap_or(DEFAULT_MAX_RESULTS);
        let mut results = Vec::new();
        if query.trim().is_empty() || section.plain_text.is_empty() {
            return results;
        }

        let is_pure_ascii = section.plain_text.is_ascii() && query.is_ascii();
        // Hoisted once per section: the snippet extractor needs to know whether it can slice by
        // byte offset, and `str::is_ascii()` is itself an O(n) scan of the whole section text.
        let plain_text_is_ascii = section.plain_text.is_ascii();

        if case_sensitive || is_pure_ascii {
            let target_text = if case_sensitive {
                &section.plain_text
            } else {
                &section.plain_text_lower
            };
            let query_low = if case_sensitive {
                query.to_string()
            } else {
                query.to_ascii_lowercase()
            };

            let finder = memchr::memmem::Finder::new(query_low.as_bytes());

            for match_byte_idx in finder.find_iter(target_text.as_bytes()) {
                if results.len() >= cap {
                    break;
                }
                let char_offset = if is_pure_ascii {
                    match_byte_idx
                } else {
                    target_text[..match_byte_idx].chars().count()
                };

                let match_len = query_low.len();

                let (before, matched, after, has_prefix, has_suffix) = extract_zero_alloc_snippet(
                    &section.plain_text,
                    match_byte_idx,
                    match_len,
                    plain_text_is_ascii,
                );

                let prefix = if has_prefix { "..." } else { "" };
                let suffix = if has_suffix { "..." } else { "" };

                let snippet = format!(
                    "{}{}<mark>{}</mark>{}{}",
                    prefix,
                    html_escape(before),
                    html_escape(matched),
                    html_escape(after),
                    suffix
                );

                let cfi = Cfi::from_spine_index(section.index, None, char_offset).to_string();

                results.push(SearchResult {
                    spine_index: section.index,
                    snippet,
                    cfi,
                    char_offset,
                });
            }
        } else {
            // Non-ASCII case-insensitive matching: match directly on section.plain_text
            let owned_re;
            let re = if let Some(r) = precompiled_re {
                r
            } else {
                let pattern = format!("(?i){}", regex::escape(query));
                if let Ok(r) = regex::Regex::new(&pattern) {
                    owned_re = Some(r);
                    owned_re.as_ref().unwrap()
                } else {
                    return results;
                }
            };
            for m in re.find_iter(&section.plain_text) {
                if results.len() >= cap {
                    break;
                }
                let start_b = m.start();
                let match_len = m.len();
                let char_offset = section.plain_text[..start_b].chars().count();

                let (before, matched, after, has_prefix, has_suffix) = extract_zero_alloc_snippet(
                    &section.plain_text,
                    start_b,
                    match_len,
                    plain_text_is_ascii,
                );

                let prefix = if has_prefix { "..." } else { "" };
                let suffix = if has_suffix { "..." } else { "" };

                let snippet = format!(
                    "{}{}<mark>{}</mark>{}{}",
                    prefix,
                    html_escape(before),
                    html_escape(matched),
                    html_escape(after),
                    suffix
                );

                let cfi = Cfi::from_spine_index(section.index, None, char_offset).to_string();

                results.push(SearchResult {
                    spine_index: section.index,
                    snippet,
                    cfi,
                    char_offset,
                });
            }
        }

        results
    }

    /// Perform full-text regex pattern search across chapter sections.
    ///
    /// `pattern` is compiled with the `regex` crate, which is linear-time, but the number of
    /// matches is still bounded so a broad pattern cannot build an unbounded result set.
    pub fn search_regex(sections: &[Section], pattern: &str) -> Result<Vec<SearchResult>, String> {
        Self::search_regex_limited(sections, pattern, None)
    }

    /// Regex search with an explicit cap on the number of returned matches.
    ///
    /// # Errors
    /// Returns an error if `pattern` is not a valid regular expression.
    pub fn search_regex_limited(
        sections: &[Section],
        pattern: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<SearchResult>, String> {
        let limit = max_results
            .unwrap_or(DEFAULT_MAX_RESULTS)
            .min(ABSOLUTE_MAX_RESULTS);
        let re = regex::Regex::new(pattern)
            .map_err(|e| format!("Invalid regex pattern '{}': {}", pattern, e))?;
        let mut results = Vec::new();
        if limit == 0 {
            return Ok(results);
        }

        'outer: for section in sections {
            let section_is_ascii = section.plain_text.is_ascii();
            for m in re.find_iter(&section.plain_text) {
                let start_b = m.start();
                let match_len = m.len();
                let char_idx = section.plain_text[..start_b.min(section.plain_text.len())]
                    .chars()
                    .count();

                let (before, matched, after, has_prefix, has_suffix) = extract_zero_alloc_snippet(
                    &section.plain_text,
                    start_b,
                    match_len,
                    section_is_ascii,
                );

                let prefix = if has_prefix { "..." } else { "" };
                let suffix = if has_suffix { "..." } else { "" };

                let snippet = format!(
                    "{}{}<mark>{}</mark>{}{}",
                    prefix,
                    html_escape(before),
                    html_escape(matched),
                    html_escape(after),
                    suffix
                );
                let cfi = Cfi::from_spine_index(section.index, None, char_idx).to_string();

                results.push(SearchResult {
                    spine_index: section.index,
                    snippet,
                    cfi,
                    char_offset: char_idx,
                });

                if results.len() >= limit {
                    break 'outer;
                }
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

/// Extract snippet window surrounding a match without converting the entire string to a Vec<char>.
///
/// `is_ascii` must be computed once by the caller: `text.is_ascii()` is itself an O(n) scan of the
/// whole section, so calling it per match made snippet extraction cost
/// `matches × section_length` — 400 KB of text with 200 000 matches took ~2 s of pure scanning.
fn extract_zero_alloc_snippet(
    text: &str,
    byte_offset: usize,
    match_bytes_len: usize,
    is_ascii: bool,
) -> (&str, &str, &str, bool, bool) {
    if is_ascii {
        let start_b = byte_offset.saturating_sub(40);
        let end_match_b = (byte_offset + match_bytes_len).min(text.len());
        let end_b = (end_match_b + 40).min(text.len());

        let prefix = start_b > 0;
        let suffix = end_b < text.len();

        let before = &text[start_b..byte_offset];
        let matched = &text[byte_offset..end_match_b];
        let after = &text[end_match_b..end_b];

        (before, matched, after, prefix, suffix)
    } else {
        let safe_byte_offset = byte_offset.min(text.len());
        let before_slice = &text[..safe_byte_offset];
        let mut start_b = 0;
        let mut count = 0;
        for (idx, _) in before_slice.char_indices().rev() {
            count += 1;
            if count == 40 {
                start_b = idx;
                break;
            }
        }

        let end_match_b = (safe_byte_offset + match_bytes_len).min(text.len());
        let after_slice = &text[end_match_b..];
        let mut end_b = text.len();
        let mut count = 0;
        for (idx, c) in after_slice.char_indices() {
            count += 1;
            if count == 40 {
                end_b = end_match_b + idx + c.len_utf8();
                break;
            }
        }

        let prefix = start_b > 0;
        let suffix = end_b < text.len();

        let before = &text[start_b..safe_byte_offset];
        let matched = &text[safe_byte_offset..end_match_b];
        let after = &text[end_match_b..end_b];

        (before, matched, after, prefix, suffix)
    }
}

/// Helper function to HTML-escape arbitrary text strings to prevent stored XSS attacks.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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

    #[test]
    fn test_search_unicode_boundary_safety() {
        let sec = Section {
            index: 0,
            idref: "ch1".to_string(),
            href: "ch1.xhtml".to_string(),
            full_path: "OEBPS/ch1.xhtml".to_string(),
            raw_html: "<p>ẞfoo bar</p>".to_string(),
            processed_html: "<p>ẞfoo bar</p>".to_string(),
            plain_text: "ẞfoo bar".to_string(),
            plain_text_lower: "ssfoo bar".to_string(),
            char_count: 8,
            viewport_width: None,
            viewport_height: None,
        };

        let results = SearchEngine::search(&[sec], "foo", false);
        assert_eq!(results.len(), 1);
        assert!(results[0].snippet.contains("foo"));
    }

    /// A single large section must not materialize every match before truncating: the cap is
    /// threaded into the section scan, not applied between sections.
    #[test]
    fn search_limited_bounds_allocation_for_one_large_section() {
        let len = 200_000;
        let sec = Section {
            index: 0,
            idref: "ch1".to_string(),
            href: "ch1.xhtml".to_string(),
            full_path: "OEBPS/ch1.xhtml".to_string(),
            raw_html: String::new(),
            processed_html: String::new(),
            plain_text: "a ".repeat(len),
            plain_text_lower: "a ".repeat(len),
            char_count: len,
            viewport_width: None,
            viewport_height: None,
        };
        let results = SearchEngine::search_limited(&[sec], "a", false, Some(42));
        assert_eq!(results.len(), 42);
    }

    /// Snippet extraction must not re-scan the section text once per match: the ASCII-ness of the
    /// text is passed in by the caller.
    #[test]
    fn extract_snippet_accepts_a_precomputed_ascii_flag() {
        // ASCII source -> byte-based window.
        let ascii = format!("{}tail", "b".repeat(100));
        let start = ascii.len() - 4;
        let (before, matched, after, _, _) = extract_zero_alloc_snippet(&ascii, start, 4, true);
        assert_eq!(matched, "tail");
        assert_eq!(after, "");
        assert_eq!(before.len(), 40);

        // Non-ASCII source -> char-based window, and no panic on multi-byte characters.
        let uni = format!("{}tail", "日".repeat(100));
        let start = uni.len() - 4;
        let (before, matched, after, _, _) = extract_zero_alloc_snippet(&uni, start, 4, false);
        assert_eq!(matched, "tail");
        assert_eq!(after, "");
        assert_eq!(before.chars().count(), 40);
    }
}
