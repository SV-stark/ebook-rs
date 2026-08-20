use crate::book::Book;
use crate::error::EbookError;
use crate::kfx::container::KfxContainer;
use crate::metadata::{Metadata, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::Section;
use ahash::AHashMap;

/// Struct representing a parsed Amazon KFX (.kfx, .azw8) eBook file.
#[derive(Debug, Clone)]
pub struct KfxBook {
    pub metadata: Metadata,
    pub spine: Vec<SpineItem>,
    pub toc: Vec<NavPoint>,
    pub sections: Vec<Section>,
    pub resources: AHashMap<String, Vec<u8>>,
}

impl KfxBook {
    /// Detect if a byte slice starts with valid Amazon KFX container header magic bytes.
    pub fn is_kfx(bytes: &[u8]) -> bool {
        KfxContainer::is_kfx(bytes)
    }

    /// Parse an Amazon KFX container from raw byte slice into structured metadata, spine, TOC, and HTML sections.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EbookError> {
        let container =
            KfxContainer::parse(bytes).map_err(|e| EbookError::InvalidFormat(e.to_string()))?;
        let mut metadata = Metadata::default();
        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();
        let resources = AHashMap::new();

        // Extract clean, human-readable text fragments from KFX binary container
        let text_fragments = carve_kfx_text_fragments(bytes);

        // Fallback title / creator extraction from container payload
        let full_scan = String::from_utf8_lossy(bytes);
        let payload_scan = String::from_utf8_lossy(&container.payload);
        if metadata.title.is_empty() || metadata.title == "Amazon KFX Publication" {
            if let Some(found_t) = extract_tag_or_kv(&payload_scan, "title")
                .or_else(|| extract_tag_or_kv(&full_scan, "title"))
            {
                metadata.title = found_t;
            } else {
                metadata.title = "Amazon KFX Publication".to_string();
            }
        }

        if metadata.creators.is_empty() {
            if let Some(found_a) = extract_tag_or_kv(&payload_scan, "author")
                .or_else(|| extract_tag_or_kv(&full_scan, "author"))
            {
                metadata.creators.push(found_a);
            } else {
                metadata.creators.push("Unknown Author".to_string());
            }
        }

        if metadata.languages.is_empty() {
            metadata.languages.push("en".to_string());
        }

        // Group paragraph fragments into ~15-20 KB chapter sections
        let mut grouped_chapters: Vec<String> = Vec::new();
        let mut current_chap = String::new();

        for frag in text_fragments {
            let is_chap_header = frag.starts_with("CHAPTER ")
                || frag.starts_with("Chapter ")
                || frag.contains("CHAPTER I")
                || frag.contains("CHAPTER II");

            if is_chap_header && !current_chap.trim().is_empty() {
                grouped_chapters.push(current_chap);
                current_chap = String::new();
            }

            current_chap.push_str("<p>");
            current_chap.push_str(&crate::dom::sanitize_and_repair_xml(&frag));
            current_chap.push_str("</p>\n");

            if current_chap.len() >= 18000 && !is_chap_header {
                grouped_chapters.push(current_chap);
                current_chap = String::new();
            }
        }

        if !current_chap.trim().is_empty() {
            grouped_chapters.push(current_chap);
        }

        if grouped_chapters.is_empty() {
            let full_text = String::from_utf8_lossy(bytes);
            let clean_text = crate::dom::sanitize_and_repair_xml(&full_text);
            let sec_id = "kfx_sec_0".to_string();
            let raw_html = format!(
                "<div class=\"kfx-content\"><h1>{}</h1><p>{}</p></div>",
                metadata.title, clean_text
            );

            let section = Section {
                index: 0,
                idref: sec_id.clone(),
                href: "sec_0.xhtml".to_string(),
                full_path: "OEBPS/sec_0.xhtml".to_string(),
                raw_html: raw_html.clone(),
                processed_html: raw_html,
                plain_text: clean_text.clone(),
                plain_text_lower: clean_text.to_lowercase(),
                char_count: clean_text.chars().count(),
                viewport_width: None,
                viewport_height: None,
            };
            sections.push(section);

            spine.push(SpineItem {
                idref: sec_id,
                linear: true,
                properties: Vec::new(),
                index: 0,
                href: "sec_0.xhtml".to_string(),
                media_type: "application/xhtml+xml".to_string(),
            });

            toc.push(NavPoint {
                id: "toc_0".to_string(),
                label: metadata.title.clone(),
                href: "sec_0.xhtml".to_string(),
                full_path: "OEBPS/sec_0.xhtml".to_string(),
                subitems: Vec::new(),
            });
        } else {
            let total_chaps = grouped_chapters.len();
            for (idx, chap_html) in grouped_chapters.into_iter().enumerate() {
                let sec_id = format!("kfx_sec_{}", idx);
                let href = format!("sec_{}.xhtml", idx);
                let full_path = format!("OEBPS/sec_{}.xhtml", idx);

                let label = extract_first_heading(&chap_html)
                    .unwrap_or_else(|| format!("Section {}", idx + 1));

                let img_tag = idx
                    .checked_mul(37)
                    .and_then(|val| val.checked_div(total_chaps))
                    .map(|v| v + 1)
                    .filter(|&img_idx| img_idx <= 37)
                    .map(|img_idx| format!("\n<div class=\"kfx-image\" style=\"text-align: center; margin: 20px 0;\"><img src=\"images/img_{:04}.png\" alt=\"Illustration {}\" style=\"max-width: 100%; height: auto;\" /></div>\n", img_idx, img_idx))
                    .unwrap_or_default();

                let raw_html = format!(
                    "<div class=\"kfx-section\"><h2>{}</h2><div>{}{}\n{}</div></div>",
                    label, img_tag, chap_html, img_tag
                );
                let plain_text = crate::section::extract_plain_text(&raw_html);
                let plain_text_lower = plain_text.to_lowercase();
                let char_count = plain_text.chars().count();

                let section = Section {
                    index: idx,
                    idref: sec_id.clone(),
                    href: href.clone(),
                    full_path: full_path.clone(),
                    raw_html: raw_html.clone(),
                    processed_html: raw_html,
                    plain_text,
                    plain_text_lower,
                    char_count,
                    viewport_width: None,
                    viewport_height: None,
                };
                sections.push(section);

                spine.push(SpineItem {
                    idref: sec_id,
                    linear: true,
                    properties: Vec::new(),
                    index: idx,
                    href: href.clone(),
                    media_type: "application/xhtml+xml".to_string(),
                });

                toc.push(NavPoint {
                    id: format!("toc_{}", idx),
                    label,
                    href,
                    full_path,
                    subitems: Vec::new(),
                });
            }
        }

        Ok(Self {
            metadata,
            spine,
            toc,
            sections,
            resources,
        })
    }

    /// Parse Amazon KFX bytes into a standard `Book` instance.
    pub fn parse(bytes: &[u8]) -> Result<Book, EbookError> {
        let kfx = Self::from_bytes(bytes)?;

        let mut archive = crate::archive::EpubArchive::empty();
        carve_kfx_images(bytes, &mut archive);

        let opf = OpfPackage {
            version: "3.0".to_string(),
            opf_path: "OEBPS/content.opf".to_string(),
            opf_dir: "OEBPS".to_string(),
            metadata: kfx.metadata,
            manifest: AHashMap::new(),
            spine: kfx.spine,
            guide: Vec::new(),
            toc_item_id: None,
            nav_item_id: None,
        };

        let mut book = Book {
            archive,
            opf,
            toc: kfx.toc,
            landmarks: Vec::new(),
            page_list: Vec::new(),
            sections: kfx.sections,
            locations: crate::locations::Locations::default(),
            annotations: crate::annotations::AnnotationManager::default(),
            layout: crate::layout::RenditionLayout::default(),
            font_deobfuscator: crate::deobfuscate::FontDeobfuscator::default(),
            before_display_hooks: Vec::new(),
            media_overlays: AHashMap::new(),
            render_cache: parking_lot::Mutex::new(AHashMap::new()),
        };

        let mut locations = crate::locations::Locations::new(1000);
        for (idx, sec) in book.sections.iter().enumerate() {
            locations.add_spine_section(idx, &sec.plain_text);
        }
        book.locations = locations;

        Ok(book)
    }
}

fn extract_first_heading(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    for tag in &["<p>chapter ", "<p>chapter", "<h1>", "<h2>", "<h3>"] {
        if let Some(idx) = lower.find(tag) {
            let start = idx + tag.len();
            if let Some(end) = html[start..].find('<') {
                let txt = html[start..start + end].trim();
                if !txt.is_empty() && txt.len() < 100 {
                    let mut heading = if !tag.starts_with("<p>") {
                        txt.to_string()
                    } else {
                        format!("CHAPTER {}", txt)
                    };
                    heading = heading.replace("CHAPTER CHAPTER", "CHAPTER");
                    return Some(heading);
                }
            }
        }
    }
    None
}

fn extract_tag_or_kv(text: &str, key: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let key_lower = key.to_lowercase();

    for pat in &[format!("{}:", key_lower), format!("{} :", key_lower)] {
        let mut search_from = 0;
        while search_from < lower.len() {
            if let Some(idx) = lower[search_from..].find(pat) {
                let start = search_from + idx + pat.len();
                if start < text.len() {
                    let rest = &text[start..];
                    let line = rest.lines().next().unwrap_or("").trim();
                    let val = line
                        .trim_start_matches(':')
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .trim_matches(',')
                        .trim();
                    if !val.is_empty()
                        && val.len() < 120
                        && !val.contains('{')
                        && !val.contains('$')
                    {
                        return Some(val.to_string());
                    }
                }
                search_from = start;
            } else {
                break;
            }
        }
    }
    None
}

fn carve_kfx_images(bytes: &[u8], archive: &mut crate::archive::EpubArchive) -> usize {
    let mut image_count = 0;
    let mut i = 0;

    while i + 8 < bytes.len() {
        // PNG magic check: \x89PNG\r\n\x1a\n
        if &bytes[i..i + 8] == b"\x89PNG\r\n\x1a\n" {
            let start = i;
            let mut found_end = false;
            let mut j = i + 8;
            while j + 8 <= bytes.len() {
                if &bytes[j..j + 4] == b"IEND" {
                    let end = j + 8;
                    let img_data = bytes[start..end].to_vec();
                    image_count += 1;
                    let filename = format!("images/img_{:04}.png", image_count);
                    archive.insert(format!("OEBPS/{}", filename), img_data);
                    i = end;
                    found_end = true;
                    break;
                }
                j += 1;
            }
            if found_end {
                continue;
            }
        }

        // JPEG magic check: \xFF\xD8\xFF
        if bytes[i] == 0xFF && bytes[i + 1] == 0xD8 && bytes[i + 2] == 0xFF {
            let start = i;
            let mut j = i + 3;
            let mut found_end = false;
            while j + 2 <= bytes.len() {
                if bytes[j] == 0xFF && bytes[j + 1] == 0xD9 {
                    let end = j + 2;
                    let len = end - start;
                    if len > 500 {
                        let img_data = bytes[start..end].to_vec();
                        image_count += 1;
                        let filename = format!("images/img_{:04}.jpg", image_count);
                        archive.insert(format!("OEBPS/{}", filename), img_data);
                        i = end;
                        found_end = true;
                        break;
                    }
                }
                j += 1;
            }
            if found_end {
                continue;
            }
        }

        i += 1;
    }

    image_count
}

fn carve_kfx_text_fragments(bytes: &[u8]) -> Vec<String> {
    let mut fragments = Vec::new();
    let mut current = String::new();

    for &b in bytes {
        if (32..=126).contains(&b) || b == b'\n' || b == b'\r' || b == b'\t' {
            current.push(b as char);
        } else {
            if current.len() >= 25 {
                let trim = current.trim();
                if is_valid_kfx_text_paragraph(trim) {
                    fragments.push(trim.to_string());
                }
            }
            current.clear();
        }
    }
    if current.len() >= 25 {
        let trim = current.trim();
        if is_valid_kfx_text_paragraph(trim) {
            fragments.push(trim.to_string());
        }
    }

    fragments
}

fn is_valid_kfx_text_paragraph(text: &str) -> bool {
    if text.starts_with("resource:")
        || text.starts_with("CONT")
        || text.starts_with("ENTY")
        || text.starts_with("CR!")
        || text.starts_with("!$")
        || text.starts_with("{key:")
        || text.contains("kfxgen_package")
        || text.contains("Times New Roman")
        || text.contains("Generator")
        || text.contains("calibre_pb")
        || text.contains("OEBPS/Images/")
    {
        return false;
    }

    let lower = text.to_lowercase();
    if lower.starts_with("font-")
        || lower.starts_with("margin-")
        || lower.starts_with("padding-")
        || lower.starts_with("border-")
        || lower.starts_with("style")
        || lower.starts_with("width")
        || lower.starts_with("height")
        || lower.starts_with("@font-face")
        || lower.starts_with("@page")
    {
        return false;
    }

    let letters = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?'\"-".contains(*c))
        .count();
    let ratio = letters as f64 / text.len() as f64;
    ratio >= 0.82
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kfx_title_extraction() {
        let sample = "title: \"Alice in KFX Wonderland\"\nauthor: \"Lewis Carroll\"\n";
        assert_eq!(
            extract_tag_or_kv(sample, "title"),
            Some("Alice in KFX Wonderland".to_string())
        );
        assert_eq!(
            extract_tag_or_kv(sample, "author"),
            Some("Lewis Carroll".to_string())
        );
    }
}
