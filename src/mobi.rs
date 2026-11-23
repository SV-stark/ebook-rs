use crate::archive::EpubArchive;
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::error::EbookError;
use crate::layout::RenditionLayout;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::{Section, extract_plain_text};
use ahash::AHashMap;

/// Maximum output size of [`decompress_palmdoc`].
///
/// A single input byte can emit up to 8 output bytes (`0x01..=0x08`), and the LZ77 branch can
/// expand further, so this is a real bound rather than a formality.
pub const MAX_PALMDOC_OUTPUT: usize = 256 * 1024 * 1024;

/// PalmDOC LZ77 Decompressor.
///
/// Output is truncated at [`MAX_PALMDOC_OUTPUT`] to bound memory: the compressed text region of
/// a MOBI is attacker-controlled and this function is part of the public API.
pub fn decompress_palmdoc(data: &[u8]) -> Vec<u8> {
    // Cap the pre-allocation too: `data.len() * 2` is itself an overflow risk for large inputs.
    let init_cap = data.len().saturating_mul(2).min(8 * 1024 * 1024);
    let mut out = Vec::with_capacity(init_cap);
    let mut i = 0;

    while i < data.len() {
        if out.len() >= MAX_PALMDOC_OUTPUT {
            break;
        }
        let byte = data[i];
        i += 1;

        match byte {
            0x00 | 0x09..=0x7F => {
                out.push(byte);
            }
            0x01..=0x08 => {
                let count = byte as usize;
                let end = (i + count).min(data.len());
                out.extend_from_slice(&data[i..end]);
                i += count;
            }
            0x80..=0xBF => {
                if i < data.len() {
                    let next = data[i];
                    i += 1;
                    let b1 = byte as usize;
                    let b2 = next as usize;
                    let distance = ((b1 & 0x3F) << 5) | (b2 >> 3);
                    let length = (b2 & 0x07) + 3;

                    if distance > 0 && distance <= out.len() {
                        let start = out.len() - distance;
                        for j in 0..length {
                            let val = out[start + (j % distance)];
                            out.push(val);
                        }
                    } else {
                        out.push(byte);
                        out.push(next);
                    }
                } else {
                    out.push(byte);
                }
            }
            0xC0..=0xFF => {
                out.push(b' ');
                out.push(byte ^ 0x80);
            }
        }
    }

    out
}

/// MOBI / AZW3 PDB Container Parser.
pub struct MobiBook;

impl MobiBook {
    /// Parse MOBI / AZW3 raw binary data into a `Book` struct.
    pub fn parse(bytes: &[u8]) -> Result<Book, EbookError> {
        if bytes.len() < 78 {
            return Err(EbookError::InvalidFormat(
                "File too small for MOBI/AZW3 PDB header".to_string(),
            ));
        }

        let name_bytes = &bytes[0..32];
        let name = String::from_utf8_lossy(name_bytes)
            .trim_matches('\0')
            .to_string();

        let num_records = u16::from_be_bytes([bytes[76], bytes[77]]) as usize;
        if bytes.len() < 78 + num_records * 8 {
            return Err(EbookError::InvalidFormat(
                "Truncated MOBI PDB record offset table".to_string(),
            ));
        }

        let mut record_offsets = Vec::with_capacity(num_records);
        for i in 0..num_records {
            let start = 78 + i * 8;
            let offset = u32::from_be_bytes([
                bytes[start],
                bytes[start + 1],
                bytes[start + 2],
                bytes[start + 3],
            ]) as usize;
            record_offsets.push(offset);
        }

        if record_offsets.is_empty() {
            return Err(EbookError::InvalidFormat(
                "MOBI archive has 0 records".to_string(),
            ));
        }

        let rec0_offset = record_offsets[0];
        let rec0_end = if record_offsets.len() > 1 {
            record_offsets[1]
        } else {
            bytes.len()
        };

        if rec0_offset >= bytes.len() || rec0_end > bytes.len() || rec0_offset + 16 > rec0_end {
            return Err(EbookError::InvalidFormat(
                "Invalid Record 0 bounds in MOBI header".to_string(),
            ));
        }

        let rec0 = &bytes[rec0_offset..rec0_end];
        let compression = u16::from_be_bytes([rec0[0], rec0[1]]);

        // Check Mobipocket DRM encryption field at rec0[12..14]
        if rec0.len() >= 14 {
            let encryption_type = u16::from_be_bytes([rec0[12], rec0[13]]);
            if encryption_type != 0 {
                return Err(EbookError::DrmProtected("Book is protected by Mobipocket DRM encryption. Please remove DRM to read or convert this book.".to_string()));
            }
        }

        let mut text_record_count = u16::from_be_bytes([rec0[8], rec0[9]]) as usize;
        let max_text_recs = if num_records > 1 { num_records - 1 } else { 1 };
        if text_record_count == 0 || text_record_count >= num_records {
            text_record_count = 1.min(max_text_recs);
        }

        let pdb_name = if name.is_empty() {
            "Untitled MOBI Book".to_string()
        } else {
            // PDB names use underscores instead of spaces — normalize as a fallback
            name.replace('_', " ")
        };
        let mut title = pdb_name.clone();
        let mut author = String::new();
        let mut publisher = None;
        let mut language = "en".to_string();
        let mut text_encoding: u32 = 0;
        let mut extra_flags: u32 = 0;
        let mut fdst_rec_index: Option<usize> = None;

        if rec0.len() >= 40 && &rec0[16..20] == b"MOBI" {
            let header_len = u32::from_be_bytes([rec0[20], rec0[21], rec0[22], rec0[23]]) as usize;
            text_encoding = u32::from_be_bytes([rec0[28], rec0[29], rec0[30], rec0[31]]);
            let mobi_version = u32::from_be_bytes([rec0[36], rec0[37], rec0[38], rec0[39]]);
            if mobi_version >= 8 && header_len >= 184 && rec0.len() >= 200 {
                let idx = u32::from_be_bytes([rec0[192], rec0[193], rec0[194], rec0[195]]) as usize;
                if idx > 0 && idx < num_records {
                    fdst_rec_index = Some(idx);
                }
            }
            if header_len >= 228 && rec0.len() >= 244 {
                extra_flags = u32::from_be_bytes([rec0[240], rec0[241], rec0[242], rec0[243]]);
            }

            // `header_len` is an attacker-controlled `u32` widened to `usize`. On 32-bit targets
            // (wasm32, i686 — both supported here) `16 + header_len + 12` overflows, and the
            // release profile sets `overflow-checks = true`, so that is a hard panic rather than
            // a wrap. Use checked arithmetic so the record is simply rejected instead.
            let header_end = rec0
                .len()
                .checked_sub(16 + 12)
                .filter(|end| *end >= header_len);
            if header_len > 0 && rec0.len() >= 40 && header_end.is_some() {
                let exth_offset = 16 + header_len;
                if exth_offset + 4 <= rec0.len() && &rec0[exth_offset..exth_offset + 4] == b"EXTH" {
                    let count = u32::from_be_bytes([
                        rec0[exth_offset + 8],
                        rec0[exth_offset + 9],
                        rec0[exth_offset + 10],
                        rec0[exth_offset + 11],
                    ]) as usize;

                    let mut curr = exth_offset + 12;
                    for _ in 0..count {
                        if curr + 8 > rec0.len() {
                            break;
                        }
                        let tag = u32::from_be_bytes([
                            rec0[curr],
                            rec0[curr + 1],
                            rec0[curr + 2],
                            rec0[curr + 3],
                        ]);
                        let len = u32::from_be_bytes([
                            rec0[curr + 4],
                            rec0[curr + 5],
                            rec0[curr + 6],
                            rec0[curr + 7],
                        ]) as usize;

                        // `curr + len` widens an attacker-controlled `u32` past `usize::MAX` on a
                        // 32-bit target, which panics under `overflow-checks = true`. Checked
                        // arithmetic keeps the loop bounded instead.
                        let val_end = curr.checked_add(len);
                        if len >= 8 && val_end.is_some_and(|end| end <= rec0.len()) {
                            let val_bytes = &rec0[curr + 8..curr + len];
                            let val_str = String::from_utf8_lossy(val_bytes).trim().to_string();

                            match tag {
                                100 => {
                                    if !val_str.is_empty() {
                                        author = val_str;
                                    }
                                }
                                101 => publisher = Some(val_str),
                                524 => {
                                    // Tag 524 = locale/language (BCP-47), always preferred
                                    if !val_str.is_empty() {
                                        language = val_str;
                                    }
                                }
                                106 => {
                                    // Tag 106 = publication date (often a datetime string)
                                    // Only use as language if it looks like a BCP-47 code
                                    let looks_like_lang = val_str.len() <= 8
                                        && val_str
                                            .chars()
                                            .all(|c| c.is_ascii_alphabetic() || c == '-');
                                    if looks_like_lang && language == "en" {
                                        language = val_str;
                                    }
                                }
                                // EXTH 503 = updated title (overrides PDB name)
                                503 if !val_str.is_empty() => {
                                    title = val_str;
                                }
                                _ => {}
                            }
                        }
                        curr += len.max(8);
                    }
                }
            }

            // Also try reading full-title from MOBI header full_name_offset / full_name_length
            // (bytes 84-87 = offset from record 0 start, bytes 88-91 = length)
            if title == pdb_name && rec0.len() >= 92 {
                let fn_offset =
                    u32::from_be_bytes([rec0[84], rec0[85], rec0[86], rec0[87]]) as usize;
                let fn_len = u32::from_be_bytes([rec0[88], rec0[89], rec0[90], rec0[91]]) as usize;
                // Both fields are attacker-controlled `u32`s widened to `usize`; `fn_offset +
                // fn_len` overflows (and panics under `overflow-checks = true`) when each is
                // close to `usize::MAX`, which is reachable on 32-bit targets.
                let fn_end = fn_offset.checked_add(fn_len);
                if fn_len > 0 && fn_end.is_some_and(|end| end <= rec0.len()) {
                    let full_name = String::from_utf8_lossy(&rec0[fn_offset..fn_offset + fn_len])
                        .trim()
                        .to_string();
                    if !full_name.is_empty() {
                        title = full_name;
                    }
                }
            }
        }

        // Fallback author if EXTH didn't provide one
        if author.is_empty() {
            author = "Unknown Author".to_string();
        }

        let first_image_index = if rec0.len() >= 112 && &rec0[16..20] == b"MOBI" {
            let img_rec_val =
                u32::from_be_bytes([rec0[108], rec0[109], rec0[110], rec0[111]]) as usize;
            if img_rec_val > 0 && img_rec_val < num_records {
                img_rec_val
            } else {
                1 + text_record_count
            }
        } else {
            1 + text_record_count
        };

        let mut raw_html_bytes = Vec::new();
        let max_text_rec = (1 + text_record_count).min(record_offsets.len());

        for i in 1..max_text_rec {
            let start = record_offsets[i];
            let end = if i + 1 < record_offsets.len() {
                record_offsets[i + 1]
            } else {
                bytes.len()
            };

            if start >= bytes.len() || end > bytes.len() || start >= end {
                continue;
            }

            let chunk = &bytes[start..end];
            let trim = if extra_flags != 0 {
                trailing_data_size(chunk, extra_flags)
            } else {
                0
            };
            let payload = &chunk[..chunk.len().saturating_sub(trim)];
            match compression {
                1 => raw_html_bytes.extend_from_slice(payload),
                2 => raw_html_bytes.extend_from_slice(&decompress_palmdoc(payload)),
                17480 => {
                    return Err(EbookError::InvalidFormat(
                        "MOBI HUFF/CDIC compression is unsupported. Please convert using Calibre or KindleUnpack.".to_string(),
                    ));
                }
                _ => raw_html_bytes.extend_from_slice(payload),
            }
        }

        if let Some(fdst_idx) = fdst_rec_index {
            let fstart = record_offsets[fdst_idx];
            let fend = if fdst_idx + 1 < record_offsets.len() {
                record_offsets[fdst_idx + 1]
            } else {
                bytes.len()
            };
            if fstart < fend && fend <= bytes.len() {
                if let Some((flow0_start, flow0_end)) =
                    fdst_flow0_range(&bytes[fstart..fend], raw_html_bytes.len())
                {
                    raw_html_bytes.truncate(flow0_end);
                    if flow0_start > 0 {
                        raw_html_bytes.drain(..flow0_start);
                    }
                }
            }
        }

        let full_html_raw = match text_encoding {
            65001 => String::from_utf8(raw_html_bytes)
                .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
            1252 => encoding_rs::WINDOWS_1252
                .decode(&raw_html_bytes)
                .0
                .into_owned(),
            _ => match std::str::from_utf8(&raw_html_bytes) {
                Ok(s) => s.to_string(),
                Err(_) => encoding_rs::WINDOWS_1252
                    .decode(&raw_html_bytes)
                    .0
                    .into_owned(),
            },
        };
        let mut full_html = full_html_raw;
        if full_html.trim().is_empty() || extract_plain_text(&full_html).trim().is_empty() {
            full_html = extract_fallback_mobi_text(bytes);
        }
        full_html = sanitize_mobi_control_chars(&full_html);

        let mut archive = EpubArchive::empty();
        let full_html = extract_mobi_images_and_populate_archive(
            &full_html,
            bytes,
            &record_offsets,
            first_image_index,
            &mut archive,
        );

        let raw_sections = split_mobi_html(&full_html);
        let mut sections = Vec::with_capacity(raw_sections.len());
        let mut spine = Vec::with_capacity(raw_sections.len());
        let mut toc = Vec::new();

        for (idx, raw_sec_html) in raw_sections.into_iter().enumerate() {
            let idref = format!("section_{}", idx);
            let href = format!("section_{}.html", idx);
            let plain_text = extract_plain_text(&raw_sec_html);
            let plain_text_lower = plain_text.to_lowercase();
            let char_count = plain_text.chars().count();

            sections.push(Section {
                index: idx,
                idref: idref.clone(),
                href: href.clone(),
                full_path: href.clone(),
                raw_html: raw_sec_html.clone(),
                processed_html: raw_sec_html.clone(),
                plain_text,
                plain_text_lower,
                char_count,
                viewport_width: None,
                viewport_height: None,
            });

            spine.push(SpineItem {
                index: idx,
                idref,
                href: href.clone(),
                linear: true,
                media_type: "text/html".to_string(),
                properties: Vec::new(),
            });

            toc.push(NavPoint {
                id: format!("toc_{}", idx),
                label: format!("Section {}", idx + 1),
                href: href.clone(),
                full_path: href,
                subitems: Vec::new(),
            });
        }

        let lang_lower = language.to_lowercase();
        let direction = if lang_lower.starts_with("ar")
            || lang_lower.starts_with("he")
            || lang_lower.starts_with("fa")
            || lang_lower.starts_with("ur")
        {
            PageProgressionDirection::Rtl
        } else {
            PageProgressionDirection::Ltr
        };

        let metadata = Metadata {
            title,
            creators: vec![author],
            publishers: publisher.map(|p| vec![p]).unwrap_or_default(),
            languages: vec![language],
            rights: None,
            description: None,
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects: Vec::new(),
            cover_id: None,
            cover_href: None,
            direction,
            meta_properties: AHashMap::new(),
            accessibility: Default::default(),
        };

        let opf = OpfPackage {
            version: "2.0".to_string(),
            opf_path: "OEBPS/content.opf".to_string(),
            opf_dir: "OEBPS".to_string(),
            metadata,
            manifest: AHashMap::new(),
            spine,
            guide: Vec::new(),
            toc_item_id: None,
            nav_item_id: None,
        };

        let mut book = Book {
            archive,
            opf,
            layout: RenditionLayout::default(),
            toc,
            landmarks: Vec::new(),
            page_list: Vec::new(),
            sections,
            locations: crate::locations::Locations::default(),
            annotations: crate::annotations::AnnotationManager::default(),
            before_display_hooks: Vec::new(),
            font_deobfuscator: FontDeobfuscator::parse_encryption_xml(""),
            lcp_passphrase: None,
            lcp_license: None,
            media_overlays: AHashMap::new(),
            render_cache: Default::default(),
            hydrated_cache: Default::default(),
        };

        book.generate_locations(1000);
        Ok(book)
    }
}

fn find_ignore_case(s: &str, pat: &str) -> Option<usize> {
    if pat.is_empty() {
        return Some(0);
    }
    if s.len() < pat.len() {
        return None;
    }
    for i in 0..=s.len() - pat.len() {
        if s.is_char_boundary(i)
            && let Some(sub) = s.get(i..i + pat.len())
            && sub.eq_ignore_ascii_case(pat)
        {
            return Some(i);
        }
    }
    None
}

fn split_mobi_html(html: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut search_idx = 0;

    // Primary split: <mbp:pagebreak> (MOBI)
    while search_idx < html.len() {
        if let Some(pb_idx) = find_ignore_case(&html[search_idx..], "<mbp:pagebreak") {
            let abs_idx = search_idx + pb_idx;
            if abs_idx > search_idx {
                let chunk = html[search_idx..abs_idx].trim();
                if !chunk.is_empty() {
                    parts.push(chunk.to_string());
                }
            }
            if let Some(abs_close) = crate::section::find_tag_end(html, abs_idx) {
                search_idx = abs_close + 1;
            } else {
                search_idx = abs_idx + "<mbp:pagebreak".len();
            }
        } else {
            break;
        }
    }

    if search_idx < html.len() {
        let remainder = html[search_idx..].trim();
        if !remainder.is_empty() {
            parts.push(remainder.to_string());
        }
    }

    // If no <mbp:pagebreak> splits were found (AZW3), fall back to splitting
    // on heading-level anchors: <a id="..."> that precede <h1>/<h2>/<h3> tags.
    // We collect boundary indices where a new chapter begins.
    if parts.len() <= 1 && !html.trim().is_empty() {
        parts.clear();
        let mut boundaries: Vec<usize> = vec![0];

        // Find all <h1>, <h2>, <h3> tag positions as chapter split points
        for (byte_idx, _) in html.char_indices() {
            if byte_idx > 200 {
                let slice = &html[byte_idx..];
                for tag in &["<h1", "<h2", "<h3"] {
                    if let Some(sub) = slice.get(..tag.len())
                        && sub.eq_ignore_ascii_case(tag)
                    {
                        boundaries.push(byte_idx);
                        break;
                    }
                }
            }
        }

        boundaries.sort_unstable();
        boundaries.dedup();

        if boundaries.len() > 1 {
            for i in 0..boundaries.len() {
                let start = boundaries[i];
                let end = if i + 1 < boundaries.len() {
                    boundaries[i + 1]
                } else {
                    html.len()
                };
                let chunk = html[start..end].trim();
                if !chunk.is_empty() {
                    parts.push(chunk.to_string());
                }
            }
        } else {
            // Last resort: single section
            parts.push(html.trim().to_string());
        }
    }

    parts
}

fn extract_fallback_mobi_text(bytes: &[u8]) -> String {
    let lossy = String::from_utf8_lossy(bytes);
    let mut out = String::new();
    for line in lossy.lines() {
        let trimmed = line.trim();
        if trimmed.len() > 10 && trimmed.chars().any(|c| c.is_alphabetic()) {
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    if out.is_empty() {
        "<p>AZW3 Book Content</p>".to_string()
    } else {
        out
    }
}

fn sanitize_mobi_control_chars(input: &str) -> String {
    input
        .chars()
        .filter(|&c| c == '\n' || c == '\r' || c == '\t' || !c.is_control())
        .collect()
}

fn extract_mobi_images_and_populate_archive(
    html: &str,
    bytes: &[u8],
    record_offsets: &[usize],
    first_image_index: usize,
    archive: &mut EpubArchive,
) -> String {
    let mut image_map: AHashMap<usize, String> = AHashMap::new();
    let num_records = record_offsets.len();

    let start_img_rec = first_image_index.max(1);
    for rec_idx in start_img_rec..num_records {
        let rec_start = record_offsets[rec_idx];
        let rec_end = if rec_idx + 1 < num_records {
            record_offsets[rec_idx + 1]
        } else {
            bytes.len()
        };

        if rec_start < bytes.len() && rec_end <= bytes.len() && rec_start < rec_end {
            let img_bytes = &bytes[rec_start..rec_end];
            if let Some(mime) = detect_image_mime(img_bytes) {
                let ext = match mime {
                    "image/jpeg" => "jpg",
                    "image/png" => "png",
                    "image/gif" => "gif",
                    "image/webp" => "webp",
                    "image/bmp" => "bmp",
                    _ => "jpg",
                };
                let img_num = (rec_idx - start_img_rec) + 1;
                let rel_path = format!("images/img_{:04}.{}", img_num, ext);
                let full_archive_path = format!("OEBPS/{}", rel_path);
                archive.insert(full_archive_path, img_bytes.to_vec());
                image_map.insert(img_num, rel_path);
            }
        }
    }

    if image_map.is_empty() {
        return html.to_string();
    }

    // Single-pass substitution. The previous implementation ran nine whole-string `replace()`
    // calls per image; with up to 65 534 image records over a multi-MB HTML body that is
    // O(images x html_len) allocations and copies (tens of GB of churn from a ~20 MB file).
    //
    // All nine patterns are digit runs anchored by a fixed prefix, so one forward scan that parses
    // the run and looks it up in `image_map` is exactly equivalent and linear. Anchoring on the
    // marker (and, for `src`, on the `.jpg"` suffix) keeps the padded and unpadded forms
    // unambiguous: `recindex="1"` cannot match inside `recindex="00001"`.
    const RECINDEX: &str = "recindex=\"";
    const KINDLE: &str = "kindle:embed:";
    const SRC: &str = "src=\"";

    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut i = 0usize;
    while i < html.len() {
        let rest = &html[i..];

        let kind = if rest.starts_with(RECINDEX) {
            0u8
        } else if rest.starts_with(KINDLE) {
            1u8
        } else if rest.starts_with(SRC) {
            2u8
        } else {
            // Not a marker: copy one whole char so we never split a UTF-8 sequence.
            let ch = rest.chars().next().unwrap_or(' ');
            out.push(ch);
            i += ch.len_utf8();
            continue;
        };
        let marker_len = match kind {
            0 => RECINDEX.len(),
            1 => KINDLE.len(),
            _ => SRC.len(),
        };

        // Read the digit run (bounded so an absurd literal cannot cost more than 10 bytes).
        let digits_start = i + marker_len;
        let mut digits_end = digits_start;
        while digits_end < bytes.len()
            && bytes[digits_end].is_ascii_digit()
            && digits_end - digits_start < 10
        {
            digits_end += 1;
        }

        if digits_end == digits_start {
            out.push_str(&rest[..marker_len]);
            i += marker_len;
            continue;
        }

        // Each marker has its own terminator; validate it before trusting the parsed number.
        let (terminator, replacement) = match kind {
            0 => ("\"", "src"),
            1 => ("", ""),
            _ => (".jpg\"", "src"),
        };
        if !html[digits_end..].starts_with(terminator) {
            out.push_str(&rest[..marker_len]);
            i += marker_len;
            continue;
        }

        let Ok(num) = html[digits_start..digits_end].parse::<usize>() else {
            out.push_str(&rest[..marker_len]);
            i += marker_len;
            continue;
        };

        let Some(rel_path) = image_map.get(&num) else {
            out.push_str(&rest[..marker_len]);
            i += marker_len;
            continue;
        };

        if replacement == "src" {
            out.push_str("src=\"");
            out.push_str(rel_path);
            out.push('"');
        } else {
            out.push_str(rel_path);
        }
        i = digits_end + terminator.len();
    }

    out
}

fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 4 {
        return None;
    }
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.starts_with(b"BM") {
        Some("image/bmp")
    } else {
        None
    }
}

/// Parse a KF8 `FDST` flow table record and return `(start, end)` byte bounds for Flow 0
/// (the HTML content flow), excluding trailing CSS/SVG flows.
fn fdst_flow0_range(fdst: &[u8], total_len: usize) -> Option<(usize, usize)> {
    if fdst.len() < 20 || &fdst[0..4] != b"FDST" {
        return None;
    }
    let num_flows = u32::from_be_bytes([fdst[8], fdst[9], fdst[10], fdst[11]]) as usize;
    if num_flows == 0 {
        return None;
    }
    let start = u32::from_be_bytes([fdst[12], fdst[13], fdst[14], fdst[15]]) as usize;
    let end = u32::from_be_bytes([fdst[16], fdst[17], fdst[18], fdst[19]]) as usize;
    if start < end && end <= total_len {
        Some((start, end))
    } else {
        None
    }
}

/// Compute the byte length of trailing extra data appended to a MOBI text record
/// according to the `extra_flags` bitfield at Record 0 offset 240..244.
fn trailing_data_size(data: &[u8], extra_flags: u32) -> usize {
    let mut num = 0usize;
    let mut flags = extra_flags >> 1;
    while flags != 0 {
        if (flags & 1) != 0 {
            let mut pos = match data.len().checked_sub(1 + num) {
                Some(p) => p,
                None => return data.len(),
            };
            let mut size = 0usize;
            let mut shift = 0u32;
            let mut varint_bytes = 0usize;
            loop {
                let b = data[pos];
                varint_bytes += 1;
                size |= ((b & 0x7F) as usize) << shift;
                shift += 7;
                if (b & 0x80) != 0 || pos == 0 || shift >= 28 {
                    break;
                }
                pos -= 1;
            }
            num = num.saturating_add(size.max(varint_bytes));
            if num >= data.len() {
                return data.len();
            }
        }
        flags >>= 1;
    }
    if (extra_flags & 1) != 0 {
        if let Some(pos) = data.len().checked_sub(1 + num) {
            let overlap = ((data[pos] & 0x03) as usize) + 1;
            num = num.saturating_add(overlap).min(data.len());
        }
    }
    num
}

#[cfg(test)]
mod tests {
    use super::{fdst_flow0_range, trailing_data_size};

    #[test]
    fn test_trailing_data_size_flags() {
        // Bit 0 only: last byte 0x02 -> (2 & 3) + 1 = 3 bytes
        assert_eq!(trailing_data_size(&[b'a', b'b', 0xAA, 0xBB, 0x02], 0x01), 3);
        // Bit 1 + Bit 0: varint 0x84 (size 4) at end, preceded by overlap 0x01 (2 bytes)
        let rec = [b'x', 0xCC, 0x01, 0x11, 0x22, 0x33, 0x84];
        assert_eq!(trailing_data_size(&rec, 0x03), 6);
    }

    #[test]
    fn test_fdst_flow0_range() {
        let mut fdst = Vec::new();
        fdst.extend_from_slice(b"FDST");
        fdst.extend_from_slice(&12u32.to_be_bytes());
        fdst.extend_from_slice(&2u32.to_be_bytes());
        fdst.extend_from_slice(&0u32.to_be_bytes());
        fdst.extend_from_slice(&1200u32.to_be_bytes());
        fdst.extend_from_slice(&1200u32.to_be_bytes());
        fdst.extend_from_slice(&1500u32.to_be_bytes());
        assert_eq!(fdst_flow0_range(&fdst, 1500), Some((0, 1200)));
        assert_eq!(fdst_flow0_range(&fdst, 1000), None);
    }
}
