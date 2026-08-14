use serde::{Deserialize, Serialize};

/// Structural NLP Reading Analytics for a chapter or section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadingAnalytics {
    pub word_count: usize,
    pub reading_time_minutes: f32,
    pub difficulty_score: f32,
    pub top_keywords: Vec<(String, usize)>,
}

const STOP_WORDS: &[&str] = &[
    "the", "and", "is", "of", "to", "in", "that", "it", "with", "for", "as", "was", "on", "are",
    "by", "at", "an", "be", "this", "which", "from", "or", "have", "had", "has", "not", "but",
    "what", "all", "were", "when", "we", "there", "can", "an", "your", "how", "her", "him", "his",
    "them", "their", "into", "some", "than", "then", "now", "only", "other", "its", "also", "out",
];

impl ReadingAnalytics {
    /// Calculate structural analytics and top keywords from plain text.
    pub fn analyze_text(text: &str) -> Self {
        let words: Vec<&str> = text
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .collect();

        let word_count = words.len();
        let reading_time_minutes = if word_count == 0 {
            0.0
        } else {
            (word_count as f32 / 200.0 * 10.0).round() / 10.0
        };

        // Compute top frequency keywords (excluding common stopwords)
        let mut freq_map = ahash::AHashMap::new();
        let mut total_chars = 0;

        for word in &words {
            let lower = word.to_lowercase();
            total_chars += lower.chars().count();
            if lower.len() >= 3 && !STOP_WORDS.contains(&lower.as_str()) {
                *freq_map.entry(lower).or_insert(0usize) += 1;
            }
        }

        let mut top_keywords: Vec<(String, usize)> = freq_map.into_iter().collect();
        top_keywords.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        top_keywords.truncate(10);

        let avg_word_length = if word_count == 0 {
            0.0
        } else {
            total_chars as f32 / word_count as f32
        };

        // Flesch-Kincaid style difficulty score indicator (0.0 easy to 10.0 complex)
        let difficulty_score = (avg_word_length * 1.5).min(10.0);
        let difficulty_score = (difficulty_score * 10.0).round() / 10.0;

        Self {
            word_count,
            reading_time_minutes,
            difficulty_score,
            top_keywords,
        }
    }
}
