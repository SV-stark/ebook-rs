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

        // 1. First, attempt to extract structured storylines from container index entries
        let mut storyline_sections: Vec<String> = Vec::new();
        for entry in &container.index_entries {
            if entry.type_id == crate::kfx::symbols::SYM_STORYLINE_FRAGMENT
                || entry.type_id == crate::kfx::symbols::SYM_SECTION_BLOCK
                || entry.type_id == crate::kfx::symbols::SYM_CONTENT_BODY
            {
                if let Some(payload_slice) = container.get_entry_payload(entry) {
                    let sec_str = String::from_utf8_lossy(payload_slice);
                    let clean = sec_str.trim();
                    if !clean.is_empty() && is_valid_kfx_text_paragraph(clean) {
                        storyline_sections.push(clean.to_string());
                    }
                }
            }
        }

        // 2. Extract clean, human-readable text fragments from KFX binary container
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

        let fragments_to_group = if !storyline_sections.is_empty() {
            storyline_sections
        } else {
            text_fragments
        };

        for frag in fragments_to_group {
            let is_chap_header = frag.starts_with("CHAPTER ")
                || frag.starts_with("Chapter ")
                || frag.starts_with("# ")
                || frag.starts_with("## ")
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
            for (idx, chap_html) in grouped_chapters.into_iter().enumerate() {
                let sec_id = format!("kfx_sec_{}", idx);
                let href = format!("sec_{}.xhtml", idx);
                let full_path = format!("OEBPS/sec_{}.xhtml", idx);

                let label = extract_first_heading(&chap_html)
                    .unwrap_or_else(|| format!("Section {}", idx + 1));

                let raw_html = format!(
                    "<div class=\"kfx-section\"><h2>{}</h2><div>\n{}</div></div>",
                    label, chap_html
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

fn find_ascii_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let needle_bytes = needle.as_bytes();
    for (idx, _) in haystack.char_indices() {
        if let Some(sub) = haystack.as_bytes().get(idx..idx + needle_bytes.len()) {
            if sub.eq_ignore_ascii_case(needle_bytes) {
                return Some(idx);
            }
        }
    }
    None
}

fn extract_first_heading(html: &str) -> Option<String> {
    for tag in &["<p>chapter ", "<p>chapter", "<h1>", "<h2>", "<h3>"] {
        if let Some(idx) = find_ascii_ignore_case(html, tag) {
            let start = idx + tag.len();
            if start <= html.len() {
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
    }
    None
}

fn extract_tag_or_kv(text: &str, key: &str) -> Option<String> {
    for pat in &[format!("{}:", key), format!("{} :", key)] {
        let mut search_from = 0;
        while search_from < text.len() {
            if let Some(idx) = find_ascii_ignore_case(&text[search_from..], pat) {
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
            if let Some(iend_pos) = memchr::memmem::find(&bytes[i + 8..], b"IEND") {
                let end = (i + 8 + iend_pos + 8).min(bytes.len());
                let img_data = bytes[start..end].to_vec();
                image_count += 1;
                let filename = format!("images/img_{:04}.png", image_count);
                archive.insert(format!("OEBPS/{}", filename), img_data);
                i = end;
                continue;
            }
        }

        // JPEG magic check: \xFF\xD8\xFF
        if bytes[i] == 0xFF && bytes[i + 1] == 0xD8 && bytes[i + 2] == 0xFF {
            let start = i;
            if let Some(eoi_pos) = memchr::memmem::find(&bytes[i + 3..], b"\xFF\xD9") {
                let end = i + 3 + eoi_pos + 2;
                let len = end - start;
                if len > 500 && len < 20 * 1024 * 1024 {
                    let img_data = bytes[start..end].to_vec();
                    image_count += 1;
                    let filename = format!("images/img_{:04}.jpg", image_count);
                    archive.insert(format!("OEBPS/{}", filename), img_data);
                    i = end;
                    continue;
                }
            }
        }

        i += 1;
    }

    image_count
}

fn carve_kfx_text_fragments(bytes: &[u8]) -> Vec<String> {
    let mut fragments = Vec::new();
    let mut current = String::new();

    let utf8_text = String::from_utf8_lossy(bytes);

    for ch in utf8_text.chars() {
        if ch != '\u{FFFD}' && (!ch.is_control() || ch == '\n' || ch == '\r' || ch == '\t') {
            current.push(ch);
        } else {
            if current.trim().len() >= 4 {
                let trim = current.trim();
                if is_valid_kfx_text_paragraph(trim) {
                    fragments.push(trim.to_string());
                }
            }
            current.clear();
        }
    }
    if current.trim().len() >= 4 {
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
        || text.contains("com.amazon")
        || text.contains("font_family")
        || text.contains("kfx_style")
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
        || lower.starts_with("text-align")
        || lower.starts_with("line-height")
        || lower.starts_with("background-")
        || lower.starts_with("@font-face")
        || lower.starts_with("@page")
    {
        return false;
    }

    let total_chars = text.chars().count();
    if total_chars == 0 {
        return false;
    }

    let valid_count = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .count();
    let ratio = valid_count as f64 / total_chars as f64;
    ratio >= 0.90
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
