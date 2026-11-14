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

        // 1. First, attempt to extract structured storylines and metadata from container index entries
        let mut storyline_sections: Vec<String> = Vec::new();
        let mut ion_content_bodies: Vec<String> = Vec::new();
        for entry in &container.index_entries {
            let Some(payload_slice) = container.get_entry_payload(entry) else {
                continue;
            };
            if container.is_ion_container {
                if let Some(val) = decode_enty_ion_value(payload_slice) {
                    match entry.type_id {
                        crate::kfx::symbols::SYM_BOOK_METADATA => {
                            extract_ion_book_metadata(&val, &mut metadata);
                        }
                        crate::kfx::symbols::SYM_METADATA_RECORD => {
                            extract_ion_legacy_metadata(&val, &mut metadata);
                        }
                        crate::kfx::symbols::SYM_CONTENT_BODY => {
                            extract_ion_content_list(&val, &mut ion_content_bodies);
                        }
                        crate::kfx::symbols::SYM_STORYLINE_FRAGMENT
                        | crate::kfx::symbols::SYM_SECTION_BLOCK => {
                            extract_ion_storyline_text(&val, &mut storyline_sections);
                        }
                        _ => {}
                    }
                }
            } else if entry.type_id == crate::kfx::symbols::SYM_STORYLINE_FRAGMENT
                || entry.type_id == crate::kfx::symbols::SYM_SECTION_BLOCK
                || entry.type_id == crate::kfx::symbols::SYM_CONTENT_BODY
            {
                let sec_str = String::from_utf8_lossy(payload_slice);
                let clean = sec_str.trim();
                if !clean.is_empty() && is_valid_kfx_text_paragraph(clean) {
                    storyline_sections.push(clean.to_string());
                }
            }
        }
        if !ion_content_bodies.is_empty() {
            storyline_sections = ion_content_bodies;
        }

        // 2. For legacy non-Ion containers, carve human-readable text fragments if needed
        let text_fragments = if container.is_ion_container {
            Vec::new()
        } else {
            carve_kfx_text_fragments(bytes)
        };

        // Fallback title / creator extraction from container payload (non-Ion containers only)
        if !container.is_ion_container {
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
        } else {
            if metadata.title.is_empty() {
                metadata.title = "Amazon KFX Publication".to_string();
            }
            if metadata.creators.is_empty() {
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
            let clean_text = if container.is_ion_container {
                String::new()
            } else {
                let full_text = String::from_utf8_lossy(bytes);
                crate::dom::sanitize_and_repair_xml(&full_text)
            };
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
            lcp_passphrase: None,
            lcp_license: None,
            before_display_hooks: Vec::new(),
            media_overlays: AHashMap::new(),
            render_cache: Default::default(),
            hydrated_cache: Default::default(),
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
        if let Some(sub) = haystack.as_bytes().get(idx..idx + needle_bytes.len())
            && sub.eq_ignore_ascii_case(needle_bytes)
        {
            return Some(idx);
        }
    }
    None
}

fn extract_first_heading(html: &str) -> Option<String> {
    for tag in &["<p>chapter ", "<p>chapter", "<h1>", "<h2>", "<h3>"] {
        if let Some(idx) = find_ascii_ignore_case(html, tag) {
            let start = idx + tag.len();
            if start <= html.len()
                && let Some(end) = html[start..].find('<')
            {
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

fn decode_enty_ion_value(payload: &[u8]) -> Option<crate::kfx::ion::IonValue> {
    let ion_slice = if payload.len() >= 12 && &payload[..4] == b"ENTY" {
        let hdr_len = u16::from_le_bytes([payload[6], payload[7]]) as usize;
        if hdr_len > payload.len() {
            return None;
        }
        &payload[hdr_len..]
    } else {
        payload
    };
    let values = crate::kfx::ion::decode_stream(ion_slice).ok()?;
    for val in values {
        // Skip local symbol table (`$ion_symbol_table` annotation ID 3)
        if let crate::kfx::ion::IonValue::Annotated(annots, _) = &val
            && annots.contains(&3)
        {
            continue;
        }
        return Some(val);
    }
    None
}

fn extract_ion_book_metadata(val: &crate::kfx::ion::IonValue, metadata: &mut Metadata) {
    // 491 = categorised_metadata; 258 = metadata (or 492 in simplified fixtures);
    // 492/493 = key, 493/494 = value
    let Some(cats) = val.get_field(491).and_then(|v| v.as_list()) else {
        return;
    };
    for cat in cats {
        let (entries, key_sym, val_sym) =
            if let Some(list) = cat.get_field(258).and_then(|v| v.as_list()) {
                (list, 492, 307)
            } else if let Some(list) = cat.get_field(492).and_then(|v| v.as_list()) {
                (list, 493, 494)
            } else {
                continue;
            };
        for item in entries {
            let Some(key) = item.get_field(key_sym).and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(val_str) = item.get_field(val_sym).and_then(|v| v.as_str()) else {
                continue;
            };
            let trimmed = val_str.trim();
            if trimmed.is_empty() {
                continue;
            }
            match key {
                "title" if metadata.title.is_empty() => metadata.title = trimmed.to_string(),
                "author" => metadata.creators.push(trimmed.to_string()),
                "publisher" => metadata.publishers.push(trimmed.to_string()),
                "language" => metadata.languages.push(trimmed.to_string()),
                "description" if metadata.description.is_none() => {
                    metadata.description = Some(trimmed.to_string());
                }
                _ => {}
            }
        }
    }
}

fn extract_ion_legacy_metadata(val: &crate::kfx::ion::IonValue, metadata: &mut Metadata) {
    if metadata.title.is_empty()
        && let Some(t) = val.get_field(153).and_then(|v| v.as_str())
        && !t.trim().is_empty()
    {
        metadata.title = t.trim().to_string();
    }
    if let Some(a) = val.get_field(222).and_then(|v| v.as_str())
        && !a.trim().is_empty()
    {
        metadata.creators.push(a.trim().to_string());
    }
    if let Some(p) = val.get_field(232).and_then(|v| v.as_str())
        && !p.trim().is_empty()
    {
        metadata.publishers.push(p.trim().to_string());
    }
    if let Some(l) = val.get_field(161).and_then(|v| v.as_str())
        && !l.trim().is_empty()
    {
        metadata.languages.push(l.trim().to_string());
    }
}

fn extract_ion_content_list(val: &crate::kfx::ion::IonValue, out: &mut Vec<String>) {
    // 146 = content_list
    if let Some(list) = val.get_field(146).and_then(|v| v.as_list()) {
        for item in list {
            if let Some(s) = item.as_str() {
                let t = s.trim();
                if !t.is_empty() {
                    out.push(t.to_string());
                }
            }
        }
    }
}

fn extract_ion_storyline_text(val: &crate::kfx::ion::IonValue, out: &mut Vec<String>) {
    use crate::kfx::ion::IonValue;
    match val {
        IonValue::Struct(fields) => {
            for (k, v) in fields {
                // 145 = content, 146 = content_list, 176 = content_chunks
                if *k == 145 || *k == 146 || *k == 176 {
                    if let Some(s) = v.as_str() {
                        let t = s.trim();
                        if !t.is_empty() {
                            out.push(t.to_string());
                        }
                    } else {
                        extract_ion_storyline_text(v, out);
                    }
                } else if matches!(
                    v,
                    IonValue::List(_) | IonValue::Struct(_) | IonValue::Annotated(_, _)
                ) {
                    extract_ion_storyline_text(v, out);
                }
            }
        }
        IonValue::List(items) => {
            for item in items {
                extract_ion_storyline_text(item, out);
            }
        }
        IonValue::Annotated(_, inner) => extract_ion_storyline_text(inner, out),
        _ => {}
    }
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

    #[test]
    fn test_kfx_ion_container_ignores_symbol_table_and_reads_metadata() {
        use crate::kfx::ion::{IonValue, encode_stream};
        use crate::kfx::symbols::*;

        // 1. doc_symbol stream containing YJ_symbols + internal symbol names that must NOT leak as text
        let doc_sym_bytes = encode_stream(&[IonValue::Annotated(
            vec![3],
            Box::new(IonValue::Struct(vec![(
                7,
                IonValue::List(vec![
                    IonValue::String("YJ_symbols".to_string()),
                    IonValue::String("c0-ad".to_string()),
                    IonValue::String("c1AJ".to_string()),
                ]),
            )])),
        )]);

        // 2. ENTY payloads: $490 book_metadata and $145 content
        let mut ent_meta = b"ENTY\x01\x00\x0c\x00\xea\x01\x00\x00".to_vec();
        ent_meta.extend_from_slice(&encode_stream(&[IonValue::Struct(vec![(
            491,
            IonValue::List(vec![IonValue::Struct(vec![(
                492,
                IonValue::List(vec![
                    IonValue::Struct(vec![
                        (493, IonValue::String("title".to_string())),
                        (494, IonValue::String("KFX input fixture".to_string())),
                    ]),
                    IonValue::Struct(vec![
                        (493, IonValue::String("author".to_string())),
                        (494, IonValue::String("Example Author".to_string())),
                    ]),
                ]),
            )])]),
        )])]));

        let mut ent_content = b"ENTY\x01\x00\x0c\x00\x91\x00\x00\x00".to_vec();
        ent_content.extend_from_slice(&encode_stream(&[IonValue::Struct(vec![(
            146,
            IonValue::List(vec![IonValue::String(
                "KFX input fixture text.".to_string(),
            )]),
        )])]));

        // Build container_info + index_table
        let info_off = 18usize;
        // Placeholder container_info to measure length, then patch offsets
        let make_info = |idx_off: u64, idx_len: u64, sym_off: u64, sym_len: u64| {
            encode_stream(&[IonValue::Struct(vec![
                (SYM_INDEX_TABLE_OFFSET as u64, IonValue::Int(idx_off as i64)),
                (SYM_INDEX_TABLE_LENGTH as u64, IonValue::Int(idx_len as i64)),
                (SYM_DOC_SYMBOL_OFFSET as u64, IonValue::Int(sym_off as i64)),
                (SYM_DOC_SYMBOL_LENGTH as u64, IonValue::Int(sym_len as i64)),
            ])])
        };
        let dummy_info = make_info(100, 48, 100, doc_sym_bytes.len() as u64);
        let sym_off = info_off + dummy_info.len();
        let idx_off = sym_off + doc_sym_bytes.len();
        let idx_len = 2 * 24;
        let info_bytes = make_info(
            idx_off as u64,
            idx_len as u64,
            sym_off as u64,
            doc_sym_bytes.len() as u64,
        );
        assert_eq!(info_bytes.len(), dummy_info.len());
        let header_len = idx_off + idx_len;

        let mut kfx = Vec::new();
        kfx.extend_from_slice(b"CONT");
        kfx.extend_from_slice(&2u16.to_le_bytes());
        kfx.extend_from_slice(&(header_len as u32).to_le_bytes());
        kfx.extend_from_slice(&(info_off as u32).to_le_bytes());
        kfx.extend_from_slice(&(info_bytes.len() as u32).to_le_bytes());
        kfx.extend_from_slice(&info_bytes);
        kfx.extend_from_slice(&doc_sym_bytes);

        // Index entry 0: book_metadata (490)
        kfx.extend_from_slice(&1u32.to_le_bytes());
        kfx.extend_from_slice(&SYM_BOOK_METADATA.to_le_bytes());
        kfx.extend_from_slice(&0u64.to_le_bytes());
        kfx.extend_from_slice(&(ent_meta.len() as u64).to_le_bytes());

        // Index entry 1: content (145)
        kfx.extend_from_slice(&2u32.to_le_bytes());
        kfx.extend_from_slice(&SYM_CONTENT_BODY.to_le_bytes());
        kfx.extend_from_slice(&(ent_meta.len() as u64).to_le_bytes());
        kfx.extend_from_slice(&(ent_content.len() as u64).to_le_bytes());

        kfx.extend_from_slice(&ent_meta);
        kfx.extend_from_slice(&ent_content);

        let book = KfxBook::from_bytes(&kfx).unwrap();
        assert_eq!(book.metadata.title, "KFX input fixture");
        assert_eq!(book.metadata.creators, vec!["Example Author"]);
        let all_text: String = book
            .sections
            .iter()
            .map(|s| s.plain_text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(all_text.contains("KFX input fixture text."));
        assert!(!all_text.contains("YJ_symbols"));
        assert!(!all_text.contains("c0-ad"));
    }
}
