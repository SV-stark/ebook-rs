use crate::book::Book;
use crate::dom::xml_escape;
use crate::error::EbookError;
use crate::kfx::container::KfxContainer;
use crate::kfx::ion::IonValue;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::Section;
use ahash::{AHashMap, AHashSet};

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
        let mut resources = AHashMap::new();

        // 1. First, attempt to extract structured entities and metadata from container index entries
        let mut storyline_sections: Vec<String> = Vec::new();
        let mut ion_content_bodies: Vec<String> = Vec::new();

        let mut sections_map: AHashMap<u32, IonValue> = AHashMap::new();
        let mut storylines_map: AHashMap<u32, IonValue> = AHashMap::new();
        let mut content_map: AHashMap<u32, Vec<String>> = AHashMap::new();
        let mut anchors_map: AHashMap<u32, IonValue> = AHashMap::new();
        let mut ext_res_map: AHashMap<u32, IonValue> = AHashMap::new();
        let mut raw_media_map: AHashMap<String, Vec<u8>> = AHashMap::new();
        let mut fonts_map: AHashMap<u32, IonValue> = AHashMap::new();
        let mut raw_fonts_map: AHashMap<String, Vec<u8>> = AHashMap::new();
        let mut styles_map: AHashMap<u32, IonValue> = AHashMap::new();
        let mut cover_res_name: Option<String> = None;
        let mut nav_containers_map: AHashMap<u32, IonValue> = AHashMap::new();
        let mut nav_toc_val: Option<IonValue> = None;
        let mut reading_order_secs: Vec<u32> = Vec::new();

        for entry in &container.index_entries {
            let Some(payload_slice) = container.get_entry_payload(entry) else {
                continue;
            };
            if container.is_ion_container {
                if entry.type_id == 417
                    || entry.type_id == crate::kfx::symbols::SYM_RAW_MEDIA_RESOURCE
                {
                    if let Some(raw) = strip_enty_header(payload_slice)
                        && let Some(sym_name) = resolve_local_symbol(&container, entry.entity_id)
                    {
                        raw_media_map.insert(sym_name, raw.to_vec());
                    }
                    continue;
                }
                if entry.type_id == 418 {
                    if let Some(raw) = strip_enty_header(payload_slice)
                        && let Some(sym_name) = resolve_local_symbol(&container, entry.entity_id)
                    {
                        raw_fonts_map.insert(sym_name, raw.to_vec());
                    }
                    continue;
                }
                if let Some(val) = decode_enty_ion_value(payload_slice) {
                    match entry.type_id {
                        crate::kfx::symbols::SYM_BOOK_METADATA => {
                            extract_ion_book_metadata(&val, &mut metadata, &mut cover_res_name);
                        }
                        crate::kfx::symbols::SYM_METADATA_RECORD => {
                            extract_ion_legacy_metadata(&val, &mut metadata);
                            if reading_order_secs.is_empty() {
                                extract_reading_order_sections(&val, &mut reading_order_secs);
                            }
                        }
                        crate::kfx::symbols::SYM_DOCUMENT_DATA => {
                            if let Some(dir_sym) = val.get_field(192).and_then(|v| v.as_symbol()) {
                                if dir_sym == 375 {
                                    metadata.direction = PageProgressionDirection::Rtl;
                                } else if dir_sym == 376 {
                                    metadata.direction = PageProgressionDirection::Ltr;
                                }
                            }
                            let mut secs = Vec::new();
                            extract_reading_order_sections(&val, &mut secs);
                            if !secs.is_empty() {
                                reading_order_secs = secs;
                            }
                        }
                        crate::kfx::symbols::SYM_CONTENT_BODY => {
                            extract_ion_content_list(&val, &mut ion_content_bodies);
                            if let Some(list) = val.get_field(146).and_then(|v| v.as_list()) {
                                let strings = list
                                    .iter()
                                    .map(|v| v.as_str().unwrap_or("").to_string())
                                    .collect();
                                content_map.insert(entry.entity_id, strings);
                            }
                        }
                        crate::kfx::symbols::SYM_STORYLINE_FRAGMENT => {
                            extract_ion_storyline_text(&val, &mut storyline_sections);
                            storylines_map.insert(entry.entity_id, val);
                        }
                        crate::kfx::symbols::SYM_SECTION_BLOCK => {
                            extract_ion_storyline_text(&val, &mut storyline_sections);
                            sections_map.insert(entry.entity_id, val);
                        }
                        crate::kfx::symbols::SYM_NAVIGATION_TOC => {
                            nav_toc_val = Some(val);
                        }
                        391 => {
                            nav_containers_map.insert(entry.entity_id, val);
                        }
                        266 => {
                            anchors_map.insert(entry.entity_id, val);
                        }
                        164 => {
                            ext_res_map.insert(entry.entity_id, val);
                        }
                        157 => {
                            styles_map.insert(entry.entity_id, val);
                        }
                        262 => {
                            fonts_map.insert(entry.entity_id, val);
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

        // 3. Structured Ion section + storyline + resource + TOC reconstruction
        if container.is_ion_container && !reading_order_secs.is_empty() && !sections_map.is_empty()
        {
            let mut img_rel_paths: AHashMap<u32, String> = AHashMap::new();
            let mut img_counter = 0usize;
            let mut sorted_ext: Vec<(u32, &IonValue)> =
                ext_res_map.iter().map(|(&k, v)| (k, v)).collect();
            sorted_ext.sort_by_key(|(k, _)| *k);
            for (ext_sym, ext_val) in sorted_ext {
                let loc = ext_val.get_field(165).and_then(|v| v.as_str());
                let mime = ext_val
                    .get_field(162)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let fmt_sym = ext_val.get_field(161).and_then(|v| v.as_symbol());
                let sym_name = resolve_local_symbol(&container, ext_sym);
                let raw_data = loc
                    .and_then(|l| raw_media_map.get(l))
                    .or_else(|| sym_name.as_deref().and_then(|s| raw_media_map.get(s)));
                if let Some(data) = raw_data {
                    let ext = image_ext_from_meta(mime, fmt_sym, data);
                    let rel = format!("images/img_{img_counter:04}.{ext}");
                    let full_path = format!("OEBPS/{rel}");
                    if cover_res_name.as_deref() == sym_name.as_deref() && sym_name.is_some() {
                        metadata.cover_id = Some(format!("kfx_img_{img_counter:04}"));
                        metadata.cover_href = Some(full_path.clone());
                    }
                    img_counter += 1;
                    resources.insert(full_path, data.clone());
                    img_rel_paths.insert(ext_sym, rel);
                }
            }

            // Extract embedded fonts ($262 + $418)
            let mut font_counter = 0usize;
            let mut used_font_locs: AHashSet<String> = AHashSet::new();
            let mut sorted_fonts: Vec<(u32, &IonValue)> =
                fonts_map.iter().map(|(&k, v)| (k, v)).collect();
            sorted_fonts.sort_by_key(|(k, _)| *k);
            for (_, font_val) in sorted_fonts {
                let Some(loc) = font_val.get_field(165).and_then(|v| v.as_str()) else {
                    continue;
                };
                if !used_font_locs.insert(loc.to_string()) {
                    continue;
                }
                if let Some(data) = raw_fonts_map.get(loc) {
                    let fname = loc.rsplit('/').next().unwrap_or(loc);
                    let safe_fname = if fname.contains('.') {
                        fname.to_string()
                    } else {
                        format!("font_{font_counter:04}.otf")
                    };
                    font_counter += 1;
                    resources.insert(format!("OEBPS/fonts/{safe_fname}"), data.clone());
                }
            }
            let mut sorted_raw_fonts: Vec<(&String, &Vec<u8>)> = raw_fonts_map.iter().collect();
            sorted_raw_fonts.sort_by_key(|(k, _)| *k);
            for (loc, data) in sorted_raw_fonts {
                if used_font_locs.contains(loc.as_str()) {
                    continue;
                }
                let fname = loc.rsplit('/').next().unwrap_or(loc.as_str());
                font_counter += 1;
                resources.insert(
                    format!("OEBPS/fonts/{font_counter:04}_{fname}"),
                    data.clone(),
                );
            }

            // Pre-collect EID -> section_idx across all sections so cross-chapter links resolve cleanly
            let mut eid_to_sec_idx: AHashMap<i64, usize> = AHashMap::new();
            let mut ordered_story_syms: Vec<Vec<u32>> =
                Vec::with_capacity(reading_order_secs.len());
            for (s_idx, sec_sym) in reading_order_secs.iter().enumerate() {
                let mut stories = Vec::new();
                if let Some(sec_val) = sections_map.get(sec_sym) {
                    collect_section_stories_and_eids(
                        sec_val,
                        s_idx,
                        &mut stories,
                        &mut eid_to_sec_idx,
                    );
                    for st_sym in &stories {
                        if let Some(st_val) = storylines_map.get(st_sym) {
                            collect_storyline_eids(st_val, s_idx, &mut eid_to_sec_idx);
                        }
                    }
                }
                ordered_story_syms.push(stories);
            }

            let mut target_eids_needed: AHashSet<i64> = AHashSet::new();
            for anc_val in anchors_map.values() {
                if let Some(pos) = anc_val.get_field(183)
                    && let Some(eid) = pos.get_field(155).and_then(|v| v.as_i64())
                {
                    target_eids_needed.insert(eid);
                }
            }

            let ctx = StructuredRenderCtx {
                content_map: &content_map,
                storylines_map: &storylines_map,
                anchors_map: &anchors_map,
                img_rel_paths: &img_rel_paths,
                styles_map: &styles_map,
                eid_to_sec_idx: &eid_to_sec_idx,
                target_eids_needed: &target_eids_needed,
            };

            for (idx, stories) in ordered_story_syms.iter().enumerate() {
                let mut body_html = String::new();
                for st_sym in stories {
                    if let Some(st_val) = storylines_map.get(st_sym)
                        && let Some(blocks) = st_val.get_field(146).and_then(|v| v.as_list())
                    {
                        for blk in blocks {
                            render_kfx_block(blk, &ctx, &mut body_html);
                        }
                    }
                }

                let sec_dir = reading_order_secs
                    .get(idx)
                    .and_then(|s| sections_map.get(s))
                    .and_then(|sv| sv.get_field(141))
                    .and_then(|v| v.as_list())
                    .and_then(|l| l.first())
                    .and_then(|pt| {
                        style_dir_info(pt.get_field(157).and_then(|v| v.as_symbol()), &styles_map)
                    })
                    .map(|(d, _)| d)
                    .or_else(|| {
                        if metadata.direction == PageProgressionDirection::Rtl {
                            Some("rtl")
                        } else {
                            None
                        }
                    });

                let sec_id = format!("kfx_sec_{idx}");
                let href = format!("sec_{idx}.xhtml");
                let full_path = format!("OEBPS/sec_{idx}.xhtml");
                let dir_attr = sec_dir.map(|d| format!(" dir=\"{d}\"")).unwrap_or_default();
                let raw_html = format!("<div class=\"kfx-section\"{dir_attr}>\n{body_html}</div>");
                let plain_text = crate::section::extract_plain_text(&raw_html);
                let plain_text_lower = plain_text.to_lowercase();
                let char_count = plain_text.chars().count();

                sections.push(Section {
                    index: idx,
                    idref: sec_id.clone(),
                    href: href.clone(),
                    full_path,
                    raw_html: raw_html.clone(),
                    processed_html: raw_html,
                    plain_text,
                    plain_text_lower,
                    char_count,
                    viewport_width: None,
                    viewport_height: None,
                });

                spine.push(SpineItem {
                    idref: sec_id,
                    linear: true,
                    properties: Vec::new(),
                    index: idx,
                    href,
                    media_type: "application/xhtml+xml".to_string(),
                });
            }

            if let Some(nav_val) = &nav_toc_val {
                let mut nav_counter = 0usize;
                toc = parse_kfx_toc(
                    nav_val,
                    &nav_containers_map,
                    &eid_to_sec_idx,
                    &mut nav_counter,
                );
            }
            if toc.is_empty() {
                for (idx, sp) in spine.iter().enumerate() {
                    let label = extract_first_heading(&sections[idx].raw_html)
                        .unwrap_or_else(|| format!("Section {}", idx + 1));
                    toc.push(NavPoint {
                        id: format!("toc_{idx}"),
                        label,
                        href: sp.href.clone(),
                        full_path: format!("OEBPS/{}", sp.href),
                        subitems: Vec::new(),
                    });
                }
            }

            return Ok(Self {
                metadata,
                spine,
                toc,
                sections,
                resources,
            });
        }

        // Group paragraph fragments into ~15-20 KB chapter sections (fallback for simplified/legacy KFX)
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
        if !kfx.resources.is_empty() {
            for (path, data) in &kfx.resources {
                archive.insert(path.clone(), data.clone());
            }
        } else {
            carve_kfx_images(bytes, &mut archive);
        }

        let mut manifest = AHashMap::new();
        let mut res_paths: Vec<String> = kfx.resources.keys().cloned().collect();
        res_paths.sort();
        for (idx, full_path) in res_paths.into_iter().enumerate() {
            let href = full_path
                .strip_prefix("OEBPS/")
                .unwrap_or(&full_path)
                .to_string();
            let lower = full_path.to_ascii_lowercase();
            let media_type = if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                "image/jpeg"
            } else if lower.ends_with(".png") {
                "image/png"
            } else if lower.ends_with(".gif") {
                "image/gif"
            } else if lower.ends_with(".svg") {
                "image/svg+xml"
            } else if lower.ends_with(".webp") {
                "image/webp"
            } else if lower.ends_with(".otf") {
                "font/otf"
            } else if lower.ends_with(".ttf") {
                "font/ttf"
            } else if lower.ends_with(".woff2") {
                "font/woff2"
            } else if lower.ends_with(".woff") {
                "font/woff"
            } else {
                "application/octet-stream"
            };
            let is_cover = kfx.metadata.cover_href.as_deref() == Some(full_path.as_str());
            let id = if is_cover {
                kfx.metadata
                    .cover_id
                    .clone()
                    .unwrap_or_else(|| format!("kfx_res_{idx}"))
            } else {
                format!("kfx_res_{idx}")
            };
            let properties = if is_cover {
                vec!["cover-image".to_string()]
            } else {
                Vec::new()
            };
            manifest.insert(
                id.clone(),
                crate::ManifestItem {
                    id,
                    href,
                    full_path,
                    media_type: media_type.to_string(),
                    properties,
                    fallback: None,
                    media_overlay: None,
                },
            );
        }

        let opf = OpfPackage {
            version: "3.0".to_string(),
            opf_path: "OEBPS/content.opf".to_string(),
            opf_dir: "OEBPS".to_string(),
            metadata: kfx.metadata,
            manifest,
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

struct StructuredRenderCtx<'a> {
    content_map: &'a AHashMap<u32, Vec<String>>,
    storylines_map: &'a AHashMap<u32, IonValue>,
    anchors_map: &'a AHashMap<u32, IonValue>,
    img_rel_paths: &'a AHashMap<u32, String>,
    styles_map: &'a AHashMap<u32, IonValue>,
    eid_to_sec_idx: &'a AHashMap<i64, usize>,
    target_eids_needed: &'a AHashSet<i64>,
}

fn style_dir_info(
    style_sym: Option<u64>,
    styles_map: &AHashMap<u32, IonValue>,
) -> Option<(&'static str, &'static str)> {
    let sym_u32 = u32::try_from(style_sym?).ok()?;
    let style_val = styles_map.get(&sym_u32)?;
    let dir_sym = style_val.get_field(682).and_then(|v| v.as_symbol())?;
    let dir = match dir_sym {
        375 => "rtl",
        376 => "ltr",
        _ => return None,
    };
    let bidi_sym = style_val.get_field(674).and_then(|v| v.as_symbol());
    let tag = match bidi_sym {
        Some(676) => "bdi",
        Some(677 | 678) => "bdo",
        _ => "span",
    };
    Some((dir, tag))
}

fn resolve_local_symbol(container: &KfxContainer, sym_id: u32) -> Option<String> {
    const YJ_MAX_ID: usize = 851;
    let idx = (sym_id as usize).checked_sub(YJ_MAX_ID + 1)?;
    container.doc_symbols.get(idx).cloned()
}

fn strip_enty_header(payload: &[u8]) -> Option<&[u8]> {
    if payload.len() >= 12 && &payload[..4] == b"ENTY" {
        let hdr_len = u16::from_le_bytes([payload[6], payload[7]]) as usize;
        if hdr_len <= payload.len() {
            return Some(&payload[hdr_len..]);
        }
        return None;
    }
    Some(payload)
}

fn image_ext_from_meta(mime: &str, fmt_sym: Option<u64>, data: &[u8]) -> &'static str {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") || fmt_sym == Some(284) || mime == "image/png" {
        "png"
    } else if (data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF)
        || fmt_sym == Some(285)
        || mime == "image/jpeg"
    {
        "jpg"
    } else if data.starts_with(b"GIF8") || fmt_sym == Some(286) || mime == "image/gif" {
        "gif"
    } else if data.starts_with(b"BM") || fmt_sym == Some(599) || mime == "image/bmp" {
        "bmp"
    } else if mime == "image/svg+xml" {
        "svg"
    } else if mime == "image/webp" {
        "webp"
    } else {
        "bin"
    }
}

fn extract_reading_order_sections(val: &IonValue, out: &mut Vec<u32>) {
    let Some(orders) = val.get_field(169).and_then(|v| v.as_list()) else {
        return;
    };
    for order in orders {
        if let Some(secs) = order.get_field(170).and_then(|v| v.as_list()) {
            for s in secs {
                if let Some(sym_id) = s.as_symbol()
                    && let Ok(u) = u32::try_from(sym_id)
                {
                    out.push(u);
                }
            }
            if !out.is_empty() {
                return;
            }
        }
    }
}

fn collect_section_stories_and_eids(
    val: &IonValue,
    sec_idx: usize,
    stories: &mut Vec<u32>,
    eid_to_sec_idx: &mut AHashMap<i64, usize>,
) {
    match val {
        IonValue::Struct(fields) => {
            for (k, v) in fields {
                if *k == 155
                    && let Some(eid) = v.as_i64()
                {
                    eid_to_sec_idx.insert(eid, sec_idx);
                } else if *k == 176
                    && let Some(sym) = v.as_symbol()
                    && let Ok(u) = u32::try_from(sym)
                    && !stories.contains(&u)
                {
                    stories.push(u);
                } else {
                    collect_section_stories_and_eids(v, sec_idx, stories, eid_to_sec_idx);
                }
            }
        }
        IonValue::List(items) => {
            for item in items {
                collect_section_stories_and_eids(item, sec_idx, stories, eid_to_sec_idx);
            }
        }
        IonValue::Annotated(_, inner) => {
            collect_section_stories_and_eids(inner, sec_idx, stories, eid_to_sec_idx);
        }
        _ => {}
    }
}

fn collect_storyline_eids(
    val: &IonValue,
    sec_idx: usize,
    eid_to_sec_idx: &mut AHashMap<i64, usize>,
) {
    match val {
        IonValue::Struct(fields) => {
            for (k, v) in fields {
                if *k == 155
                    && let Some(eid) = v.as_i64()
                {
                    eid_to_sec_idx.insert(eid, sec_idx);
                } else {
                    collect_storyline_eids(v, sec_idx, eid_to_sec_idx);
                }
            }
        }
        IonValue::List(items) => {
            for item in items {
                collect_storyline_eids(item, sec_idx, eid_to_sec_idx);
            }
        }
        IonValue::Annotated(_, inner) => collect_storyline_eids(inner, sec_idx, eid_to_sec_idx),
        _ => {}
    }
}

fn resolve_anchor_href(anc_sym: u64, ctx: &StructuredRenderCtx<'_>) -> Option<String> {
    let anc_id = u32::try_from(anc_sym).ok()?;
    let anc = ctx.anchors_map.get(&anc_id)?;
    if let Some(uri) = anc.get_field(186).and_then(|v| v.as_str()) {
        return Some(uri.to_string());
    }
    if let Some(pos) = anc.get_field(183)
        && let Some(eid) = pos.get_field(155).and_then(|v| v.as_i64())
        && let Some(&target_sec) = ctx.eid_to_sec_idx.get(&eid)
    {
        return Some(format!("sec_{target_sec}.xhtml#eid_{eid}"));
    }
    None
}

fn render_text_with_style_events(
    text: &str,
    blk: &IonValue,
    ctx: &StructuredRenderCtx<'_>,
    out: &mut String,
) {
    let Some(events) = blk.get_field(142).and_then(|v| v.as_list()) else {
        out.push_str(&xml_escape(text));
        return;
    };
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    struct InlineSpan {
        start: usize,
        end: usize,
        href: Option<String>,
        dir: Option<(&'static str, &'static str)>,
    }
    let mut spans: Vec<InlineSpan> = Vec::new();
    for ev in events {
        let Some(off) = ev.get_field(143).and_then(|v| v.as_i64()) else {
            continue;
        };
        let Some(len) = ev.get_field(144).and_then(|v| v.as_i64()) else {
            continue;
        };
        if off < 0 || len <= 0 {
            continue;
        }
        let start = (off as usize).min(total);
        let end = (start + len as usize).min(total);
        if end <= start {
            continue;
        }
        let href = ev
            .get_field(179)
            .and_then(|v| v.as_symbol())
            .and_then(|anc_sym| resolve_anchor_href(anc_sym, ctx));
        let dir = style_dir_info(
            ev.get_field(157).and_then(|v| v.as_symbol()),
            ctx.styles_map,
        );
        if href.is_some() || dir.is_some() {
            spans.push(InlineSpan {
                start,
                end,
                href,
                dir,
            });
        }
    }
    if spans.is_empty() {
        out.push_str(&xml_escape(text));
        return;
    }
    spans.sort_by_key(|s| (s.start, s.end));
    let mut cursor = 0usize;
    for sp in spans {
        if sp.start < cursor {
            continue;
        }
        if sp.start > cursor {
            let plain: String = chars[cursor..sp.start].iter().collect();
            out.push_str(&xml_escape(&plain));
        }
        let seg: String = chars[sp.start..sp.end].iter().collect();
        if let Some((d, tag)) = sp.dir {
            out.push('<');
            out.push_str(tag);
            out.push_str(" dir=\"");
            out.push_str(d);
            out.push_str("\">");
        }
        if let Some(href) = &sp.href {
            out.push_str("<a href=\"");
            out.push_str(&xml_escape(href));
            out.push_str("\">");
        }
        out.push_str(&xml_escape(&seg));
        if sp.href.is_some() {
            out.push_str("</a>");
        }
        if let Some((_, tag)) = sp.dir {
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
        cursor = sp.end;
    }
    if cursor < total {
        let tail: String = chars[cursor..total].iter().collect();
        out.push_str(&xml_escape(&tail));
    }
}

fn resolve_block_text<'a>(
    blk: &'a IonValue,
    content_map: &'a AHashMap<u32, Vec<String>>,
) -> &'a str {
    let Some(c_ref) = blk.get_field(145) else {
        return "";
    };
    if let Some(direct) = c_ref.as_str() {
        return direct;
    }
    let Some(c_sym) = c_ref.get_field(4).and_then(|v| v.as_symbol()) else {
        return "";
    };
    let Ok(c_id) = u32::try_from(c_sym) else {
        return "";
    };
    let idx = c_ref
        .get_field(403)
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .max(0) as usize;
    content_map
        .get(&c_id)
        .and_then(|list| list.get(idx))
        .map(String::as_str)
        .unwrap_or("")
}

fn push_id_attr(eid: Option<i64>, ctx: &StructuredRenderCtx<'_>, out: &mut String) {
    if let Some(id) = eid
        && ctx.target_eids_needed.contains(&id)
    {
        out.push_str(&format!(" id=\"eid_{id}\""));
    }
}

fn render_kfx_block(blk: &IonValue, ctx: &StructuredRenderCtx<'_>, out: &mut String) {
    let type_sym = blk.get_field(159).and_then(|v| v.as_symbol());
    let eid = blk.get_field(155).and_then(|v| v.as_i64());

    match type_sym {
        Some(271) => {
            out.push_str("<p");
            push_id_attr(eid, ctx, out);
            out.push('>');
            let link_href = blk
                .get_field(179)
                .and_then(|v| v.as_symbol())
                .and_then(|s| resolve_anchor_href(s, ctx));
            if let Some(href) = &link_href {
                out.push_str("<a href=\"");
                out.push_str(&xml_escape(href));
                out.push_str("\">");
            }
            if let Some(res_sym) = blk
                .get_field(175)
                .and_then(|v| v.as_symbol())
                .and_then(|s| u32::try_from(s).ok())
                && let Some(rel_path) = ctx.img_rel_paths.get(&res_sym)
            {
                let alt = blk.get_field(584).and_then(|v| v.as_str()).unwrap_or("");
                out.push_str("<img src=\"");
                out.push_str(&xml_escape(rel_path));
                out.push_str("\" alt=\"");
                out.push_str(&xml_escape(alt));
                out.push_str("\" />");
            }
            if link_href.is_some() {
                out.push_str("</a>");
            }
            out.push_str("</p>\n");
        }
        Some(278) => {
            out.push_str("<table");
            push_id_attr(eid, ctx, out);
            out.push_str(">\n");
            if let Some(rows) = blk.get_field(146).and_then(|v| v.as_list()) {
                for row in rows {
                    let row_eid = row.get_field(155).and_then(|v| v.as_i64());
                    out.push_str("<tr");
                    push_id_attr(row_eid, ctx, out);
                    out.push('>');
                    if let Some(cells) = row.get_field(146).and_then(|v| v.as_list()) {
                        for cell in cells {
                            let cell_eid = cell.get_field(155).and_then(|v| v.as_i64());
                            out.push_str("<td");
                            push_id_attr(cell_eid, ctx, out);
                            if let Some(cs) = cell.get_field(148).and_then(|v| v.as_i64())
                                && cs > 1
                            {
                                out.push_str(&format!(" colspan=\"{cs}\""));
                            }
                            if let Some(rs) = cell.get_field(149).and_then(|v| v.as_i64())
                                && rs > 1
                            {
                                out.push_str(&format!(" rowspan=\"{rs}\""));
                            }
                            out.push('>');
                            let text = resolve_block_text(cell, ctx.content_map);
                            render_text_with_style_events(text, cell, ctx, out);
                            out.push_str("</td>");
                        }
                    }
                    out.push_str("</tr>\n");
                }
            }
            out.push_str("</table>\n");
        }
        _ => {
            let text = resolve_block_text(blk, ctx.content_map);
            let children = blk.get_field(146).and_then(|v| v.as_list());
            let nested_story = blk
                .get_field(176)
                .and_then(|v| v.as_symbol())
                .and_then(|s| u32::try_from(s).ok())
                .and_then(|id| ctx.storylines_map.get(&id));

            if !text.is_empty() {
                out.push_str("<p");
                push_id_attr(eid, ctx, out);
                if let Some((d, _)) = style_dir_info(
                    blk.get_field(157).and_then(|v| v.as_symbol()),
                    ctx.styles_map,
                ) {
                    out.push_str(&format!(" dir=\"{d}\""));
                }
                out.push('>');
                render_text_with_style_events(text, blk, ctx, out);
                out.push_str("</p>\n");
            } else if children.is_some() || nested_story.is_some() {
                let is_list = matches!(type_sym, Some(276));
                let wrap_id = eid.filter(|id| ctx.target_eids_needed.contains(id));
                if is_list {
                    out.push_str("<ul");
                    push_id_attr(wrap_id, ctx, out);
                    out.push_str(">\n");
                } else if let Some(id) = wrap_id {
                    out.push_str(&format!("<div id=\"eid_{id}\">\n"));
                }
                if let Some(items) = children {
                    for child in items {
                        if let Some(s) = child.as_str() {
                            if !s.trim().is_empty() {
                                out.push_str("<p>");
                                out.push_str(&xml_escape(s));
                                out.push_str("</p>\n");
                            }
                        } else {
                            render_kfx_block(child, ctx, out);
                        }
                    }
                }
                if let Some(st_val) = nested_story
                    && let Some(blocks) = st_val.get_field(146).and_then(|v| v.as_list())
                {
                    for child in blocks {
                        render_kfx_block(child, ctx, out);
                    }
                }
                if is_list {
                    out.push_str("</ul>\n");
                } else if wrap_id.is_some() {
                    out.push_str("</div>\n");
                }
            }
        }
    }
}

fn parse_kfx_toc(
    nav_val: &IonValue,
    nav_containers_map: &AHashMap<u32, IonValue>,
    eid_to_sec_idx: &AHashMap<i64, usize>,
    counter: &mut usize,
) -> Vec<NavPoint> {
    let Some(orders) = nav_val.as_list() else {
        return Vec::new();
    };
    for order in orders {
        let Some(containers) = order.get_field(392).and_then(|v| v.as_list()) else {
            continue;
        };
        for item in containers {
            let container = match item {
                IonValue::Annotated(_, inner) => inner.as_ref(),
                IonValue::Symbol(sym) => {
                    let Ok(id) = u32::try_from(*sym) else {
                        continue;
                    };
                    let Some(c) = nav_containers_map.get(&id) else {
                        continue;
                    };
                    c
                }
                other => other,
            };
            if container.get_field(235).and_then(|v| v.as_symbol()) != Some(212) {
                continue;
            }
            if let Some(entries) = container.get_field(247).and_then(|v| v.as_list()) {
                return parse_kfx_nav_units(entries, eid_to_sec_idx, counter);
            }
        }
    }
    Vec::new()
}

fn parse_kfx_nav_units(
    entries: &[IonValue],
    eid_to_sec_idx: &AHashMap<i64, usize>,
    counter: &mut usize,
) -> Vec<NavPoint> {
    let mut out = Vec::new();
    for entry in entries {
        let inner = match entry {
            IonValue::Annotated(_, v) => v.as_ref(),
            v => v,
        };
        let label = inner
            .get_field(241)
            .and_then(|r| r.get_field(244))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Section")
            .to_string();
        let target_eid = inner
            .get_field(246)
            .and_then(|t| t.get_field(155))
            .and_then(|v| v.as_i64());
        let sec_idx = target_eid
            .and_then(|eid| eid_to_sec_idx.get(&eid).copied())
            .unwrap_or(0);
        let href = format!("sec_{sec_idx}.xhtml");
        let full_path = format!("OEBPS/{href}");
        let id = format!("toc_{}", *counter);
        *counter += 1;

        let subitems = inner
            .get_field(247)
            .and_then(|v| v.as_list())
            .map(|children| parse_kfx_nav_units(children, eid_to_sec_idx, counter))
            .unwrap_or_default();

        out.push(NavPoint {
            id,
            label,
            href,
            full_path,
            subitems,
        });
    }
    out
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

fn decode_enty_ion_value(payload: &[u8]) -> Option<IonValue> {
    let ion_slice = strip_enty_header(payload)?;
    let values = crate::kfx::ion::decode_stream(ion_slice).ok()?;
    for val in values {
        // Skip local symbol table (`$ion_symbol_table` annotation ID 3)
        if let IonValue::Annotated(annots, _) = &val
            && annots.contains(&3)
        {
            continue;
        }
        return Some(val);
    }
    None
}

fn extract_ion_book_metadata(
    val: &IonValue,
    metadata: &mut Metadata,
    cover_res_name: &mut Option<String>,
) {
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
                "ASIN" | "book_id" | "content_id" if metadata.identifier.is_none() => {
                    metadata.identifier = Some(trimmed.to_string());
                }
                "cover_image" if cover_res_name.is_none() => {
                    *cover_res_name = Some(trimmed.to_string());
                }
                _ => {}
            }
        }
    }
}

fn extract_ion_legacy_metadata(val: &IonValue, metadata: &mut Metadata) {
    if metadata.title.is_empty()
        && let Some(t) = val.get_field(153).and_then(|v| v.as_str())
        && !t.trim().is_empty()
    {
        metadata.title = t.trim().to_string();
    }
    if metadata.identifier.is_none()
        && let Some(id) = val.get_field(224).and_then(|v| v.as_str())
        && !id.trim().is_empty()
    {
        metadata.identifier = Some(id.trim().to_string());
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

fn extract_ion_content_list(val: &IonValue, out: &mut Vec<String>) {
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

fn extract_ion_storyline_text(val: &IonValue, out: &mut Vec<String>) {
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
