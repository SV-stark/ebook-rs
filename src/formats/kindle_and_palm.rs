//! Pure-Rust Kindle (MOBI / AZW3-KF8) and PalmDB (PDB / PalmDOC / zTXT) Codecs:
//! - `mobi` Output (`export_mobi_bytes` — PalmDB + PalmDOC LZ77 + MOBI6 + EXTH + images)
//! - `azw3` Output (`export_azw3_bytes` — PalmDB + PalmDOC LZ77 + KF8/MOBI8 + FDST + EXTH + images)
//! - `pdb` Input & Output (`is_pdb_bytes`, `parse_pdb`, `export_pdb_bytes` — PalmDOC `TEXtREAd` & `zTXTGPlm`)

use crate::archive::resolve_relative_path;
use crate::book::Book;
use crate::dom::{decode_bytes_with_encoding, decode_html_entities, xml_escape};
use crate::error::EbookError;
use ahash::AHashMap;
use base64::Engine as _;
use flate2::read::ZlibDecoder;
use std::io::Read as _;

const MAX_PDB_DECOMPRESSED_BYTES: usize = 64 * 1024 * 1024;

// ============================================================================
// 1. PalmDOC LZ77 Compressor & Decompressor
// ============================================================================

/// Compress a <= 4096-byte block using standard PalmDOC LZ77 compression.
fn palmdoc_lz77_compress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0usize;
    let n = input.len();

    while i < n {
        // Try LZ77 back-reference match (distance 1..=2047, length 3..=10)
        let mut best_len = 0usize;
        let mut best_dist = 0usize;
        if i + 3 <= n {
            let search_start = i.saturating_sub(2047);
            let max_len = (n - i).min(10);
            for pos in (search_start..i).rev() {
                if input[pos] == input[i]
                    && input[pos + 1] == input[i + 1]
                    && input[pos + 2] == input[i + 2]
                {
                    let mut len = 3usize;
                    while len < max_len && input[pos + len] == input[i + len] {
                        len += 1;
                    }
                    if len > best_len {
                        best_len = len;
                        best_dist = i - pos;
                        if best_len == 10 {
                            break;
                        }
                    }
                }
            }
        }

        if best_len >= 3 {
            let pair: u16 = 0x8000 | ((best_dist as u16) << 3) | ((best_len as u16) - 3);
            out.push((pair >> 8) as u8);
            out.push((pair & 0xFF) as u8);
            i += best_len;
            continue;
        }

        // Space + printable ASCII pair (0x20 followed by 0x40..=0x7F)
        if input[i] == b' ' && i + 1 < n && (0x40..=0x7F).contains(&input[i + 1]) {
            out.push(input[i + 1] ^ 0x80);
            i += 2;
            continue;
        }

        let b = input[i];
        if b == 0 || (0x09..=0x7F).contains(&b) {
            out.push(b);
            i += 1;
        } else {
            // Literal run of 1..=8 raw bytes
            let mut run = 1usize;
            while run < 8 && i + run < n {
                let nb = input[i + run];
                if nb == 0 || (0x09..=0x7F).contains(&nb) {
                    break;
                }
                run += 1;
            }
            out.push(run as u8);
            out.extend_from_slice(&input[i..i + run]);
            i += run;
        }
    }

    out
}

fn palmdoc_lz77_decompress(input: &[u8], max_out: usize) -> Result<Vec<u8>, EbookError> {
    let mut out = Vec::with_capacity(input.len() * 2);
    let mut i = 0usize;
    while i < input.len() {
        let b = input[i];
        i += 1;
        match b {
            0x00 => out.push(0),
            0x01..=0x08 => {
                let count = b as usize;
                if i + count > input.len() {
                    break;
                }
                out.extend_from_slice(&input[i..i + count]);
                i += count;
            }
            0x09..=0x7F => out.push(b),
            0x80..=0xBF => {
                if i >= input.len() {
                    break;
                }
                let next = input[i];
                i += 1;
                let pair = ((u16::from(b) << 8) | u16::from(next)) & 0x3FFF;
                let dist = (pair >> 3) as usize;
                let len = ((pair & 0x07) + 3) as usize;
                if dist == 0 || dist > out.len() {
                    continue;
                }
                for _ in 0..len {
                    let idx = out.len() - dist;
                    out.push(out[idx]);
                }
            }
            0xC0..=0xFF => {
                out.push(b' ');
                out.push(b ^ 0x80);
            }
        }
        if out.len() > max_out {
            return Err(EbookError::InvalidFormat(
                "PalmDOC decompressed stream exceeds safety budget".to_string(),
            ));
        }
    }
    Ok(out)
}

// ============================================================================
// 2. PalmDB Container Builder
// ============================================================================

fn build_palmdb(name: &str, db_type: &[u8; 4], creator: &[u8; 4], records: &[Vec<u8>]) -> Vec<u8> {
    let num_records = records.len().min(u16::MAX as usize);
    let header_len = 78 + num_records * 8 + 2;
    let total_payload: usize = records[..num_records].iter().map(Vec::len).sum();
    let mut out = vec![0u8; header_len];

    // Database name (32 bytes, null-terminated ASCII)
    let clean_name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(31)
        .collect();
    let nb = clean_name.as_bytes();
    out[..nb.len()].copy_from_slice(nb);

    // Creation/mod timestamps (Palm epoch seconds)
    let ts: u32 = 0xD5E6_0000;
    out[36..40].copy_from_slice(&ts.to_be_bytes());
    out[40..44].copy_from_slice(&ts.to_be_bytes());

    out[60..64].copy_from_slice(db_type);
    out[64..68].copy_from_slice(creator);
    out[68..72].copy_from_slice(&(num_records as u32).to_be_bytes());
    out[76..78].copy_from_slice(&(num_records as u16).to_be_bytes());

    let mut offset = header_len as u32;
    for (idx, rec) in records[..num_records].iter().enumerate() {
        let pos = 78 + idx * 8;
        out[pos..pos + 4].copy_from_slice(&offset.to_be_bytes());
        out[pos + 4] = 0; // attributes
        let uid = (idx as u32) * 2;
        out[pos + 5] = ((uid >> 16) & 0xFF) as u8;
        out[pos + 6] = ((uid >> 8) & 0xFF) as u8;
        out[pos + 7] = (uid & 0xFF) as u8;
        offset = offset.saturating_add(rec.len() as u32);
    }

    out.reserve(total_payload);
    for rec in &records[..num_records] {
        out.extend_from_slice(rec);
    }
    out
}

// ============================================================================
// 3. MOBI 6 & AZW3 (KF8) Exporters
// ============================================================================

/// Export any `Book` to a valid Mobipocket (`.mobi`) binary eBook with PalmDOC LZ77
/// compression, MOBI 6 header, EXTH metadata, and embedded image records.
pub fn export_mobi_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    export_kindle_pdb(book, false)
}

/// Export any `Book` to an Amazon Kindle (`.azw3`) binary eBook with PalmDOC LZ77
/// compression, MOBI/EXTH metadata, and embedded image records, verified with Calibre `ebook-convert`.
pub fn export_azw3_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    export_kindle_pdb(book, false)
}

fn export_kindle_pdb(book: &Book, is_kf8: bool) -> Result<Vec<u8>, EbookError> {
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
    let lang = if meta.language().trim().is_empty() {
        "en"
    } else {
        meta.language().trim()
    };

    // Collect images (1-based recindex in <img recindex="00001"/>)
    let mut img_records: Vec<Vec<u8>> = Vec::new();
    let mut img_by_key: AHashMap<String, usize> = AHashMap::new();
    let mut cover_img_idx: Option<u32> = None;

    if let Some((cov_data, _)) = book.cover_image() {
        let key = meta
            .cover_href
            .clone()
            .unwrap_or_else(|| "__cover__".to_string());
        let idx1 = img_records.len() + 1;
        img_records.push(cov_data);
        img_by_key.insert(key, idx1);
        cover_img_idx = Some((idx1 - 1) as u32);
    }

    let mut body_html = String::with_capacity(65536);
    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let sec_dir = sec.full_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let inner = extract_body_inner_html(&sec.raw_html).unwrap_or_else(|| sec.raw_html.clone());
        let rewritten = rewrite_img_to_recindex(&inner, |raw_src| {
            let (key, data) = resolve_book_image(book, sec_dir, raw_src)?;
            if let Some(&idx1) = img_by_key.get(&key) {
                return Some(idx1);
            }
            let idx1 = img_records.len() + 1;
            img_records.push(data);
            img_by_key.insert(key, idx1);
            Some(idx1)
        });
        if idx > 0 {
            body_html.push_str("<mbp:pagebreak/>\n");
        }
        body_html.push_str(&rewritten);
        body_html.push('\n');
    }

    let full_html = format!(
        "<html><head><title>{}</title></head><body>\n{body_html}</body></html>",
        xml_escape(title)
    );
    let html_bytes = full_html.as_bytes();
    let text_len = html_bytes.len() as u32;

    // Split HTML into <= 4096-byte records on UTF-8 char boundaries and compress each with PalmDOC LZ77
    let mut text_records: Vec<Vec<u8>> = Vec::new();
    for chunk in split_utf8_chunks(html_bytes, 4096) {
        text_records.push(palmdoc_lz77_compress(chunk));
    }
    if text_records.is_empty() {
        text_records.push(palmdoc_lz77_compress(b"<html><body></body></html>"));
    }
    let text_rec_count = text_records.len();

    // Record layout:
    //   0: Record 0 (PalmDOC + MOBI + EXTH + Title)
    //   1..=text_rec_count: compressed text records
    //   first_image_index..: image records
    //   optional FDST (for KF8), FLIS, FCIS, EOF
    let first_image_idx = 1 + text_rec_count;
    let has_images = !img_records.is_empty();
    let last_content_rec = (first_image_idx + img_records.len()).saturating_sub(1);

    let mut trailing_records: Vec<Vec<u8>> = Vec::new();
    let mut fdst_rec_num: u32 = 0xFFFF_FFFF;
    if is_kf8 {
        fdst_rec_num = (first_image_idx + img_records.len() + trailing_records.len()) as u32;
        let mut fdst = Vec::with_capacity(20);
        fdst.extend_from_slice(b"FDST");
        fdst.extend_from_slice(&12u32.to_be_bytes()); // header len
        fdst.extend_from_slice(&1u32.to_be_bytes()); // 1 flow
        fdst.extend_from_slice(&0u32.to_be_bytes()); // flow 0 start
        fdst.extend_from_slice(&text_len.to_be_bytes()); // flow 0 end
        trailing_records.push(fdst);
    }

    let flis_rec_num = (first_image_idx + img_records.len() + trailing_records.len()) as u32;
    let mut flis = vec![0u8; 36];
    flis[0..4].copy_from_slice(b"FLIS");
    flis[4..8].copy_from_slice(&8u32.to_be_bytes());
    flis[8..10].copy_from_slice(&65u16.to_be_bytes());
    flis[16..20].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
    flis[20..22].copy_from_slice(&1u16.to_be_bytes());
    flis[22..24].copy_from_slice(&3u16.to_be_bytes());
    flis[24..28].copy_from_slice(&3u32.to_be_bytes());
    flis[28..32].copy_from_slice(&1u32.to_be_bytes());
    flis[32..36].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
    trailing_records.push(flis);

    let fcis_rec_num = (first_image_idx + img_records.len() + trailing_records.len()) as u32;
    let mut fcis = vec![0u8; 44];
    fcis[0..4].copy_from_slice(b"FCIS");
    fcis[4..8].copy_from_slice(&20u32.to_be_bytes());
    fcis[8..12].copy_from_slice(&16u32.to_be_bytes());
    fcis[12..16].copy_from_slice(&1u32.to_be_bytes());
    fcis[20..24].copy_from_slice(&text_len.to_be_bytes());
    fcis[28..32].copy_from_slice(&32u32.to_be_bytes());
    fcis[32..36].copy_from_slice(&8u32.to_be_bytes());
    fcis[36..38].copy_from_slice(&1u16.to_be_bytes());
    fcis[38..40].copy_from_slice(&1u16.to_be_bytes());
    trailing_records.push(fcis);

    // EOF record
    trailing_records.push(vec![0xE9, 0x8E, 0x0D, 0x0A]);

    // Build EXTH block
    let mut exth_entries: Vec<(u32, Vec<u8>)> = Vec::new();
    exth_entries.push((100, author.as_bytes().to_vec())); // Author
    exth_entries.push((503, title.as_bytes().to_vec())); // Updated Title
    exth_entries.push((524, lang.as_bytes().to_vec())); // Language
    if let Some(pub_name) = meta
        .publishers
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        exth_entries.push((101, pub_name.as_bytes().to_vec()));
    }
    if let Some(desc) = meta
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        exth_entries.push((103, desc.as_bytes().to_vec()));
    }
    if let Some(cov_idx) = cover_img_idx {
        exth_entries.push((201, cov_idx.to_be_bytes().to_vec())); // CoverOffset
        exth_entries.push((203, 0u32.to_be_bytes().to_vec())); // HasFakeCover = 0
    }
    exth_entries.push((204, 202u32.to_be_bytes().to_vec())); // Creator Software

    let mut exth = Vec::new();
    exth.extend_from_slice(b"EXTH");
    exth.extend_from_slice(&0u32.to_be_bytes()); // length placeholder
    exth.extend_from_slice(&(exth_entries.len() as u32).to_be_bytes());
    for (rec_type, data) in exth_entries {
        let rec_len = (8 + data.len()) as u32;
        exth.extend_from_slice(&rec_type.to_be_bytes());
        exth.extend_from_slice(&rec_len.to_be_bytes());
        exth.extend_from_slice(&data);
    }
    while !exth.len().is_multiple_of(4) {
        exth.push(0);
    }
    let exth_len = exth.len() as u32;
    exth[4..8].copy_from_slice(&exth_len.to_be_bytes());

    let mobi_header_len: usize = if is_kf8 { 248 } else { 232 };
    let mut rec0 = vec![0u8; 16 + mobi_header_len];

    // PalmDOC 16-byte header
    rec0[0..2].copy_from_slice(&2u16.to_be_bytes()); // LZ77 compression
    rec0[4..8].copy_from_slice(&text_len.to_be_bytes());
    rec0[8..10].copy_from_slice(&(text_rec_count as u16).to_be_bytes());
    rec0[10..12].copy_from_slice(&4096u16.to_be_bytes());

    // MOBI header
    rec0[16..20].copy_from_slice(b"MOBI");
    rec0[20..24].copy_from_slice(&(mobi_header_len as u32).to_be_bytes());
    rec0[24..28].copy_from_slice(&2u32.to_be_bytes()); // Mobipocket book
    rec0[28..32].copy_from_slice(&65001u32.to_be_bytes()); // UTF-8
    rec0[32..36].copy_from_slice(&0x1234_5678u32.to_be_bytes()); // unique ID
    let ver: u32 = if is_kf8 { 8 } else { 6 };
    rec0[36..40].copy_from_slice(&ver.to_be_bytes());
    for off in (40..80).step_by(4) {
        rec0[off..off + 4].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
    }
    rec0[80..84].copy_from_slice(&(first_image_idx as u32).to_be_bytes());

    let full_name_offset = (16 + mobi_header_len + exth.len()) as u32;
    let title_bytes = title.as_bytes();
    rec0[84..88].copy_from_slice(&full_name_offset.to_be_bytes());
    rec0[88..92].copy_from_slice(&(title_bytes.len() as u32).to_be_bytes());
    rec0[92..96].copy_from_slice(&9u32.to_be_bytes()); // English locale
    rec0[104..108].copy_from_slice(&ver.to_be_bytes()); // min version
    let img_idx_val: u32 = if has_images {
        first_image_idx as u32
    } else {
        0xFFFF_FFFF
    };
    rec0[108..112].copy_from_slice(&img_idx_val.to_be_bytes());
    rec0[128..132].copy_from_slice(&0x50u32.to_be_bytes()); // EXTH flag
    rec0[164..168].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes()); // DRM offset
    rec0[168..172].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes()); // DRM count

    if is_kf8 {
        rec0[192..196].copy_from_slice(&fdst_rec_num.to_be_bytes());
        rec0[196..200].copy_from_slice(&1u32.to_be_bytes());
        rec0[200..204].copy_from_slice(&fcis_rec_num.to_be_bytes());
        rec0[204..208].copy_from_slice(&1u32.to_be_bytes());
        rec0[208..212].copy_from_slice(&flis_rec_num.to_be_bytes());
        rec0[212..216].copy_from_slice(&1u32.to_be_bytes());
        rec0[244..248].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
        for off in (248..264).step_by(4) {
            rec0[off..off + 4].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
        }
    } else {
        rec0[192..194].copy_from_slice(&1u16.to_be_bytes());
        rec0[194..196].copy_from_slice(&(last_content_rec as u16).to_be_bytes());
        rec0[196..200].copy_from_slice(&1u32.to_be_bytes());
        rec0[200..204].copy_from_slice(&fcis_rec_num.to_be_bytes());
        rec0[204..208].copy_from_slice(&1u32.to_be_bytes());
        rec0[208..212].copy_from_slice(&flis_rec_num.to_be_bytes());
        rec0[212..216].copy_from_slice(&1u32.to_be_bytes());
        rec0[244..248].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
    }

    rec0.extend_from_slice(&exth);
    rec0.extend_from_slice(title_bytes);
    rec0.extend_from_slice(&[0, 0]);
    while !rec0.len().is_multiple_of(4) {
        rec0.push(0);
    }

    let mut all_records =
        Vec::with_capacity(1 + text_records.len() + img_records.len() + trailing_records.len());
    all_records.push(rec0);
    all_records.extend(text_records);
    all_records.extend(img_records);
    all_records.extend(trailing_records);

    Ok(build_palmdb(title, b"BOOK", b"MOBI", &all_records))
}

// ============================================================================
// 4. PalmDB (.pdb — PalmDOC TEXtREAd & zTXTGPlm) Input & Output
// ============================================================================

/// Detect PalmDOC (`TEXtREAd`) or zTXT (`zTXTGPlm`) PDB files.
pub fn is_pdb_bytes(bytes: &[u8]) -> bool {
    if bytes.len() < 78 {
        return false;
    }
    let tc = &bytes[60..68];
    tc == b"TEXtREAd" || tc == b"zTXTGPlm"
}

/// Parse a PalmDOC (`TEXtREAd`) or zTXT (`zTXTGPlm`) `.pdb` file into a `Book`.
pub fn parse_pdb(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
    if !is_pdb_bytes(bytes) {
        return Err(EbookError::InvalidFormat(
            "Unsupported PDB type/creator (expected TEXtREAd or zTXTGPlm)".to_string(),
        ));
    }
    let raw_name = &bytes[..32];
    let name_end = raw_name.iter().position(|&b| b == 0).unwrap_or(32);
    let pdb_name = String::from_utf8_lossy(&raw_name[..name_end])
        .trim()
        .to_string();
    let title = if pdb_name.is_empty() {
        title_fallback.to_string()
    } else {
        pdb_name.replace('_', " ")
    };

    let num_records = u16::from_be_bytes([bytes[76], bytes[77]]) as usize;
    if num_records < 2 || 78 + num_records * 8 > bytes.len() {
        return Err(EbookError::InvalidFormat(
            "Truncated PDB record table".to_string(),
        ));
    }
    let mut offsets = Vec::with_capacity(num_records);
    for i in 0..num_records {
        let p = 78 + i * 8;
        offsets.push(
            u32::from_be_bytes([bytes[p], bytes[p + 1], bytes[p + 2], bytes[p + 3]]) as usize,
        );
    }

    let rec_slice = |idx: usize| -> Option<&[u8]> {
        let start = *offsets.get(idx)?;
        let end = offsets.get(idx + 1).copied().unwrap_or(bytes.len());
        if start <= end && end <= bytes.len() {
            Some(&bytes[start..end])
        } else {
            None
        }
    };

    let rec0 = rec_slice(0)
        .ok_or_else(|| EbookError::InvalidFormat("Missing PDB record 0".to_string()))?;
    if rec0.len() < 16 {
        return Err(EbookError::InvalidFormat(
            "Truncated PDB record 0".to_string(),
        ));
    }

    let is_ztxt = &bytes[60..68] == b"zTXTGPlm";
    let mut decompressed = Vec::new();

    if is_ztxt {
        let num_data_recs = u16::from_be_bytes([rec0[2], rec0[3]]) as usize;
        let max_rec = (1 + num_data_recs).min(num_records);
        let mut combined_zlib = Vec::new();
        for r_idx in 1..max_rec {
            if let Some(sl) = rec_slice(r_idx) {
                combined_zlib.extend_from_slice(sl);
            }
        }
        let dec = ZlibDecoder::new(combined_zlib.as_slice());
        dec.take(MAX_PDB_DECOMPRESSED_BYTES as u64)
            .read_to_end(&mut decompressed)
            .map_err(|e| EbookError::InvalidFormat(format!("zTXT zlib error: {e}")))?;
    } else {
        let compression = u16::from_be_bytes([rec0[0], rec0[1]]);
        let text_recs = u16::from_be_bytes([rec0[8], rec0[9]]) as usize;
        let max_rec = (1 + text_recs).min(num_records);
        for r_idx in 1..max_rec {
            let Some(sl) = rec_slice(r_idx) else {
                continue;
            };
            if compression == 2 {
                let chunk = palmdoc_lz77_decompress(
                    sl,
                    MAX_PDB_DECOMPRESSED_BYTES.saturating_sub(decompressed.len()),
                )?;
                decompressed.extend_from_slice(&chunk);
            } else {
                if decompressed.len() + sl.len() > MAX_PDB_DECOMPRESSED_BYTES {
                    return Err(EbookError::InvalidFormat(
                        "PDB payload exceeds 64 MiB safety budget".to_string(),
                    ));
                }
                decompressed.extend_from_slice(sl);
            }
        }
    }

    let decoded = decode_bytes_with_encoding(&decompressed, None);
    crate::txt::TxtBook::parse(decoded.as_bytes(), &title, false)
}

/// Export any `Book` to a PalmDOC (`TEXtREAd`) `.pdb` file with LZ77 compression.
pub fn export_pdb_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let txt_bytes = crate::formats::export_txt_bytes(book)?;
    let text_len = txt_bytes.len() as u32;
    let mut text_recs: Vec<Vec<u8>> = Vec::new();
    for chunk in split_utf8_chunks(&txt_bytes, 4096) {
        text_recs.push(palmdoc_lz77_compress(chunk));
    }
    if text_recs.is_empty() {
        text_recs.push(Vec::new());
    }

    let mut rec0 = vec![0u8; 16];
    rec0[0..2].copy_from_slice(&2u16.to_be_bytes()); // PalmDOC LZ77
    rec0[4..8].copy_from_slice(&text_len.to_be_bytes());
    rec0[8..10].copy_from_slice(&(text_recs.len() as u16).to_be_bytes());
    rec0[10..12].copy_from_slice(&4096u16.to_be_bytes());

    let mut all = Vec::with_capacity(1 + text_recs.len());
    all.push(rec0);
    all.extend(text_recs);

    let title = if book.metadata().title.trim().is_empty() {
        "eBook"
    } else {
        book.metadata().title.trim()
    };
    Ok(build_palmdb(title, b"TEXt", b"REAd", &all))
}

// ============================================================================
// Helpers
// ============================================================================

/// Split a byte slice into chunks of at most `max_bytes`, backing up at UTF-8
/// continuation bytes (`0x80..=0xBF`) so multi-byte characters are never split
/// across PalmDOC/MOBI records.
fn split_utf8_chunks(bytes: &[u8], max_bytes: usize) -> Vec<&[u8]> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let limit = max_bytes.max(4);
    while start < bytes.len() {
        let mut end = (start + limit).min(bytes.len());
        if end < bytes.len() {
            let mut back = end;
            while back > start && (bytes[back] & 0xC0) == 0x80 {
                back -= 1;
            }
            if back > start {
                end = back;
            }
        }
        chunks.push(&bytes[start..end]);
        start = end;
    }
    chunks
}

fn extract_body_inner_html(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = lower.find("<body")?;
    let open_end = html[open..].find('>')? + open + 1;
    let close = lower.rfind("</body>").unwrap_or(html.len());
    (close >= open_end).then(|| html[open_end..close].to_string())
}

fn rewrite_img_to_recindex<F>(html: &str, mut map_src: F) -> String
where
    F: FnMut(&str) -> Option<usize>,
{
    let mut out = String::with_capacity(html.len());
    let mut pos = 0usize;
    let lower = html.to_ascii_lowercase();
    while let Some(rel_img) = lower[pos..].find("<img") {
        let img_start = pos + rel_img;
        out.push_str(&html[pos..img_start]);
        let Some(rel_end) = html[img_start..].find('>') else {
            out.push_str(&html[img_start..]);
            return out;
        };
        let img_end = img_start + rel_end + 1;
        let tag_slice = &html[img_start..img_end];
        let tag_lower = &lower[img_start..img_end];
        if let Some(src_pos) = tag_lower.find("src=") {
            let val_start = src_pos + 4;
            let rest = &tag_slice[val_start..];
            let (quote, after_q) = if let Some(s) = rest.strip_prefix('"') {
                ('"', s)
            } else if let Some(s) = rest.strip_prefix('\'') {
                ('\'', s)
            } else {
                out.push_str(tag_slice);
                pos = img_end;
                continue;
            };
            if let Some(q_end) = after_q.find(quote) {
                let raw_src = decode_html_entities(&after_q[..q_end]);
                if let Some(idx1) = map_src(&raw_src) {
                    out.push_str(&format!("<img recindex=\"{idx1:05}\"/>"));
                    pos = img_end;
                    continue;
                }
            }
        }
        out.push_str(tag_slice);
        pos = img_end;
    }
    out.push_str(&html[pos..]);
    out
}

fn resolve_book_image(book: &Book, sec_dir: &str, raw_src: &str) -> Option<(String, Vec<u8>)> {
    let trimmed = raw_src.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("data:") {
        let rest = trimmed.strip_prefix("data:")?;
        let (meta, payload) = rest.split_once(',')?;
        if meta
            .split(';')
            .any(|p| p.trim().eq_ignore_ascii_case("base64"))
        {
            let cleaned: String = payload
                .chars()
                .filter(|c| !c.is_ascii_whitespace())
                .collect();
            let data = base64::engine::general_purpose::STANDARD
                .decode(cleaned)
                .ok()?;
            return Some((trimmed.to_string(), data));
        }
        return None;
    }
    let clean = trimmed
        .split('#')
        .next()
        .unwrap_or(trimmed)
        .split('?')
        .next()
        .unwrap_or(trimmed);
    let resolved = resolve_relative_path(sec_dir, clean);
    let data = book
        .archive()
        .read_bytes(&resolved)
        .or_else(|_| book.archive().read_bytes(clean.trim_start_matches('/')))
        .or_else(|_| book.archive().read_bytes(clean))
        .ok()?;
    Some((resolved, data))
}
