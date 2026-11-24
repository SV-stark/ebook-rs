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

/// Microsoft Reader LIT Format Parser.
pub struct LitBook;

impl LitBook {
    /// Parse Microsoft Reader LIT file bytes into a `Book` struct.
    pub fn parse(bytes: &[u8]) -> Result<Book, EbookError> {
        if bytes.len() < 8 {
            return Err(EbookError::InvalidFormat(
                "File too small for LIT header".to_string(),
            ));
        }

        // LIT files start with ITOL / ITLS header or raw container stream
        if !bytes.starts_with(b"ITOL")
            && !bytes.starts_with(b"ITLS")
            && !bytes.contains_str(b"ITOL")
        {
            return Err(EbookError::InvalidFormat(
                "Not a valid Microsoft Reader LIT container".to_string(),
            ));
        }

        if bytes.starts_with(b"ITOLITLS") {
            return parse_itolitls_container(bytes)
                .map_err(|e| EbookError::InvalidFormat(format!("LIT parse error: {e}")));
        }

        // Extract HTML text strings embedded inside LIT binary container streams
        let text_content = extract_html_from_lit_bytes(bytes);
        let raw_lossy = String::from_utf8_lossy(bytes);

        let mut title = "LIT Document".to_string();
        let mut creators = Vec::new();

        if let Some(t) = extract_tag_content(&text_content, "title")
            .or_else(|| extract_tag_content(&raw_lossy, "dc:title"))
            .or_else(|| extract_meta_attr(&text_content, "title"))
            .or_else(|| extract_meta_attr(&raw_lossy, "title"))
            && !t.trim().is_empty()
        {
            title = t.trim().to_string();
        }

        if let Some(a) = extract_tag_content(&text_content, "author")
            .or_else(|| extract_tag_content(&raw_lossy, "dc:creator"))
            .or_else(|| extract_meta_attr(&text_content, "author"))
            .or_else(|| extract_meta_attr(&raw_lossy, "author"))
            && !a.trim().is_empty()
        {
            creators.push(a.trim().to_string());
        }

        if creators.is_empty() {
            creators.push("Unknown Author".to_string());
        }

        // Split text content into sections based on <h1>, <h2>, <h3> tag boundaries
        let raw_parts = split_lit_html(&text_content);
        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();

        for (idx, raw_sec_html) in raw_parts.into_iter().enumerate() {
            let idref = format!("lit_sec_{}", idx);
            let href = format!("lit_sec_{}.html", idx);
            let plain_text = extract_plain_text(&raw_sec_html);
            let plain_text_lower = plain_text.to_lowercase();
            let char_count = plain_text.chars().count();

            sections.push(Section {
                index: idx,
                idref: idref.clone(),
                href: href.clone(),
                full_path: href.clone(),
                raw_html: raw_sec_html.clone(),
                processed_html: raw_sec_html,
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

        let metadata = Metadata {
            title,
            creators,
            publishers: Vec::new(),
            languages: vec!["en".to_string()],
            rights: None,
            description: None,
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects: Vec::new(),
            cover_id: None,
            cover_href: None,
            direction: PageProgressionDirection::Ltr,
            meta_properties: AHashMap::new(),
            accessibility: Default::default(),
        };

        let opf = OpfPackage {
            version: "2.0".to_string(),
            opf_path: "content.opf".to_string(),
            opf_dir: "".to_string(),
            metadata,
            manifest: AHashMap::new(),
            spine,
            guide: Vec::new(),
            toc_item_id: None,
            nav_item_id: None,
        };

        let mut archive = EpubArchive::empty();
        extract_images_from_lit_bytes(bytes, &mut archive);

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

trait ContainsStr {
    fn contains_str(&self, needle: &[u8]) -> bool;
}

impl ContainsStr for [u8] {
    fn contains_str(&self, needle: &[u8]) -> bool {
        self.windows(needle.len()).any(|w| w == needle)
    }
}

fn extract_html_from_lit_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();

    // Check if container bytes are UTF-16LE encoded
    let text = if bytes.windows(2).any(|w| w == b"<\0") {
        let u16_data: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| u16::from_le_bytes(*chunk))
            .collect();
        String::from_utf16_lossy(&u16_data)
    } else {
        String::from_utf8_lossy(bytes).to_string()
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && (trimmed.contains('<') || trimmed.len() > 15) {
            out.push_str(trimmed);
            out.push('\n');
        }
    }

    if out.is_empty() {
        "<p>LIT Document Content</p>".to_string()
    } else {
        out
    }
}

fn extract_tag_content(html: &str, tag_name: &str) -> Option<String> {
    let open_prefix = format!("<{}", tag_name);
    let close_tag = format!("</{}>", tag_name);
    let open_bytes = open_prefix.as_bytes();
    let close_bytes = close_tag.as_bytes();
    let html_bytes = html.as_bytes();

    for (start_byte, _) in html.char_indices() {
        if start_byte + open_bytes.len() <= html_bytes.len()
            && html_bytes[start_byte..start_byte + open_bytes.len()]
                .eq_ignore_ascii_case(open_bytes)
        {
            let slice = &html[start_byte..];
            let next_idx = open_bytes.len();
            if next_idx < slice.len() {
                let next_char = slice[next_idx..].chars().next().unwrap_or(' ');
                if next_char != '>' && !next_char.is_whitespace() {
                    continue;
                }
            }
            if let Some(tag_end) = slice.find('>') {
                let content_start = start_byte + tag_end + 1;
                let remaining = &html_bytes[content_start..];
                for (end_byte, _) in html[content_start..].char_indices() {
                    if end_byte + close_bytes.len() <= remaining.len()
                        && remaining[end_byte..end_byte + close_bytes.len()]
                            .eq_ignore_ascii_case(close_bytes)
                    {
                        let val = html[content_start..content_start + end_byte].trim();
                        if !val.is_empty() {
                            return Some(val.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_meta_name(html: &str, name: &str) -> Option<String> {
    let target = format!("name=\"{}\"", name);
    let target_bytes = target.as_bytes();
    let html_bytes = html.as_bytes();

    for (byte_idx, _) in html.char_indices() {
        if byte_idx + target_bytes.len() <= html_bytes.len()
            && html_bytes[byte_idx..byte_idx + target_bytes.len()]
                .eq_ignore_ascii_case(target_bytes)
        {
            let remaining = &html[byte_idx..];
            for (c_idx, _) in remaining.char_indices() {
                if c_idx + 9 <= remaining.len()
                    && remaining.as_bytes()[c_idx..c_idx + 9].eq_ignore_ascii_case(b"content=\"")
                {
                    let val_start = byte_idx + c_idx + 9;
                    if val_start < html.len()
                        && let Some(val_end) = html[val_start..].find('"')
                    {
                        let val = html[val_start..val_start + val_end].trim();
                        if !val.is_empty() {
                            return Some(val.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_meta_attr(html: &str, name: &str) -> Option<String> {
    extract_meta_name(html, name)
}

fn split_lit_html(html: &str) -> Vec<String> {
    let mut boundaries = vec![0];
    let html_bytes = html.as_bytes();

    let patterns = &[
        "<h1",
        "<h2",
        "<h3",
        "<div class=\"chapter\"",
        "chapter i.",
        "chapter ii.",
        "chapter iii.",
        "chapter iv.",
        "chapter v.",
        "chapter vi.",
        "chapter vii.",
        "chapter viii.",
        "chapter ix.",
        "chapter x.",
        "chapter xi.",
        "chapter xii.",
        "chapter i ",
        "chapter ii ",
        "chapter iii ",
        "chapter iv ",
        "chapter v ",
        "chapter vi ",
        "chapter vii ",
        "chapter viii ",
        "chapter ix ",
        "chapter x ",
        "chapter xi ",
        "chapter xii ",
        "chapter 1",
        "chapter 2",
        "chapter 3",
        "chapter 4",
        "chapter 5",
        "chapter 6",
        "chapter 7",
        "chapter 8",
        "chapter 9",
        "chapter 10",
    ];

    for (byte_idx, _) in html.char_indices() {
        if byte_idx > 200 {
            let b = html_bytes[byte_idx];
            if b != b'<' && b != b'c' && b != b'C' {
                continue;
            }
            let slice = &html[byte_idx..];
            for &tag in patterns {
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

    // Fallback: If no tag boundaries were found and the file is large, split into ~15KB chapter chunks at line breaks
    if boundaries.len() <= 1 && html.len() > 8000 {
        boundaries.clear();
        boundaries.push(0);
        let target_size = 15000;
        let mut pos = target_size;
        while pos < html.len() {
            while pos < html.len() && !html.is_char_boundary(pos) {
                pos += 1;
            }
            if pos >= html.len() {
                break;
            }
            let next_break = html[pos..]
                .find('\n')
                .or_else(|| html[pos..].find("<p"))
                .or_else(|| html[pos..].find("<P"))
                .or_else(|| html[pos..].find("<div"))
                .or_else(|| html[pos..].find("<DIV"));
            if let Some(break_offset) = next_break {
                let mut abs = pos + break_offset;
                while abs < html.len() && !html.is_char_boundary(abs) {
                    abs += 1;
                }
                boundaries.push(abs);
                pos = abs + target_size;
            } else {
                break;
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
    }

    let mut parts = Vec::new();
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
        parts.push(html.trim().to_string());
    }

    parts
}

fn extract_images_from_lit_bytes(bytes: &[u8], archive: &mut EpubArchive) -> usize {
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

// ============================================================================
// ITOLITLS Container + LZX Decompressor + LIT Binary HTML Detokenizer
// ============================================================================

#[derive(Debug, Clone)]
struct LitDirEntry {
    name: String,
    section: usize,
    offset: usize,
    size: usize,
}

fn read_encint(buf: &[u8], pos: &mut usize) -> Option<usize> {
    let mut val = 0usize;
    while *pos < buf.len() {
        let b = buf[*pos];
        *pos += 1;
        val = val.checked_shl(7)?.checked_add((b & 0x7F) as usize)?;
        if (b & 0x80) == 0 {
            return Some(val);
        }
    }
    None
}

fn parse_itolitls_container(bytes: &[u8]) -> Result<Book, String> {
    if bytes.len() < 40 || &bytes[0..8] != b"ITOLITLS" {
        return Err("missing ITOLITLS header".to_string());
    }
    let num_pieces = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
    if num_pieces < 2 || 40 + num_pieces * 16 > bytes.len() {
        return Err("invalid piece table".to_string());
    }

    let mut pieces = Vec::with_capacity(num_pieces);
    let mut max_piece_end = 40 + num_pieces * 16;
    for i in 0..num_pieces {
        let p = 40 + i * 16;
        let off = u64::from_le_bytes(bytes[p..p + 8].try_into().map_err(|_| "piece off")?) as usize;
        let sz =
            u64::from_le_bytes(bytes[p + 8..p + 16].try_into().map_err(|_| "piece sz")?) as usize;
        let end = off.checked_add(sz).ok_or("piece overflow")?;
        if end > max_piece_end && end <= bytes.len() {
            max_piece_end = end;
        }
        pieces.push((off, sz));
    }

    let (ifcm_off, ifcm_sz) = pieces[1];
    if ifcm_off.checked_add(ifcm_sz).ok_or("ifcm overflow")? > bytes.len() || ifcm_sz < 32 {
        return Err("invalid IFCM bounds".to_string());
    }
    let ifcm = &bytes[ifcm_off..ifcm_off + ifcm_sz];
    if &ifcm[0..4] != b"IFCM" {
        return Err("missing IFCM magic".to_string());
    }
    let chunk_size = u32::from_le_bytes([ifcm[8], ifcm[9], ifcm[10], ifcm[11]]) as usize;
    let num_chunks = u32::from_le_bytes([ifcm[24], ifcm[25], ifcm[26], ifcm[27]]) as usize;
    if chunk_size < 64 {
        return Err("invalid IFCM chunk_size".to_string());
    }

    let mut entries: Vec<LitDirEntry> = Vec::new();
    let mut cpos = 32usize;
    for _ in 0..num_chunks {
        if cpos + chunk_size > ifcm.len() {
            break;
        }
        let chunk = &ifcm[cpos..cpos + chunk_size];
        cpos += chunk_size;
        if &chunk[0..4] != b"AOLL" {
            continue;
        }
        let quickref_len = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as usize;
        if quickref_len >= chunk.len() || chunk.len() < 48 {
            continue;
        }
        let limit = chunk.len() - quickref_len;
        let mut ep = 48usize;
        while ep < limit {
            let Some(nlen) = read_encint(chunk, &mut ep) else {
                break;
            };
            if nlen == 0 || ep + nlen > limit {
                break;
            }
            let name = String::from_utf8_lossy(&chunk[ep..ep + nlen]).into_owned();
            ep += nlen;
            let Some(section) = read_encint(chunk, &mut ep) else {
                break;
            };
            let Some(offset) = read_encint(chunk, &mut ep) else {
                break;
            };
            let Some(size) = read_encint(chunk, &mut ep) else {
                break;
            };
            entries.push(LitDirEntry {
                name,
                section,
                offset,
                size,
            });
        }
    }

    let sec0_base = max_piece_end;
    let read_sec0 = |name: &str| -> Option<&[u8]> {
        let ent = entries.iter().find(|e| e.section == 0 && e.name == name)?;
        let start = sec0_base.checked_add(ent.offset)?;
        let end = start.checked_add(ent.size)?;
        bytes.get(start..end)
    };

    // Decompress Section 1 (MSCompressed) if present
    let mut sec1_data: Vec<u8> = Vec::new();
    if let Some(comp_bytes) = read_sec0("::DataSpace/Storage/MSCompressed/Content") {
        let mut window_bits = 17u32;
        let mut reset_interval = 0usize;
        if let Some(ctrl) = read_sec0("::DataSpace/Storage/MSCompressed/ControlData")
            && ctrl.len() >= 20
            && &ctrl[4..8] == b"LZXC"
        {
            let rst_blocks = u32::from_le_bytes([ctrl[12], ctrl[13], ctrl[14], ctrl[15]]) as usize;
            reset_interval = rst_blocks.saturating_mul(0x8000);
            let win_blocks = u32::from_le_bytes([ctrl[16], ctrl[17], ctrl[18], ctrl[19]]);
            let win_bytes = win_blocks.saturating_mul(0x8000).clamp(0x8000, 0x200000);
            window_bits = win_bytes.trailing_zeros();
        }
        let mut uncomp_len = 0usize;
        if let Some(rtbl) = read_sec0(
            "::DataSpace/Storage/MSCompressed/Transform/{0A9007C6-4076-11D3-8789-0000F8105754}/InstanceData/ResetTable",
        ) && rtbl.len() >= 24
        {
            uncomp_len =
                u64::from_le_bytes(rtbl[16..24].try_into().map_err(|_| "rtbl len")?) as usize;
        }
        if uncomp_len > 0 {
            sec1_data = lzx_decompress(comp_bytes, uncomp_len, window_bits, reset_interval)
                .map_err(|e| format!("LZX decompression failed: {e}"))?;
        }
    }

    let read_entry = |ent: &LitDirEntry| -> Option<&[u8]> {
        if ent.section == 0 {
            let start = sec0_base.checked_add(ent.offset)?;
            let end = start.checked_add(ent.size)?;
            bytes.get(start..end)
        } else if ent.section == 1 {
            let end = ent.offset.checked_add(ent.size)?;
            sec1_data.get(ent.offset..end)
        } else {
            None
        }
    };

    // Parse /manifest for ordered spine & assets, or fall back to /data/*/content
    let mut spine_ids: Vec<(String, String, String)> = Vec::new(); // (id, href, mime)
    if let Some(manifest_bytes) = read_sec0("/manifest") {
        spine_ids = parse_lit_manifest(manifest_bytes);
    }

    let mut title = "LIT Document".to_string();
    let mut creators = Vec::new();
    let mut language = "en".to_string();
    if let Some(meta_bytes) = read_sec0("/meta") {
        let (t, c, l) = parse_lit_meta(meta_bytes);
        if let Some(t) = t {
            title = t;
        }
        if !c.is_empty() {
            creators = c;
        }
        if let Some(l) = l {
            language = l;
        }
    }
    if creators.is_empty() {
        creators.push("Unknown Author".to_string());
    }

    let mut sections = Vec::new();
    let mut spine = Vec::new();
    let mut toc = Vec::new();
    let mut archive = EpubArchive::empty();

    if !spine_ids.is_empty() {
        for (id, href, mime) in &spine_ids {
            let data_path = format!("/data/{id}/content");
            let alt_path = format!("/data/{id}");
            let content_bytes = entries
                .iter()
                .find(|e| e.name == data_path || e.name == alt_path)
                .and_then(read_entry);
            let Some(raw) = content_bytes else {
                continue;
            };
            if mime.contains("html") || mime.contains("xml") {
                let html = decode_lit_content_stream(raw);
                let plain_text = extract_plain_text(&html);
                let plain_text_lower = plain_text.to_lowercase();
                let char_count = plain_text.chars().count();
                let idx = sections.len();
                sections.push(Section {
                    index: idx,
                    idref: id.clone(),
                    href: href.clone(),
                    full_path: href.clone(),
                    raw_html: html.clone(),
                    processed_html: html,
                    plain_text,
                    plain_text_lower,
                    char_count,
                    viewport_width: None,
                    viewport_height: None,
                });
                spine.push(SpineItem {
                    index: idx,
                    idref: id.clone(),
                    href: href.clone(),
                    linear: true,
                    media_type: mime.clone(),
                    properties: Vec::new(),
                });
                toc.push(NavPoint {
                    id: format!("toc_{idx}"),
                    label: format!("Section {}", idx + 1),
                    href: href.clone(),
                    full_path: href.clone(),
                    subitems: Vec::new(),
                });
            } else {
                archive.insert(format!("OEBPS/{href}"), raw.to_vec());
            }
        }
    }

    let total_chars: usize = sections.iter().map(|s| s.char_count).sum();
    if sections.is_empty() || total_chars == 0 {
        return Err(format!(
            "empty LIT text: spine_ids={} sections={} entries={} sec1_len={}",
            spine_ids.len(),
            sections.len(),
            entries.len(),
            sec1_data.len()
        ));
    }

    let metadata = Metadata {
        title,
        creators,
        publishers: Vec::new(),
        languages: vec![language],
        rights: None,
        description: None,
        identifier: None,
        pub_date: None,
        modified_date: None,
        subjects: Vec::new(),
        cover_id: None,
        cover_href: None,
        direction: PageProgressionDirection::Ltr,
        meta_properties: AHashMap::new(),
        accessibility: Default::default(),
    };

    let opf = OpfPackage {
        version: "2.0".to_string(),
        opf_path: "content.opf".to_string(),
        opf_dir: String::new(),
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

fn parse_lit_manifest(data: &[u8]) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    if data.len() < 6 {
        return out;
    }
    let mut pos = 2usize; // skip leading 2-byte header (e.g. 0x01, num_dirs/flag)
    while pos + 4 <= data.len() {
        let count =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;
        for _ in 0..count {
            if pos + 4 > data.len() {
                return out;
            }
            pos += 4; // dir_offset
            if pos >= data.len() {
                return out;
            }
            let id_len = data[pos] as usize;
            pos += 1;
            if pos + id_len > data.len() {
                return out;
            }
            let id = String::from_utf8_lossy(&data[pos..pos + id_len]).into_owned();
            pos += id_len;

            if pos >= data.len() {
                return out;
            }
            let href_len = data[pos] as usize;
            pos += 1;
            if pos + href_len > data.len() {
                return out;
            }
            let href = String::from_utf8_lossy(&data[pos..pos + href_len]).into_owned();
            pos += href_len;

            if pos >= data.len() {
                return out;
            }
            let mime_len = data[pos] as usize;
            pos += 1;
            if pos + mime_len > data.len() {
                return out;
            }
            let mime = String::from_utf8_lossy(&data[pos..pos + mime_len]).into_owned();
            pos += mime_len;

            if pos < data.len() {
                pos += 1; // trailing 0x00
            }
            out.push((id, href, mime));
        }
    }
    out
}

fn parse_lit_meta(data: &[u8]) -> (Option<String>, Vec<String>, Option<String>) {
    // /meta contains UTF-8 strings for dc:title, author, language, etc.
    // Extract title from <rdf:li xml:lang="x-default"> or length-prefixed Dublin Core fields.
    let s = String::from_utf8_lossy(data);
    let title = extract_tag_content(&s, "dc:title")
        .or_else(|| {
            // Binary XML encoded `dc:title\x00<title>\x00`
            data.windows(9)
                .position(|w| w == b"dc:title\x00")
                .and_then(|pos| {
                    let start = pos + 9;
                    let end = data[start..].iter().position(|&b| b == 0)? + start;
                    let t = std::str::from_utf8(&data[start..end]).ok()?.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                })
        })
        .or_else(|| {
            // Calibre /meta stores `\x01\x02\x00<title>\x00`
            data.windows(3)
                .position(|w| w == [0x01, 0x02, 0x00])
                .and_then(|pos| {
                    let start = pos + 3;
                    let end = data[start..].iter().position(|&b| b == 0)? + start;
                    let t = std::str::from_utf8(&data[start..end]).ok()?.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                })
        });
    let mut creators = Vec::new();
    if let Some(pos) = data.windows(4).position(|w| w == b"aut\x00") {
        let start = pos + 4;
        if let Some(rel_end) = data[start..].iter().position(|&b| b == 0)
            && let Ok(a) = std::str::from_utf8(&data[start..start + rel_end])
        {
            let a = a.trim();
            if !a.is_empty() {
                creators.push(a.to_string());
            }
        }
    } else if let Some(pos) = data.windows(11).position(|w| w == b"dc:creator\x00") {
        let start = pos + 11;
        if let Some(rel_end) = data[start..].iter().position(|&b| b == 0)
            && let Ok(a) = std::str::from_utf8(&data[start..start + rel_end])
        {
            let a = a.trim();
            if !a.is_empty() {
                creators.push(a.to_string());
            }
        }
    }
    let lang = data
        .windows(3)
        .position(|w| w == [0x01, 0x1f, 0x00])
        .and_then(|pos| {
            let start = pos + 3;
            let end = data[start..].iter().position(|&b| b == 0)? + start;
            let l = std::str::from_utf8(&data[start..end]).ok()?.trim();
            if l.is_empty() {
                None
            } else {
                Some(l.to_string())
            }
        });
    (title, creators, lang)
}

fn decode_lit_content_stream(raw: &[u8]) -> String {
    // If already plain HTML/XML, return directly
    let trimmed = raw
        .iter()
        .copied()
        .skip_while(|b| b.is_ascii_whitespace())
        .take(5)
        .collect::<Vec<u8>>();
    if trimmed.starts_with(b"<") {
        return String::from_utf8_lossy(raw).into_owned();
    }

    // LIT binary HTML stream:
    // - `0x00 0x00` closes the current element.
    // - `0x00 <flags!=0> ... 0x00` opens an element (with tag + attribute tokens inside).
    // - Text runs sit outside tag headers. `<head>`, `<title>`, `<style>`, `<script>` text is skipped.
    //   In Calibre LIT files, `<head>` ends at `U+FFFF` (`\xEF\xBF\xBF`) or when `<body>` opens.
    let mut out = String::from("<p>");
    let mut i = 0usize;

    while i < raw.len() {
        if raw[i] == 0x00 {
            if i + 1 < raw.len() && raw[i + 1] == 0x00 {
                // Closing tag
                if !out.ends_with("<p>") && !out.ends_with("</p>") {
                    out.push_str("</p><p>");
                }
                i += 2;
                continue;
            }
            // Opening tag: `0x00 <flags> ... 0x00`
            i += 2;
            while i < raw.len() && raw[i] != 0x00 {
                i += 1;
            }
            if i < raw.len() && raw[i] == 0x00 {
                i += 1;
            }
            continue;
        }

        // Skip U+FFFF marker
        if i + 3 <= raw.len() && raw[i..i + 3] == [0xEF, 0xBF, 0xBF] {
            i += 3;
            continue;
        }

        // Read UTF-8 char
        let ch = if raw[i] < 0x80 {
            let c = raw[i] as char;
            i += 1;
            c
        } else {
            match std::str::from_utf8(&raw[i..]) {
                Ok(s) => {
                    let c = s.chars().next().unwrap();
                    i += c.len_utf8();
                    c
                }
                Err(e) => {
                    if e.valid_up_to() > 0 {
                        // SAFETY: std::str::from_utf8 guarantees `raw[i..i + e.valid_up_to()]` is valid UTF-8.
                        let s =
                            unsafe { std::str::from_utf8_unchecked(&raw[i..i + e.valid_up_to()]) };
                        let c = s.chars().next().unwrap();
                        i += c.len_utf8();
                        c
                    } else {
                        i += 1;
                        continue;
                    }
                }
            }
        };

        if ch >= ' ' || ch == '\n' || ch == '\r' || ch == '\t' {
            match ch {
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '&' => out.push_str("&amp;"),
                _ => out.push(ch),
            }
        }
    }
    if out.ends_with("<p>") {
        out.truncate(out.len() - 3);
    } else {
        out.push_str("</p>");
    }
    out
}

fn lzx_decompress(
    src: &[u8],
    out_len: usize,
    window_bits: u32,
    reset_interval: usize,
) -> Result<Vec<u8>, String> {
    struct BitReader<'a> {
        src: &'a [u8],
        pos: usize,
        buf: u32,
        bits: u32,
    }
    impl<'a> BitReader<'a> {
        fn new(src: &'a [u8]) -> Self {
            Self {
                src,
                pos: 0,
                buf: 0,
                bits: 0,
            }
        }
        fn ensure(&mut self, n: u32) {
            while self.bits < n {
                let w = if self.pos + 1 < self.src.len() {
                    let v = u16::from_le_bytes([self.src[self.pos], self.src[self.pos + 1]]) as u32;
                    self.pos += 2;
                    v
                } else if self.pos < self.src.len() {
                    let v = self.src[self.pos] as u32;
                    self.pos += 1;
                    v
                } else {
                    0
                };
                self.buf |= w << (16 - self.bits);
                self.bits += 16;
            }
        }
        fn read_bits(&mut self, n: u32) -> u32 {
            if n == 0 {
                return 0;
            }
            if n > 16 {
                let hi = self.read_bits(n - 16);
                let lo = self.read_bits(16);
                return (hi << 16) | lo;
            }
            self.ensure(n);
            let v = self.buf >> (32 - n);
            self.buf <<= n;
            self.bits -= n;
            v
        }
        fn peek_bits(&mut self, n: u32) -> u32 {
            self.ensure(n);
            self.buf >> (32 - n)
        }
        fn consume(&mut self, n: u32) {
            self.buf <<= n;
            self.bits -= n;
        }
        fn align_16(&mut self) {
            let rem = self.bits & 15;
            if rem > 0 {
                self.consume(rem);
            }
            if self.bits >= 16 {
                let words = (self.bits / 16) as usize;
                self.pos = self.pos.saturating_sub(words * 2);
                self.bits = 0;
                self.buf = 0;
            }
        }
        fn read_u32_le_aligned(&mut self) -> u32 {
            if self.pos + 4 <= self.src.len() {
                let v = u32::from_le_bytes([
                    self.src[self.pos],
                    self.src[self.pos + 1],
                    self.src[self.pos + 2],
                    self.src[self.pos + 3],
                ]);
                self.pos += 4;
                v
            } else {
                0
            }
        }
    }

    struct HuffTable {
        table: Vec<u16>, // (len << 11) | sym, 12-bit lookup
        lens: Vec<u8>,
        codes: Vec<u32>,
    }
    impl HuffTable {
        fn build(lens: &[u8]) -> Result<Self, String> {
            let max_bits = 16usize;
            let mut bl_count = [0u32; 17];
            for &l in lens {
                if (l as usize) > max_bits {
                    return Err(format!("huff len {l} > 16"));
                }
                if l > 0 {
                    bl_count[l as usize] += 1;
                }
            }
            let mut next_code = [0u32; 17];
            let mut code = 0u32;
            for bits in 1..=max_bits {
                code = (code + bl_count[bits - 1]) << 1;
                next_code[bits] = code;
            }
            let mut codes = vec![0u32; lens.len()];
            let mut table = vec![0u16; 1 << 12];
            for (sym, &len) in lens.iter().enumerate() {
                if len == 0 {
                    continue;
                }
                let c = next_code[len as usize];
                next_code[len as usize] += 1;
                codes[sym] = c;
                if len <= 12 {
                    let shift = 12 - len as usize;
                    let base = (c as usize) << shift;
                    let entry = ((len as u16) << 11) | (sym as u16);
                    for fill in 0..(1usize << shift) {
                        if base + fill < table.len() {
                            table[base + fill] = entry;
                        }
                    }
                }
            }
            Ok(Self {
                table,
                lens: lens.to_vec(),
                codes,
            })
        }
        fn decode(&self, br: &mut BitReader<'_>) -> Result<usize, String> {
            let p = br.peek_bits(12) as usize;
            let entry = self.table[p];
            if entry != 0 {
                let len = (entry >> 11) as u32;
                let sym = (entry & 0x7FF) as usize;
                br.consume(len);
                return Ok(sym);
            }
            let full = br.peek_bits(16);
            for (sym, &len) in self.lens.iter().enumerate() {
                if len > 12 {
                    let shift = 16 - len as u32;
                    if (full >> shift) == self.codes[sym] {
                        br.consume(len as u32);
                        return Ok(sym);
                    }
                }
            }
            Err(format!(
                "huff decode miss at src_pos={} bits={}",
                br.pos, br.bits
            ))
        }
    }

    fn read_lengths(
        br: &mut BitReader<'_>,
        lens: &mut [u8],
        first: usize,
        last: usize,
    ) -> Result<(), String> {
        let mut pre_lens = [0u8; 20];
        for pl in &mut pre_lens {
            *pl = br.read_bits(4) as u8;
        }
        let pre_tree = HuffTable::build(&pre_lens)?;
        let mut i = first;
        while i < last {
            let sym = pre_tree.decode(br)?;
            match sym {
                0..=16 => {
                    lens[i] = ((lens[i] as usize + 17 - sym) % 17) as u8;
                    i += 1;
                }
                17 => {
                    let run = 4 + br.read_bits(4) as usize;
                    for _ in 0..run {
                        if i >= last {
                            break;
                        }
                        lens[i] = 0;
                        i += 1;
                    }
                }
                18 => {
                    let run = 20 + br.read_bits(5) as usize;
                    for _ in 0..run {
                        if i >= last {
                            break;
                        }
                        lens[i] = 0;
                        i += 1;
                    }
                }
                19 => {
                    let run = 4 + br.read_bits(1) as usize;
                    let s2 = pre_tree.decode(br)?;
                    if s2 > 16 {
                        return Err(format!(
                            "pretree sym19 s2={s2} i={i}/{last} src_pos={}",
                            br.pos
                        ));
                    }
                    let val = ((lens[i] as usize + 17 - s2) % 17) as u8;
                    for _ in 0..run {
                        if i >= last {
                            break;
                        }
                        lens[i] = val;
                        i += 1;
                    }
                }
                _ => return Err(format!("pretree bad sym={sym}")),
            }
        }
        Ok(())
    }

    let num_pos_slots = match window_bits {
        20 => 42,
        21 => 50,
        _ => (window_bits as usize) * 2,
    };
    let mut pos_base = vec![0usize; num_pos_slots];
    let mut pos_extra = vec![0u32; num_pos_slots];
    let mut base = 0usize;
    for slot in 0..num_pos_slots {
        let extra = if slot < 4 {
            0
        } else {
            ((slot / 2) - 1).min(17) as u32
        };
        pos_extra[slot] = extra;
        pos_base[slot] = base;
        base += 1usize << extra;
    }

    let main_elems = 256 + num_pos_slots * 8;
    let mut main_lens = vec![0u8; main_elems];
    let mut length_lens = vec![0u8; 249];

    let mut br = BitReader::new(src);
    let e8_flag = br.read_bits(1);
    let _e8_filesize = if e8_flag != 0 {
        let hi = br.read_bits(16);
        let lo = br.read_bits(16);
        (hi << 16) | lo
    } else {
        0
    };

    let mut out = Vec::with_capacity(out_len);
    let mut r0 = 1usize;
    let mut r1 = 1usize;
    let mut r2 = 1usize;
    let mut block_type = 0u32;
    let mut block_rem = 0usize;
    let mut main_tree = HuffTable::build(&main_lens)?;
    let mut length_tree = HuffTable::build(&length_lens)?;
    let mut aligned_tree = HuffTable::build(&[0u8; 8])?;
    let mut block_log = String::new();

    while out.len() < out_len {
        let frame_target = (out.len() + 0x8000).min(out_len);
        while out.len() < frame_target {
            if block_rem == 0 {
                if block_type == 3 && (out.len() & 1) != 0 {
                    // padding handled after uncompressed block
                }
                block_type = br.read_bits(3);
                block_rem = br.read_bits(24) as usize;
                use std::fmt::Write as _;
                let _ = write!(
                    block_log,
                    "[out={},pos={},bt={},rem={}] ",
                    out.len(),
                    br.pos,
                    block_type,
                    block_rem
                );
                if block_rem == 0 {
                    return Err(format!("block_rem=0 at out={} log={block_log}", out.len()));
                }
                match block_type {
                    1 => {
                        read_lengths(&mut br, &mut main_lens, 0, 256)
                            .map_err(|e| format!("{e} log={block_log}"))?;
                        read_lengths(&mut br, &mut main_lens, 256, main_elems)
                            .map_err(|e| format!("{e} log={block_log}"))?;
                        read_lengths(&mut br, &mut length_lens, 0, 249)
                            .map_err(|e| format!("{e} log={block_log}"))?;
                        main_tree = HuffTable::build(&main_lens)?;
                        length_tree = HuffTable::build(&length_lens)?;
                    }
                    2 => {
                        let mut alens = [0u8; 8];
                        for al in &mut alens {
                            *al = br.read_bits(3) as u8;
                        }
                        aligned_tree = HuffTable::build(&alens)?;
                        read_lengths(&mut br, &mut main_lens, 0, 256).map_err(|e| {
                            let tail =
                                String::from_utf8_lossy(&out[out.len().saturating_sub(120)..]);
                            format!("{e} log={block_log} tail={tail:?}")
                        })?;
                        read_lengths(&mut br, &mut main_lens, 256, main_elems)?;
                        read_lengths(&mut br, &mut length_lens, 0, 249)?;
                        main_tree = HuffTable::build(&main_lens)?;
                        length_tree = HuffTable::build(&length_lens)?;
                    }
                    3 => {
                        br.align_16();
                        r0 = br.read_u32_le_aligned() as usize;
                        r1 = br.read_u32_le_aligned() as usize;
                        r2 = br.read_u32_le_aligned() as usize;
                    }
                    _ => return Err(format!("bad block_type={block_type} at out={}", out.len())),
                }
            }

            let step_limit = (frame_target - out.len()).min(block_rem);
            if block_type == 3 {
                let avail = br.src.len().saturating_sub(br.pos).min(step_limit);
                out.extend_from_slice(&br.src[br.pos..br.pos + avail]);
                br.pos += avail;
                block_rem -= avail;
                if block_rem == 0 && (avail & 1) != 0 && br.pos < br.src.len() {
                    br.pos += 1;
                }
                continue;
            }

            let mut decoded = 0usize;
            while decoded < step_limit {
                let sym = main_tree.decode(&mut br)?;
                if sym < 256 {
                    out.push(sym as u8);
                    decoded += 1;
                } else {
                    let mh = sym - 256;
                    let len_hdr = mh & 7;
                    let match_len = if len_hdr == 7 {
                        7 + length_tree.decode(&mut br)?
                    } else {
                        len_hdr
                    } + 2;
                    let pos_slot = mh >> 3;
                    let match_offset = match pos_slot {
                        0 => r0,
                        1 => {
                            std::mem::swap(&mut r0, &mut r1);
                            r0
                        }
                        2 => {
                            std::mem::swap(&mut r0, &mut r2);
                            r0
                        }
                        _ => {
                            let extra = pos_extra[pos_slot];
                            let formatted = if block_type == 2 && extra >= 3 {
                                let vbits = br.read_bits(extra - 3) as usize;
                                let abits = aligned_tree.decode(&mut br)?;
                                pos_base[pos_slot] + (vbits << 3) + abits
                            } else {
                                pos_base[pos_slot] + (br.read_bits(extra) as usize)
                            };
                            let off = formatted.saturating_sub(2);
                            r2 = r1;
                            r1 = r0;
                            r0 = off;
                            off
                        }
                    };
                    if match_offset == 0 || match_offset > out.len() {
                        return Err(format!(
                            "bad match_offset={match_offset} out={} block_type={block_type} pos_slot={pos_slot}",
                            out.len()
                        ));
                    }
                    for _ in 0..match_len {
                        let b = out[out.len() - match_offset];
                        out.push(b);
                    }
                    decoded += match_len;
                }
            }
            block_rem = block_rem.saturating_sub(decoded);
        }
        br.align_16();
        if reset_interval > 0 && out.len() < out_len && out.len() % reset_interval == 0 {
            r0 = 1;
            r1 = 1;
            r2 = 1;
            main_lens.fill(0);
            length_lens.fill(0);
            block_rem = 0;
            if br.read_bits(1) != 0 {
                let _ = br.read_bits(16);
                let _ = br.read_bits(16);
            }
        }
        use std::fmt::Write as _;
        let _ = write!(
            block_log,
            "[FRAME out={},pos={},bits={}] ",
            out.len(),
            br.pos,
            br.bits
        );
    }
    out.truncate(out_len);
    Ok(out)
}

/// Export any `Book` to a Microsoft Reader (`.lit`) `ITOLITLS` binary eBook with LZX-framed content.
pub fn export_lit_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let meta = book.metadata();
    let title = if meta.title.trim().is_empty() {
        "Untitled"
    } else {
        meta.title.trim()
    };
    let author = meta
        .creators
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("Unknown Author");
    let lang = if meta.language().is_empty() {
        "en"
    } else {
        meta.language()
    };

    // 1. Build Section 1 uncompressed stream from all spine sections
    let mut sec1_plain = Vec::new();
    let mut manifest_items: Vec<(String, String, String, usize, usize)> = Vec::new();
    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let html_str = if !sec.raw_html.trim().is_empty() {
            sec.raw_html.clone()
        } else {
            format!(
                "<html><head><title>{}</title></head><body><p>{}</p></body></html>",
                crate::dom::xml_escape(title),
                crate::dom::xml_escape(&sec.plain_text)
            )
        };
        let encoded = encode_lit_binary_xml(&html_str);
        let off = sec1_plain.len();
        let sz = encoded.len();
        sec1_plain.extend_from_slice(&encoded);
        let id = format!("sec_{idx}");
        let href = format!("sec_{idx}.html");
        manifest_items.push((id, href, "application/xhtml+xml".to_string(), off, sz));
    }
    if manifest_items.is_empty() {
        let fallback = encode_lit_binary_xml("<html><body><p></p></body></html>");
        sec1_plain.extend_from_slice(&fallback);
        manifest_items.push((
            "sec_0".to_string(),
            "sec_0.html".to_string(),
            "application/xhtml+xml".to_string(),
            0,
            fallback.len(),
        ));
    }

    // 2. Encode Section 1 as valid LZX Type 3 (Uncompressed) blocks, one per 0x8000 frame,
    //    with 1-bit e8_flag=0 at reset boundaries (every 4 frames = 128 KB).
    let mut lzx_stream = Vec::new();
    let mut reset_offsets = Vec::new();
    let mut pos = 0usize;
    let mut frame_idx = 0usize;
    while pos < sec1_plain.len() {
        let chunk_end = (pos + 0x8000).min(sec1_plain.len());
        let chunk = &sec1_plain[pos..chunk_end];
        let block_len = chunk.len() as u32;
        reset_offsets.push(lzx_stream.len() as u64);
        let (w0, w1) = if frame_idx.is_multiple_of(4) {
            // 1-bit e8_flag(0) + 3-bit type(3) + 24-bit block_len + 4-bit pad(0)
            (
                ((3u32 << 12) | ((block_len >> 12) & 0x0FFF)) as u16,
                ((block_len & 0x0FFF) << 4) as u16,
            )
        } else {
            // 3-bit type(3) + 24-bit block_len + 5-bit pad(0)
            (
                ((3u32 << 13) | ((block_len >> 11) & 0x1FFF)) as u16,
                ((block_len & 0x07FF) << 5) as u16,
            )
        };
        lzx_stream.extend_from_slice(&w0.to_le_bytes());
        lzx_stream.extend_from_slice(&w1.to_le_bytes());
        lzx_stream.extend_from_slice(&1u32.to_le_bytes()); // R0
        lzx_stream.extend_from_slice(&1u32.to_le_bytes()); // R1
        lzx_stream.extend_from_slice(&1u32.to_le_bytes()); // R2
        lzx_stream.extend_from_slice(chunk);
        if (block_len & 1) != 0 {
            lzx_stream.push(0);
        }
        pos = chunk_end;
        frame_idx += 1;
    }

    // 3. Build /manifest, /meta, /Version, and ::DataSpace/* Section 0 blobs
    let mut manifest = Vec::new();
    manifest.push(0x01);
    manifest.push((manifest_items.len().min(255)) as u8);
    manifest.extend_from_slice(&(manifest_items.len() as u32).to_le_bytes());
    for (id, href, mime, _, _) in &manifest_items {
        manifest.extend_from_slice(&0u32.to_le_bytes());
        let id_b = id.as_bytes();
        let href_b = href.as_bytes();
        let mime_b = mime.as_bytes();
        manifest.push(id_b.len().min(255) as u8);
        manifest.extend_from_slice(&id_b[..id_b.len().min(255)]);
        manifest.push(href_b.len().min(255) as u8);
        manifest.extend_from_slice(&href_b[..href_b.len().min(255)]);
        manifest.push(mime_b.len().min(255) as u8);
        manifest.extend_from_slice(&mime_b[..mime_b.len().min(255)]);
        manifest.push(0x00);
    }
    // Empty non-spine / asset / guide sections (3 x u32 count = 0)
    manifest.extend_from_slice(&[0u8; 12]);

    let mut opf_manifest = String::new();
    let mut opf_spine = String::new();
    for (id, href, mime, _, _) in &manifest_items {
        use std::fmt::Write as _;
        let _ = write!(
            opf_manifest,
            "<item id=\"{}\" href=\"{}\" media-type=\"{}\"></item>",
            crate::dom::xml_escape(id),
            crate::dom::xml_escape(href),
            crate::dom::xml_escape(mime),
        );
        let _ = write!(
            opf_spine,
            "<itemref idref=\"{}\"></itemref>",
            crate::dom::xml_escape(id),
        );
    }
    let opf_meta_xml = format!(
        "<package xmlns=\"http://openebook.org/namespaces/oeb-package/1.0/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" unique-identifier=\"uid\"><metadata><dc:title>{}</dc:title><dc:creator>{}</dc:creator><dc:language>{}</dc:language><dc:identifier id=\"uid\">urn:uuid:ebook-rs-lit</dc:identifier></metadata><manifest>{}</manifest><spine>{}</spine></package>",
        crate::dom::xml_escape(title),
        crate::dom::xml_escape(author),
        crate::dom::xml_escape(lang),
        opf_manifest,
        opf_spine,
    );
    let meta_blob = encode_lit_binary_xml(&opf_meta_xml);

    let mut ctrl_data = Vec::with_capacity(32);
    for v in [7u32, u32::from_le_bytes(*b"LZXC"), 3, 4, 4, 2, 0, 0] {
        ctrl_data.extend_from_slice(&v.to_le_bytes());
    }

    let mut rtbl = Vec::with_capacity(40 + reset_offsets.len() * 8);
    rtbl.extend_from_slice(&3u32.to_le_bytes());
    rtbl.extend_from_slice(&(reset_offsets.len() as u32).to_le_bytes());
    rtbl.extend_from_slice(&8u32.to_le_bytes());
    rtbl.extend_from_slice(&40u32.to_le_bytes());
    rtbl.extend_from_slice(&(sec1_plain.len() as u64).to_le_bytes());
    rtbl.extend_from_slice(&(lzx_stream.len() as u64).to_le_bytes());
    rtbl.extend_from_slice(&0x8000u32.to_le_bytes());
    rtbl.extend_from_slice(&0u32.to_le_bytes());
    for roff in reset_offsets {
        rtbl.extend_from_slice(&roff.to_le_bytes());
    }

    let mut span_info = Vec::with_capacity(8);
    span_info.extend_from_slice(&(sec1_plain.len() as u64).to_le_bytes());

    let mut sec0_data = Vec::new();
    let mut dir_entries: Vec<(String, usize, usize, usize)> = Vec::new();
    let mut push_sec0 = |name: &str, blob: &[u8]| {
        let off = sec0_data.len();
        sec0_data.extend_from_slice(blob);
        dir_entries.push((name.to_string(), 0, off, blob.len()));
    };

    let mut name_list = Vec::new();
    let names_utf16: [&str; 2] = ["Uncompressed", "MSCompressed"];
    let total_words: u16 = 2 + names_utf16.iter().map(|s| 2 + s.len() as u16).sum::<u16>();
    name_list.extend_from_slice(&total_words.to_le_bytes());
    name_list.extend_from_slice(&(names_utf16.len() as u16).to_le_bytes());
    for n in names_utf16 {
        name_list.extend_from_slice(&(n.len() as u16).to_le_bytes());
        for ch in n.encode_utf16() {
            name_list.extend_from_slice(&ch.to_le_bytes());
        }
        name_list.extend_from_slice(&0u16.to_le_bytes());
    }
    let transform_guid: [u8; 16] = [
        0xc6, 0x07, 0x90, 0x0a, 0x76, 0x40, 0xd3, 0x11, 0x87, 0x89, 0x00, 0x00, 0xf8, 0x10, 0x57,
        0x54,
    ];

    push_sec0("/manifest", &manifest);
    push_sec0("/meta", &meta_blob);
    push_sec0("/Version", &[1, 0, 0, 0]);
    push_sec0("::DataSpace/NameList", &name_list);
    push_sec0("::DataSpace/Storage/MSCompressed/Content", &lzx_stream);
    push_sec0("::DataSpace/Storage/MSCompressed/ControlData", &ctrl_data);
    push_sec0("::DataSpace/Storage/MSCompressed/SpanInfo", &span_info);
    push_sec0(
        "::DataSpace/Storage/MSCompressed/Transform/List",
        &transform_guid,
    );
    push_sec0(
        "::DataSpace/Storage/MSCompressed/Transform/{0A9007C6-4076-11D3-8789-0000F8105754}/InstanceData/ResetTable",
        &rtbl,
    );
    for (id, _, _, off, sz) in &manifest_items {
        dir_entries.push((format!("/data/{id}/content"), 1, *off, *sz));
    }
    dir_entries.sort_by(|a, b| a.0.cmp(&b.0));

    // 4. Pack IFCM directory chunks (0x2000 bytes per AOLL chunk)
    fn write_encint(out: &mut Vec<u8>, mut v: usize) {
        let mut tmp = [0u8; 10];
        let mut n = 0;
        loop {
            tmp[n] = (v & 0x7F) as u8;
            n += 1;
            v >>= 7;
            if v == 0 {
                break;
            }
        }
        for i in (0..n).rev() {
            let b = tmp[i] | if i > 0 { 0x80 } else { 0 };
            out.push(b);
        }
    }

    let chunk_size = 0x2000usize;
    let mut aoll_chunks: Vec<Vec<u8>> = Vec::new();
    let mut cur_chunk = vec![0u8; chunk_size];
    cur_chunk[0..4].copy_from_slice(b"AOLL");
    let mut cur_pos = 48usize;
    let mut cur_count = 0u16;

    for (name, sec, off, sz) in &dir_entries {
        let mut rec = Vec::new();
        write_encint(&mut rec, name.len());
        rec.extend_from_slice(name.as_bytes());
        write_encint(&mut rec, *sec);
        write_encint(&mut rec, *off);
        write_encint(&mut rec, *sz);

        if cur_pos + rec.len() + 4 > chunk_size {
            cur_chunk[4..8].copy_from_slice(&2u32.to_le_bytes());
            cur_chunk[chunk_size - 2..chunk_size].copy_from_slice(&cur_count.to_le_bytes());
            aoll_chunks.push(cur_chunk);
            cur_chunk = vec![0u8; chunk_size];
            cur_chunk[0..4].copy_from_slice(b"AOLL");
            cur_pos = 48;
            cur_count = 0;
        }
        cur_chunk[cur_pos..cur_pos + rec.len()].copy_from_slice(&rec);
        cur_pos += rec.len();
        cur_count = cur_count.saturating_add(1);
    }
    cur_chunk[4..8].copy_from_slice(&2u32.to_le_bytes());
    cur_chunk[chunk_size - 2..chunk_size].copy_from_slice(&cur_count.to_le_bytes());
    aoll_chunks.push(cur_chunk);

    let total_chunks = aoll_chunks.len();
    for (ci, c) in aoll_chunks.iter_mut().enumerate() {
        let prev: u64 = if ci == 0 { u64::MAX } else { (ci - 1) as u64 };
        let next: u64 = if ci + 1 >= total_chunks {
            u64::MAX
        } else {
            (ci + 1) as u64
        };
        c[16..24].copy_from_slice(&prev.to_le_bytes());
        c[24..32].copy_from_slice(&next.to_le_bytes());
    }

    let mut ifcm = Vec::with_capacity(32 + aoll_chunks.len() * chunk_size);
    ifcm.extend_from_slice(b"IFCM");
    ifcm.extend_from_slice(&1u32.to_le_bytes());
    ifcm.extend_from_slice(&(chunk_size as u32).to_le_bytes());
    ifcm.extend_from_slice(&0x0010_0000u32.to_le_bytes());
    ifcm.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    ifcm.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    ifcm.extend_from_slice(&(aoll_chunks.len() as u32).to_le_bytes());
    ifcm.extend_from_slice(&0u32.to_le_bytes());
    for c in aoll_chunks {
        ifcm.extend_from_slice(&c);
    }

    // 5. Assemble ITOLITLS header + 5 pieces + secondary header + Section 0 data
    let hdr_len = 40usize;
    let num_pieces = 5usize;
    let hlen2 = 232usize;
    let p0_off = hdr_len + num_pieces * 16 + hlen2;
    let p0_sz = 24usize;
    let p1_off = p0_off + p0_sz;
    let p1_sz = ifcm.len();
    let mut ifcm2 = vec![0u8; 0x220];
    ifcm2[0..4].copy_from_slice(b"IFCM");
    ifcm2[4..8].copy_from_slice(&1u32.to_le_bytes());
    ifcm2[8..12].copy_from_slice(&0x0200u32.to_le_bytes());
    ifcm2[12..16].copy_from_slice(&0x0002_0000u32.to_le_bytes());
    ifcm2[16..20].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    ifcm2[20..24].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    ifcm2[24..28].copy_from_slice(&1u32.to_le_bytes());
    ifcm2[32..36].copy_from_slice(b"AOLL");
    ifcm2[36..40].copy_from_slice(&2u32.to_le_bytes());
    ifcm2[0x220 - 2..0x220].copy_from_slice(&0u16.to_le_bytes());

    let p2_off = p1_off + p1_sz;
    let p2_sz = ifcm2.len();
    let p3_off = p2_off + p2_sz;
    let p3_sz = 16usize;
    let p4_off = p3_off + p3_sz;
    let p4_sz = 16usize;
    let content_off = (p4_off + p4_sz) as u64;
    let total_file_size = content_off + sec0_data.len() as u64;

    let mut hdr2 = vec![0u8; hlen2];
    // Secondary header (0..0x98)
    hdr2[0..4].copy_from_slice(&2u32.to_le_bytes());
    hdr2[4..8].copy_from_slice(&0x98u32.to_le_bytes());
    hdr2[8..12].copy_from_slice(&2u32.to_le_bytes());
    hdr2[24..28].copy_from_slice(&1u32.to_le_bytes());
    hdr2[40..44].copy_from_slice(&0x2000u32.to_le_bytes());
    hdr2[44..48].copy_from_slice(&2u32.to_le_bytes());
    hdr2[52..56].copy_from_slice(&2u32.to_le_bytes());
    hdr2[64..68].copy_from_slice(&(dir_entries.len() as u32).to_le_bytes());
    hdr2[72..80].copy_from_slice(&u64::MAX.to_le_bytes());
    hdr2[104..108].copy_from_slice(&0x0200u32.to_le_bytes());
    hdr2[108..112].copy_from_slice(&2u32.to_le_bytes());
    hdr2[116..120].copy_from_slice(&1u32.to_le_bytes());
    hdr2[128..132].copy_from_slice(&2u32.to_le_bytes());
    hdr2[136..140].copy_from_slice(&0x0010_0000u32.to_le_bytes());
    hdr2[140..144].copy_from_slice(&0x0002_0000u32.to_le_bytes());
    // CAOL (0x98..0xC8)
    hdr2[152..156].copy_from_slice(b"CAOL");
    hdr2[156..160].copy_from_slice(&2u32.to_le_bytes());
    hdr2[160..164].copy_from_slice(&0x50u32.to_le_bytes());
    hdr2[164..168].copy_from_slice(&0x0003_1337u32.to_le_bytes());
    hdr2[172..176].copy_from_slice(&0x2000u32.to_le_bytes());
    hdr2[176..180].copy_from_slice(&0x0200u32.to_le_bytes());
    hdr2[180..184].copy_from_slice(&0x0010_0000u32.to_le_bytes());
    hdr2[184..188].copy_from_slice(&0x0002_0000u32.to_le_bytes());
    // ITSF (0xC8..0xE8)
    hdr2[200..204].copy_from_slice(b"ITSF");
    hdr2[204..208].copy_from_slice(&4u32.to_le_bytes());
    hdr2[208..212].copy_from_slice(&0x20u32.to_le_bytes());
    hdr2[212..216].copy_from_slice(&1u32.to_le_bytes());
    hdr2[216..224].copy_from_slice(&content_off.to_le_bytes());

    let mut piece0 = vec![0u8; p0_sz];
    piece0[0..4].copy_from_slice(&0x01FEu32.to_le_bytes());
    piece0[8..16].copy_from_slice(&total_file_size.to_le_bytes());

    let mut out = Vec::with_capacity(total_file_size as usize);
    out.extend_from_slice(b"ITOLITLS");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(hdr_len as u32).to_le_bytes());
    out.extend_from_slice(&(num_pieces as u32).to_le_bytes());
    out.extend_from_slice(&(hlen2 as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]); // GUID

    for (off, sz) in [
        (p0_off, p0_sz),
        (p1_off, p1_sz),
        (p2_off, p2_sz),
        (p3_off, p3_sz),
        (p4_off, p4_sz),
    ] {
        out.extend_from_slice(&(off as u64).to_le_bytes());
        out.extend_from_slice(&(sz as u64).to_le_bytes());
    }
    out.extend_from_slice(&hdr2);
    out.extend_from_slice(&piece0);
    out.extend_from_slice(&ifcm);
    out.extend_from_slice(&ifcm2);
    out.extend_from_slice(&vec![0u8; p3_sz + p4_sz]);
    out.extend_from_slice(&sec0_data);
    Ok(out)
}

fn encode_lit_binary_xml(xml: &str) -> Vec<u8> {
    use crate::dom::{DomNode, EbookDomTree, decode_html_entities};
    fn emit_nodes(nodes: &[DomNode], out: &mut Vec<u8>) {
        for node in nodes {
            match node {
                DomNode::Comment(_) => {}
                DomNode::Text(t) => {
                    let decoded = decode_html_entities(t);
                    out.extend_from_slice(decoded.replace('\0', "").as_bytes());
                }
                DomNode::Element {
                    tag_name,
                    attributes,
                    children,
                } => {
                    let tag_b = tag_name.as_bytes();
                    if tag_b.is_empty() || tag_b.len() > 250 || tag_b[0] == b'?' || tag_b[0] == b'!'
                    {
                        emit_nodes(children, out);
                        continue;
                    }
                    // Open tag with custom tag name:
                    // flags = 0x01 if element has children (closed with 0x00 0x02 0x00),
                    // flags = 0x03 if leaf/empty element (no close tag).
                    let has_children = !children.is_empty();
                    out.push(0x00);
                    out.push(if has_children { 0x01 } else { 0x03 });
                    out.extend_from_slice(&[0xE8, 0x80, 0x80]);
                    out.push((tag_b.len() + 1) as u8);
                    out.extend_from_slice(tag_b);
                    // Custom attributes: U+8000 (E8 80 80), len+1, name, len+1, value
                    for (k, v) in attributes {
                        let kb = k.as_bytes();
                        let vb = v.as_bytes();
                        if !kb.is_empty() && kb.len() <= 250 && vb.len() <= 250 {
                            out.extend_from_slice(&[0xE8, 0x80, 0x80]);
                            out.push((kb.len() + 1) as u8);
                            out.extend_from_slice(kb);
                            out.push((vb.len() + 1) as u8);
                            out.extend_from_slice(vb);
                        }
                    }
                    out.push(0x00); // end tag header
                    if has_children {
                        emit_nodes(children, out);
                        out.push(0x00);
                        out.push(0x02);
                        out.push(0x00); // close tag
                    }
                }
            }
        }
    }
    let dom = EbookDomTree::parse(xml);
    let mut out = Vec::with_capacity(xml.len());
    emit_nodes(&dom.root_nodes, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::{LitBook, export_lit_bytes};
    use crate::book::Book;

    #[test]
    fn test_lit_export_and_parse_roundtrip() {
        let epub = crate::generate_sample_epub().expect("sample epub");
        let book = Book::from_bytes(&epub).expect("load sample epub");
        let lit_bytes = export_lit_bytes(&book).expect("export lit");
        assert!(lit_bytes.starts_with(b"ITOLITLS"));
        let parsed = LitBook::parse(&lit_bytes).expect("parse exported lit");
        assert_eq!(parsed.metadata().title, book.metadata().title);
        assert_eq!(parsed.spine().len(), book.spine().len());
        let txt = parsed.export_txt_bytes().expect("export txt");
        let txt_str = String::from_utf8(txt).expect("utf8");
        assert!(txt_str.contains("Welcome to EBook-RS"));
        assert!(txt_str.contains("Canonical Fragment Identifiers"));
    }
}
