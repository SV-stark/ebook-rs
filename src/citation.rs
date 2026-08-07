use crate::metadata::Metadata;
use serde::{Deserialize, Serialize};

/// Citation format enum for academic export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CitationStyle {
    BibTeX,
    APA,
    MLA,
    Chicago,
}

/// Academic citation exporter engine.
pub struct CitationExporter;

impl CitationExporter {
    /// Format eBook metadata into BibTeX citation string.
    pub fn to_bibtex(metadata: &Metadata) -> String {
        let cite_key = generate_cite_key(metadata);
        let title = if metadata.title.is_empty() {
            "Untitled"
        } else {
            &metadata.title
        };
        let author = if metadata.creator().is_empty() {
            "Unknown Author"
        } else {
            metadata.creator()
        };
        let publisher = metadata.publisher().unwrap_or("Self-Published");
        let year = extract_year(metadata.pub_date.as_deref());

        let mut bib = format!("@book{{{},\n", cite_key);
        bib.push_str(&format!("  title     = {{{}}},\n", title));
        bib.push_str(&format!("  author    = {{{}}},\n", author));
        bib.push_str(&format!("  publisher = {{{}}},\n", publisher));
        if let Some(y) = year {
            bib.push_str(&format!("  year      = {{{}}},\n", y));
        }
        if !metadata.language().is_empty() {
            bib.push_str(&format!("  language  = {{{}}},\n", metadata.language()));
        }
        if let Some(id) = metadata.identifier.as_deref() {
            bib.push_str(&format!("  isbn      = {{{}}}\n", id));
        } else {
            bib.pop(); // remove trailing comma
            bib.push('\n');
        }
        bib.push('}');
        bib
    }

    /// Format eBook metadata into APA (7th ed.) citation string.
    pub fn to_apa(metadata: &Metadata) -> String {
        let author = format_author_apa(metadata.creator());
        let year = extract_year(metadata.pub_date.as_deref())
            .map(|y| format!("({})", y))
            .unwrap_or_else(|| "(n.d.)".to_string());
        let title = if metadata.title.is_empty() {
            "Untitled"
        } else {
            &metadata.title
        };
        let publisher = metadata.publisher().unwrap_or("");

        if publisher.is_empty() {
            format!("{} {}. *{}*.", author, year, title)
        } else {
            format!("{} {}. *{}*. {}.", author, year, title, publisher)
        }
    }

    /// Format eBook metadata into MLA (9th ed.) citation string.
    pub fn to_mla(metadata: &Metadata) -> String {
        let author = if metadata.creator().is_empty() {
            String::new()
        } else {
            format!("{}. ", invert_author_name(metadata.creator()))
        };
        let title = if metadata.title.is_empty() {
            "Untitled"
        } else {
            &metadata.title
        };
        let publisher = metadata
            .publisher()
            .map(|p| format!("{}. ", p))
            .unwrap_or_default();
        let year = extract_year(metadata.pub_date.as_deref())
            .map(|y| format!("{}, ", y))
            .unwrap_or_default();

        format!(
            "{}*{}*. {}{}.",
            author,
            title,
            publisher,
            year.trim_end_matches(", ")
        )
    }

    /// Format eBook metadata into Chicago (17th ed.) citation string.
    pub fn to_chicago(metadata: &Metadata) -> String {
        let author = if metadata.creator().is_empty() {
            "Unknown Author".to_string()
        } else {
            invert_author_name(metadata.creator())
        };
        let title = if metadata.title.is_empty() {
            "Untitled"
        } else {
            &metadata.title
        };
        let publisher = metadata.publisher().unwrap_or("n.p.");
        let year = extract_year(metadata.pub_date.as_deref())
            .map(|y| y.to_string())
            .unwrap_or_else(|| "n.d.".to_string());

        format!(
            "{}. *{}*. {}: {}, {}.",
            author, title, "N.p.", publisher, year
        )
    }
}

fn invert_author_name(creator: &str) -> String {
    let parts: Vec<&str> = creator.split_whitespace().collect();
    if parts.len() >= 2 {
        let last = parts.last().unwrap();
        let firsts = parts[..parts.len() - 1].join(" ");
        format!("{}, {}", last, firsts)
    } else {
        creator.to_string()
    }
}

fn generate_cite_key(metadata: &Metadata) -> String {
    let author_part = metadata
        .creator()
        .split_whitespace()
        .last()
        .unwrap_or("author")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();

    let year_part =
        extract_year(metadata.pub_date.as_deref()).unwrap_or_else(|| "2026".to_string());

    let title_part = metadata
        .title
        .split_whitespace()
        .next()
        .unwrap_or("book")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();

    format!("{}{}{}", author_part, year_part, title_part)
}

fn extract_year(date_str: Option<&str>) -> Option<String> {
    let d = date_str?;
    let digits: String = d.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 4 {
        Some(digits[..4].to_string())
    } else {
        None
    }
}

fn format_author_apa(author: &str) -> String {
    if author.trim().is_empty() {
        return "Unknown Author".to_string();
    }

    let parts: Vec<&str> = author.split_whitespace().collect();
    if parts.len() > 1 {
        let last = parts.last().unwrap();
        let initials: String = parts[..parts.len() - 1]
            .iter()
            .filter_map(|p| p.chars().next())
            .map(|c| format!("{}.", c))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{}, {}", last, initials)
    } else {
        author.to_string()
    }
}
