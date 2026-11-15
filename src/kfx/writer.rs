use crate::archive::resolve_relative_path;
use crate::book::Book;
use crate::dom::{DomNode, EbookDomTree, decode_html_entities};
use crate::kfx::container::{KFX_HEADER_LEN, KFX_MAGIC_CONT, KfxIndexEntry};
use crate::kfx::ion::{IonValue, encode_stream};
use crate::kfx::symbols::*;
use crate::metadata::PageProgressionDirection;
use crate::nav::NavPoint;
use ahash::AHashMap;
use base64::Engine as _;
use sha1_smol::Sha1;

const YJ_MAX_ID: u32 = 851;
const SYM_NULL_EID: u32 = 348;
const MAX_CONTENT_FRAGMENT_SIZE: usize = 8192;
const ENTY_PREAMBLE: [u8; 21] =
    *b"ENTY\x01\x00\x15\x00\x00\x00\xe0\x01\x00\xea\xd6\x03\x9a\x20\x03\x9b\x20";

/// Universal Amazon KFX Container Exporter capable of serializing any `Book` into valid KFX container bytes.
pub struct UniversalKfxExporter;

#[derive(Clone, Debug)]
struct LinkSpan {
    start: usize,
    len: usize,
    href: String,
}

#[derive(Clone, Debug)]
struct InlineId {
    id: String,
    offset: usize,
}

#[derive(Clone, Debug)]
struct TextBlockData {
    text: String,
    links: Vec<LinkSpan>,
    inline_ids: Vec<InlineId>,
    ids: Vec<String>,
}

#[derive(Clone, Debug)]
struct CellData {
    text: String,
    links: Vec<LinkSpan>,
    inline_ids: Vec<InlineId>,
    ids: Vec<String>,
    colspan: i64,
    rowspan: i64,
}

#[derive(Clone, Debug)]
struct RowData {
    cells: Vec<CellData>,
    ids: Vec<String>,
}

#[derive(Clone, Debug)]
struct ImageBlockData {
    res_idx: usize,
    alt: String,
    href: Option<String>,
    ids: Vec<String>,
}

#[derive(Clone, Debug)]
enum ParsedBlock {
    Text(TextBlockData),
    Image(ImageBlockData),
    Table {
        rows: Vec<RowData>,
        ids: Vec<String>,
    },
}

#[derive(Clone, Debug)]
struct ParsedSection {
    full_path: String,
    href: String,
    blocks: Vec<ParsedBlock>,
}

#[derive(Clone, Debug)]
struct EmbeddedImage {
    data: Vec<u8>,
    mime: String,
    format_sym: Option<u32>,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, Debug)]
struct AnchorTarget {
    eid: i64,
    offset: i64,
    is_first_in_sec: bool,
    block_char_len: usize,
}

struct SymbolAlloc {
    symbols: Vec<String>,
    map: AHashMap<String, u32>,
}

impl SymbolAlloc {
    fn new() -> Self {
        Self {
            symbols: Vec::new(),
            map: AHashMap::new(),
        }
    }

    fn alloc(&mut self, name: String) -> u32 {
        if let Some(&id) = self.map.get(&name) {
            return id;
        }
        let id = YJ_MAX_ID + 1 + self.symbols.len() as u32;
        self.symbols.push(name.clone());
        self.map.insert(name, id);
        id
    }

    fn max_id(&self) -> u32 {
        YJ_MAX_ID + self.symbols.len() as u32
    }
}

struct InlineAcc {
    text: String,
    links: Vec<LinkSpan>,
    inline_ids: Vec<InlineId>,
    char_len: usize,
}

impl InlineAcc {
    fn new() -> Self {
        Self {
            text: String::new(),
            links: Vec::new(),
            inline_ids: Vec::new(),
            char_len: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.text.trim().is_empty() && self.inline_ids.is_empty()
    }

    fn push_text(&mut self, raw: &str, in_pre: bool) {
        let decoded = decode_html_entities(raw);
        if in_pre {
            self.char_len += decoded.chars().count();
            self.text.push_str(&decoded);
            return;
        }
        for ch in decoded.chars() {
            if ch.is_ascii_whitespace() {
                if !self.text.is_empty() && !self.text.ends_with(' ') {
                    self.text.push(' ');
                    self.char_len += 1;
                }
            } else {
                self.text.push(ch);
                self.char_len += 1;
            }
        }
    }

    fn push_newline(&mut self) {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
            self.char_len += 1;
        }
    }

    fn record_id(&mut self, id: &str) {
        if !id.is_empty() {
            self.inline_ids.push(InlineId {
                id: id.to_string(),
                offset: self.char_len,
            });
        }
    }

    fn take(&mut self, pending_ids: &mut Vec<String>) -> Option<TextBlockData> {
        let trimmed = self.text.trim_end().to_string();
        if trimmed.is_empty() {
            self.text.clear();
            self.links.clear();
            self.inline_ids.clear();
            self.char_len = 0;
            return None;
        }
        let max_chars = trimmed.chars().count();
        let mut links = Vec::new();
        for mut span in std::mem::take(&mut self.links) {
            if span.start < max_chars {
                if span.start + span.len > max_chars {
                    span.len = max_chars - span.start;
                }
                if span.len > 0 {
                    links.push(span);
                }
            }
        }
        let mut inline_ids = std::mem::take(&mut self.inline_ids);
        for iid in &mut inline_ids {
            if iid.offset >= max_chars {
                iid.offset = max_chars.saturating_sub(1);
            }
        }
        self.text.clear();
        self.char_len = 0;
        Some(TextBlockData {
            text: trimmed,
            links,
            inline_ids,
            ids: std::mem::take(pending_ids),
        })
    }
}

struct ImageCollector<'a> {
    book: &'a Book,
    images: Vec<EmbeddedImage>,
    by_key: AHashMap<String, usize>,
}

impl<'a> ImageCollector<'a> {
    fn new(book: &'a Book) -> Self {
        Self {
            book,
            images: Vec::new(),
            by_key: AHashMap::new(),
        }
    }

    fn collect(&mut self, sec_dir: &str, src: &str) -> Option<usize> {
        let raw_src = decode_html_entities(src.trim());
        if raw_src.is_empty() {
            return None;
        }
        if raw_src.starts_with("data:") {
            let key = format!("data:{raw_src}");
            if let Some(&idx) = self.by_key.get(&key) {
                return Some(idx);
            }
            let (mime_hint, data) = decode_data_uri(&raw_src)?;
            return self.insert_image(key, data, mime_hint.as_deref());
        }

        let resolved = resolve_relative_path(sec_dir, &raw_src);
        let key = format!("path:{resolved}");
        if let Some(&idx) = self.by_key.get(&key) {
            return Some(idx);
        }

        let clean_rel = raw_src
            .split('#')
            .next()
            .unwrap_or(&raw_src)
            .split('?')
            .next()
            .unwrap_or(&raw_src)
            .trim_start_matches('/');

        let data = self
            .book
            .archive()
            .read_bytes(&resolved)
            .or_else(|_| self.book.archive().read_bytes(clean_rel))
            .or_else(|_| self.book.archive().read_bytes(&raw_src))
            .ok()?;

        self.insert_image(key, data, Some(&resolved))
    }

    fn insert_image(&mut self, key: String, data: Vec<u8>, hint: Option<&str>) -> Option<usize> {
        if data.is_empty() {
            return None;
        }
        let (mime, format_sym, width, height) = inspect_image(&data, hint)?;
        let idx = self.images.len();
        self.images.push(EmbeddedImage {
            data,
            mime,
            format_sym,
            width,
            height,
        });
        self.by_key.insert(key, idx);
        Some(idx)
    }
}

impl UniversalKfxExporter {
    /// Export any `Book` (EPUB, MOBI, AZW3, PDF, FB2, TXT, MD) to a valid Amazon KFX binary container buffer.
    pub fn export(book: &Book) -> Result<Vec<u8>, String> {
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

        let (container_id, asin, book_id) = compute_publication_ids(book, title, author, lang);

        // Pass 1: Parse spine sections into structured blocks + embedded images
        let mut img_collector = ImageCollector::new(book);
        let mut parsed_sections: Vec<ParsedSection> = Vec::new();
        let mut raw_sec_indices: Vec<Option<usize>> = Vec::with_capacity(book.spine().len());

        for idx in 0..book.spine().len() {
            let Ok(sec) = book.get_section(idx) else {
                raw_sec_indices.push(None);
                continue;
            };
            let sec_dir = sec
                .full_path
                .rsplit_once('/')
                .map(|(dir, _)| dir)
                .unwrap_or("");
            let mut blocks = Vec::new();

            if !sec.raw_html.trim().is_empty() {
                let dom = EbookDomTree::parse(&sec.raw_html);
                extract_blocks_from_nodes(
                    &dom.root_nodes,
                    sec_dir,
                    &mut img_collector,
                    &mut blocks,
                    &mut Vec::new(),
                    None,
                    false,
                );
            }

            if blocks.is_empty() && !sec.plain_text.trim().is_empty() {
                for para in sec
                    .plain_text
                    .split('\n')
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                {
                    blocks.push(ParsedBlock::Text(TextBlockData {
                        text: para.to_string(),
                        links: Vec::new(),
                        inline_ids: Vec::new(),
                        ids: Vec::new(),
                    }));
                }
            }

            if blocks.is_empty() {
                raw_sec_indices.push(None);
            } else {
                let out_idx = parsed_sections.len();
                raw_sec_indices.push(Some(out_idx));
                parsed_sections.push(ParsedSection {
                    full_path: normalize_href_path(&sec.full_path),
                    href: normalize_href_path(&sec.href),
                    blocks,
                });
            }
        }

        if parsed_sections.is_empty() {
            parsed_sections.push(ParsedSection {
                full_path: "sec_0.xhtml".to_string(),
                href: "sec_0.xhtml".to_string(),
                blocks: vec![ParsedBlock::Text(TextBlockData {
                    text: title.to_string(),
                    links: Vec::new(),
                    inline_ids: Vec::new(),
                    ids: Vec::new(),
                })],
            });
        }

        let images = img_collector.images;

        // Assign EIDs and build anchor resolution maps
        let mut next_eid: i64 = 1000;
        let mut sec_page_eids: Vec<i64> = Vec::with_capacity(parsed_sections.len());
        let mut block_eids: Vec<Vec<AssignedBlockEids>> = Vec::with_capacity(parsed_sections.len());

        // Anchor lookup maps:
        // - per-section id map: sec_idx -> id -> AnchorTarget
        // - path -> parsed_section index
        let mut per_sec_ids: Vec<AHashMap<String, AnchorTarget>> =
            vec![AHashMap::new(); parsed_sections.len()];
        let mut sec_first_target: Vec<AnchorTarget> = Vec::with_capacity(parsed_sections.len());
        let mut path_to_psec: AHashMap<String, usize> = AHashMap::new();
        let mut basename_to_psec: AHashMap<String, Option<usize>> = AHashMap::new();

        for (p_idx, psec) in parsed_sections.iter().enumerate() {
            let page_eid = next_eid;
            next_eid += 1;
            sec_page_eids.push(page_eid);

            if !psec.full_path.is_empty() {
                path_to_psec.insert(psec.full_path.clone(), p_idx);
                if let Some(base) = psec.full_path.rsplit('/').next() {
                    basename_to_psec
                        .entry(base.to_string())
                        .and_modify(|e| *e = None)
                        .or_insert(Some(p_idx));
                }
            }
            if !psec.href.is_empty() {
                path_to_psec.insert(psec.href.clone(), p_idx);
                if let Some(base) = psec.href.rsplit('/').next() {
                    basename_to_psec
                        .entry(base.to_string())
                        .and_modify(|e| *e = None)
                        .or_insert(Some(p_idx));
                }
            }

            let mut psec_eids = Vec::with_capacity(psec.blocks.len());
            let mut first_target: Option<AnchorTarget> = None;

            for (b_idx, blk) in psec.blocks.iter().enumerate() {
                let is_first = b_idx == 0;
                match blk {
                    ParsedBlock::Text(tb) => {
                        let eid = next_eid;
                        next_eid += 1;
                        let char_len = tb.text.chars().count();
                        let tgt = AnchorTarget {
                            eid,
                            offset: 0,
                            is_first_in_sec: is_first,
                            block_char_len: char_len,
                        };
                        if first_target.is_none() {
                            first_target = Some(tgt);
                        }
                        for id in &tb.ids {
                            per_sec_ids[p_idx].entry(id.clone()).or_insert(tgt);
                        }
                        for iid in &tb.inline_ids {
                            let inline_tgt = AnchorTarget {
                                eid,
                                offset: iid.offset as i64,
                                is_first_in_sec: is_first && iid.offset == 0,
                                block_char_len: char_len,
                            };
                            per_sec_ids[p_idx]
                                .entry(iid.id.clone())
                                .or_insert(inline_tgt);
                        }
                        psec_eids.push(AssignedBlockEids::Single(eid));
                    }
                    ParsedBlock::Image(ib) => {
                        let _ = ib;
                        let eid = next_eid;
                        next_eid += 1;
                        let tgt = AnchorTarget {
                            eid,
                            offset: 0,
                            is_first_in_sec: is_first,
                            block_char_len: 1,
                        };
                        if first_target.is_none() {
                            first_target = Some(tgt);
                        }
                        for id in &ib.ids {
                            per_sec_ids[p_idx].entry(id.clone()).or_insert(tgt);
                        }
                        psec_eids.push(AssignedBlockEids::Single(eid));
                    }
                    ParsedBlock::Table { rows, ids } => {
                        let table_eid = next_eid;
                        next_eid += 1;
                        let table_tgt = AnchorTarget {
                            eid: table_eid,
                            offset: 0,
                            is_first_in_sec: is_first,
                            block_char_len: 1,
                        };
                        if first_target.is_none() {
                            first_target = Some(table_tgt);
                        }
                        for id in ids {
                            per_sec_ids[p_idx].entry(id.clone()).or_insert(table_tgt);
                        }
                        let mut row_eids = Vec::with_capacity(rows.len());
                        for row in rows {
                            let row_eid = next_eid;
                            next_eid += 1;
                            let row_tgt = AnchorTarget {
                                eid: row_eid,
                                offset: 0,
                                is_first_in_sec: false,
                                block_char_len: 1,
                            };
                            for id in &row.ids {
                                per_sec_ids[p_idx].entry(id.clone()).or_insert(row_tgt);
                            }
                            let mut cell_eids = Vec::with_capacity(row.cells.len());
                            for cell in &row.cells {
                                let cell_eid = next_eid;
                                next_eid += 1;
                                let cell_len = cell.text.chars().count();
                                let cell_tgt = AnchorTarget {
                                    eid: cell_eid,
                                    offset: 0,
                                    is_first_in_sec: false,
                                    block_char_len: cell_len,
                                };
                                for id in &cell.ids {
                                    per_sec_ids[p_idx].entry(id.clone()).or_insert(cell_tgt);
                                }
                                for iid in &cell.inline_ids {
                                    let inline_tgt = AnchorTarget {
                                        eid: cell_eid,
                                        offset: iid.offset as i64,
                                        is_first_in_sec: false,
                                        block_char_len: cell_len,
                                    };
                                    per_sec_ids[p_idx]
                                        .entry(iid.id.clone())
                                        .or_insert(inline_tgt);
                                }
                                cell_eids.push(cell_eid);
                            }
                            row_eids.push((row_eid, cell_eids));
                        }
                        psec_eids.push(AssignedBlockEids::Table {
                            table_eid,
                            rows: row_eids,
                        });
                    }
                }
            }

            sec_first_target.push(first_target.unwrap_or(AnchorTarget {
                eid: page_eid,
                offset: 0,
                is_first_in_sec: true,
                block_char_len: 0,
            }));
            block_eids.push(psec_eids);
        }

        // Map original spine indices that were skipped to nearest non-empty parsed section
        let fallback_psec = |orig_idx: usize| -> usize {
            if let Some(Some(p)) = raw_sec_indices.get(orig_idx) {
                return *p;
            }
            for &opt in raw_sec_indices.iter().skip(orig_idx) {
                if let Some(p) = opt {
                    return p;
                }
            }
            0
        };
        let _ = fallback_psec;

        // Allocate symbols strictly for referenced entities
        let mut sym_alloc = SymbolAlloc::new();
        let style_sym = sym_alloc.alloc("s0".to_string());

        let mut sec_syms: Vec<(u32, u32)> = Vec::with_capacity(parsed_sections.len());
        for i in 0..parsed_sections.len() {
            let sec_s = sym_alloc.alloc(format!("c{i}"));
            let story_s = sym_alloc.alloc(format!("l{i}"));
            sec_syms.push((sec_s, story_s));
        }

        let mut img_syms: Vec<(u32, u32, String)> = Vec::with_capacity(images.len());
        for i in 0..images.len() {
            let e_sym = sym_alloc.alloc(format!("e{i}"));
            let rsrc_name = format!("resource/rsrc{i}");
            let rsrc_sym = sym_alloc.alloc(rsrc_name.clone());
            img_syms.push((e_sym, rsrc_sym, rsrc_name));
        }

        // Link & Anchor resolver
        let mut anchor_entities: Vec<(u32, IonValue)> = Vec::new();
        let mut ext_anchor_cache: AHashMap<String, u32> = AHashMap::new();
        let mut int_anchor_cache: AHashMap<(i64, i64), u32> = AHashMap::new();
        let mut next_anchor_idx: usize = 0;

        let mut alloc_anchor_sym = |sym_alloc: &mut SymbolAlloc| -> u32 {
            let name = if next_anchor_idx < 1_000_000 {
                format!("a{next_anchor_idx}")
            } else {
                format!("a{:X}", next_anchor_idx)
            };
            next_anchor_idx += 1;
            sym_alloc.alloc(name)
        };

        let resolve_href_target = |from_psec: usize, href: &str| -> Option<ResolvedHref> {
            let trimmed = href.trim();
            if trimmed.is_empty() {
                return None;
            }
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("http://")
                || lower.starts_with("https://")
                || lower.starts_with("mailto:")
                || lower.starts_with("ftp://")
                || lower.starts_with("tel:")
            {
                return Some(ResolvedHref::External(trimmed.to_string()));
            }
            if lower.starts_with("javascript:") || lower.starts_with("data:") {
                return None;
            }

            let (path_part, frag_part) = match trimmed.split_once('#') {
                Some((p, f)) => (p, Some(f)),
                None => (trimmed, None),
            };

            let target_psec = if path_part.is_empty() {
                Some(from_psec)
            } else {
                let from_dir = parsed_sections[from_psec]
                    .full_path
                    .rsplit_once('/')
                    .map(|(d, _)| d)
                    .unwrap_or("");
                let resolved = normalize_href_path(&resolve_relative_path(from_dir, path_part));
                let clean_direct = normalize_href_path(path_part);
                path_to_psec
                    .get(&resolved)
                    .copied()
                    .or_else(|| path_to_psec.get(&clean_direct).copied())
                    .or_else(|| {
                        let base = resolved.rsplit('/').next().unwrap_or(&resolved);
                        basename_to_psec.get(base).and_then(|opt| *opt)
                    })
            }?;

            if let Some(frag) = frag_part.filter(|f| !f.is_empty())
                && let Some(&tgt) = per_sec_ids[target_psec].get(frag)
            {
                // KFX Input strips #id if it lands at offset 0 of the very first visible
                // element in a section. Nudging an explicit #fragment target to offset 1
                // (when the block has >= 2 chars) keeps the fragment anchor intact.
                let offset = if tgt.is_first_in_sec && tgt.offset == 0 && tgt.block_char_len >= 2 {
                    1
                } else {
                    tgt.offset
                };
                return Some(ResolvedHref::Internal(tgt.eid, offset));
            }

            let tgt = sec_first_target[target_psec];
            Some(ResolvedHref::Internal(tgt.eid, tgt.offset))
        };

        let mut get_or_create_anchor =
            |from_psec: usize, href: &str, sym_alloc: &mut SymbolAlloc| -> Option<u32> {
                match resolve_href_target(from_psec, href)? {
                    ResolvedHref::External(uri) => {
                        if let Some(&sym) = ext_anchor_cache.get(&uri) {
                            return Some(sym);
                        }
                        let a_sym = alloc_anchor_sym(sym_alloc);
                        anchor_entities.push((
                            a_sym,
                            IonValue::Struct(vec![
                                (180, IonValue::Symbol(u64::from(a_sym))),
                                (186, IonValue::String(uri.clone())),
                            ]),
                        ));
                        ext_anchor_cache.insert(uri, a_sym);
                        Some(a_sym)
                    }
                    ResolvedHref::Internal(eid, offset) => {
                        if let Some(&sym) = int_anchor_cache.get(&(eid, offset)) {
                            return Some(sym);
                        }
                        let a_sym = alloc_anchor_sym(sym_alloc);
                        anchor_entities.push((
                            a_sym,
                            IonValue::Struct(vec![
                                (180, IonValue::Symbol(u64::from(a_sym))),
                                (
                                    183,
                                    IonValue::Struct(vec![
                                        (155, IonValue::Int(eid)),
                                        (143, IonValue::Int(offset)),
                                    ]),
                                ),
                            ]),
                        ));
                        int_anchor_cache.insert((eid, offset), a_sym);
                        Some(a_sym)
                    }
                }
            };

        // Build TOC nav_units ($389 book_navigation)
        let resolve_toc_target = |nav: &NavPoint| -> (i64, i64) {
            let candidates = [nav.full_path.as_str(), nav.href.as_str()];
            for cand in candidates {
                let trimmed = cand.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let (path_part, frag_part) = match trimmed.split_once('#') {
                    Some((p, f)) => (p, Some(f)),
                    None => (trimmed, None),
                };
                let norm = normalize_href_path(path_part);
                let target_psec = path_to_psec.get(&norm).copied().or_else(|| {
                    let base = norm.rsplit('/').next().unwrap_or(&norm);
                    basename_to_psec.get(base).and_then(|opt| *opt)
                });
                if let Some(p_idx) = target_psec {
                    if let Some(frag) = frag_part.filter(|f| !f.is_empty())
                        && let Some(&tgt) = per_sec_ids[p_idx].get(frag)
                    {
                        return (tgt.eid, tgt.offset);
                    }
                    let tgt = sec_first_target[p_idx];
                    return (tgt.eid, tgt.offset);
                }
            }
            let first = sec_first_target[0];
            (first.eid, first.offset)
        };

        let toc_units = build_toc_nav_units(book.toc(), &resolve_toc_target);
        let nav_containers = if toc_units.is_empty() {
            Vec::new()
        } else {
            let n0_sym = sym_alloc.alloc("n0".to_string());
            vec![IonValue::Annotated(
                vec![391],
                Box::new(IonValue::Struct(vec![
                    (235, IonValue::Symbol(212)),
                    (239, IonValue::Symbol(u64::from(n0_sym))),
                    (247, IonValue::List(toc_units)),
                ])),
            )]
        };

        // Build sections, storylines, chunked $145 content fragments, and position maps
        let mut content_chunks: Vec<(u32, Vec<String>)> = Vec::new();
        let mut curr_content_sym: Option<u32> = None;
        let mut curr_content_strings: Vec<String> = Vec::new();
        let mut curr_content_bytes: usize = 0;
        let mut next_content_idx: usize = 0;

        let mut push_content_string = |s: String, sym_alloc: &mut SymbolAlloc| -> (u32, i64) {
            if curr_content_sym.is_none()
                || (!curr_content_strings.is_empty()
                    && curr_content_bytes >= MAX_CONTENT_FRAGMENT_SIZE)
            {
                if let Some(old_sym) = curr_content_sym.take() {
                    content_chunks.push((old_sym, std::mem::take(&mut curr_content_strings)));
                    curr_content_bytes = 0;
                }
                let c_sym = sym_alloc.alloc(format!("content_{next_content_idx}"));
                next_content_idx += 1;
                curr_content_sym = Some(c_sym);
            }
            let c_sym = curr_content_sym.unwrap();
            let idx = curr_content_strings.len() as i64;
            curr_content_bytes += s.len();
            curr_content_strings.push(s);
            (c_sym, idx)
        };

        let mut sec_pos_map = Vec::new();
        let mut pos_id_map = Vec::new();
        let mut loc_items = Vec::new();
        let mut running_pid: i64 = 0;

        let mut section_entities: Vec<(u32, IonValue)> = Vec::with_capacity(parsed_sections.len());
        let mut storyline_entities: Vec<(u32, IonValue)> =
            Vec::with_capacity(parsed_sections.len());

        for (p_idx, psec) in parsed_sections.iter().enumerate() {
            let (sec_s, story_s) = sec_syms[p_idx];
            let sec_page_eid = sec_page_eids[p_idx];

            let mut contains_ids = vec![IonValue::Int(sec_page_eid)];
            pos_id_map.push(IonValue::Struct(vec![
                (184, IonValue::Int(running_pid)),
                (185, IonValue::Int(sec_page_eid)),
            ]));
            running_pid += 1;

            let mut story_blocks = Vec::with_capacity(psec.blocks.len());

            for (b_idx, blk) in psec.blocks.iter().enumerate() {
                match (blk, &block_eids[p_idx][b_idx]) {
                    (ParsedBlock::Text(tb), AssignedBlockEids::Single(blk_eid)) => {
                        let blk_eid = *blk_eid;
                        contains_ids.push(IonValue::Int(blk_eid));
                        pos_id_map.push(IonValue::Struct(vec![
                            (184, IonValue::Int(running_pid)),
                            (185, IonValue::Int(blk_eid)),
                        ]));
                        loc_items.push(IonValue::Struct(vec![
                            (155, IonValue::Int(blk_eid)),
                            (143, IonValue::Int(0)),
                        ]));
                        running_pid += tb.text.chars().count().max(1) as i64;

                        let (c_sym, c_idx) = push_content_string(tb.text.clone(), &mut sym_alloc);
                        let style_events = build_style_events(
                            p_idx,
                            &tb.links,
                            &mut sym_alloc,
                            &mut get_or_create_anchor,
                        );
                        let mut fields = vec![
                            (155, IonValue::Int(blk_eid)),
                            (157, IonValue::Symbol(u64::from(style_sym))),
                            (159, IonValue::Symbol(269)),
                            (
                                145,
                                IonValue::Struct(vec![
                                    (4, IonValue::Symbol(u64::from(c_sym))),
                                    (403, IonValue::Int(c_idx)),
                                ]),
                            ),
                        ];
                        if !style_events.is_empty() {
                            fields.push((142, IonValue::List(style_events)));
                        }
                        story_blocks.push(IonValue::Struct(fields));
                    }
                    (ParsedBlock::Image(ib), AssignedBlockEids::Single(blk_eid)) => {
                        let blk_eid = *blk_eid;
                        contains_ids.push(IonValue::Int(blk_eid));
                        pos_id_map.push(IonValue::Struct(vec![
                            (184, IonValue::Int(running_pid)),
                            (185, IonValue::Int(blk_eid)),
                        ]));
                        loc_items.push(IonValue::Struct(vec![
                            (155, IonValue::Int(blk_eid)),
                            (143, IonValue::Int(0)),
                        ]));
                        running_pid += 1;

                        let (e_sym, _, _) = img_syms[ib.res_idx];
                        let mut fields = vec![
                            (155, IonValue::Int(blk_eid)),
                            (159, IonValue::Symbol(271)),
                            (175, IonValue::Symbol(u64::from(e_sym))),
                        ];
                        if !ib.alt.is_empty() {
                            fields.push((584, IonValue::String(ib.alt.clone())));
                        }
                        if let Some(href) = &ib.href
                            && let Some(a_sym) = get_or_create_anchor(p_idx, href, &mut sym_alloc)
                        {
                            fields.push((179, IonValue::Symbol(u64::from(a_sym))));
                        }
                        story_blocks.push(IonValue::Struct(fields));
                    }
                    (
                        ParsedBlock::Table { rows, .. },
                        AssignedBlockEids::Table {
                            table_eid,
                            rows: row_eids,
                        },
                    ) => {
                        let table_eid = *table_eid;
                        contains_ids.push(IonValue::Int(table_eid));
                        pos_id_map.push(IonValue::Struct(vec![
                            (184, IonValue::Int(running_pid)),
                            (185, IonValue::Int(table_eid)),
                        ]));
                        loc_items.push(IonValue::Struct(vec![
                            (155, IonValue::Int(table_eid)),
                            (143, IonValue::Int(0)),
                        ]));
                        running_pid += 1;

                        let mut row_vals = Vec::with_capacity(rows.len());
                        for (r_idx, row) in rows.iter().enumerate() {
                            let (row_eid, cell_eids) = &row_eids[r_idx];
                            let row_eid = *row_eid;
                            contains_ids.push(IonValue::Int(row_eid));
                            pos_id_map.push(IonValue::Struct(vec![
                                (184, IonValue::Int(running_pid)),
                                (185, IonValue::Int(row_eid)),
                            ]));
                            loc_items.push(IonValue::Struct(vec![
                                (155, IonValue::Int(row_eid)),
                                (143, IonValue::Int(0)),
                            ]));
                            running_pid += 1;

                            let mut cell_vals = Vec::with_capacity(row.cells.len());
                            for (c_idx, cell) in row.cells.iter().enumerate() {
                                let cell_eid = cell_eids[c_idx];
                                contains_ids.push(IonValue::Int(cell_eid));
                                pos_id_map.push(IonValue::Struct(vec![
                                    (184, IonValue::Int(running_pid)),
                                    (185, IonValue::Int(cell_eid)),
                                ]));
                                loc_items.push(IonValue::Struct(vec![
                                    (155, IonValue::Int(cell_eid)),
                                    (143, IonValue::Int(0)),
                                ]));
                                running_pid += cell.text.chars().count().max(1) as i64;

                                let mut c_fields = vec![
                                    (155, IonValue::Int(cell_eid)),
                                    (157, IonValue::Symbol(u64::from(style_sym))),
                                    (159, IonValue::Symbol(269)),
                                ];
                                if cell.colspan > 1 {
                                    c_fields.push((148, IonValue::Int(cell.colspan)));
                                }
                                if cell.rowspan > 1 {
                                    c_fields.push((149, IonValue::Int(cell.rowspan)));
                                }
                                if !cell.text.is_empty() {
                                    let (c_sym, s_idx) =
                                        push_content_string(cell.text.clone(), &mut sym_alloc);
                                    c_fields.push((
                                        145,
                                        IonValue::Struct(vec![
                                            (4, IonValue::Symbol(u64::from(c_sym))),
                                            (403, IonValue::Int(s_idx)),
                                        ]),
                                    ));
                                    let style_events = build_style_events(
                                        p_idx,
                                        &cell.links,
                                        &mut sym_alloc,
                                        &mut get_or_create_anchor,
                                    );
                                    if !style_events.is_empty() {
                                        c_fields.push((142, IonValue::List(style_events)));
                                    }
                                }
                                cell_vals.push(IonValue::Struct(c_fields));
                            }
                            row_vals.push(IonValue::Struct(vec![
                                (155, IonValue::Int(row_eid)),
                                (159, IonValue::Symbol(279)),
                                (146, IonValue::List(cell_vals)),
                            ]));
                        }
                        story_blocks.push(IonValue::Struct(vec![
                            (155, IonValue::Int(table_eid)),
                            (159, IonValue::Symbol(278)),
                            (146, IonValue::List(row_vals)),
                        ]));
                    }
                    _ => unreachable!("mismatched block and EID assignment"),
                }
            }

            sec_pos_map.push(IonValue::Struct(vec![
                (181, IonValue::List(contains_ids)),
                (174, IonValue::Symbol(u64::from(sec_s))),
            ]));

            section_entities.push((
                sec_s,
                IonValue::Struct(vec![
                    (174, IonValue::Symbol(u64::from(sec_s))),
                    (
                        141,
                        IonValue::List(vec![IonValue::Struct(vec![
                            (155, IonValue::Int(sec_page_eid)),
                            (176, IonValue::Symbol(u64::from(story_s))),
                            (159, IonValue::Symbol(269)),
                        ])]),
                    ),
                ]),
            ));

            storyline_entities.push((
                story_s,
                IonValue::Struct(vec![
                    (176, IonValue::Symbol(u64::from(story_s))),
                    (146, IonValue::List(story_blocks)),
                ]),
            ));
        }

        if let Some(last_sym) = curr_content_sym.take() {
            content_chunks.push((last_sym, curr_content_strings));
        }

        pos_id_map.push(IonValue::Struct(vec![
            (184, IonValue::Int(running_pid)),
            (185, IonValue::Int(0)),
        ]));

        let max_sym_id = sym_alloc.max_id();

        let mut entries: Vec<KfxIndexEntry> = Vec::new();
        let mut payload: Vec<u8> = Vec::new();

        // 1. $585 content_features
        let make_feature = |ns: &str, key: &str, maj: i64| {
            IonValue::Struct(vec![
                (586, IonValue::String(ns.to_string())),
                (492, IonValue::String(key.to_string())),
                (
                    589,
                    IonValue::Struct(vec![(
                        5,
                        IonValue::Struct(vec![(587, IonValue::Int(maj)), (588, IonValue::Int(0))]),
                    )]),
                ),
            ])
        };
        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            SYM_CONTENT_FEATURES,
            IonValue::Struct(vec![(
                590,
                IonValue::List(vec![
                    make_feature("com.amazon.yjconversion", "reflow-style", 1),
                    make_feature("SDK.Marker", "CanonicalFormat", 1),
                ]),
            )]),
        );

        // 2. $490 book_metadata
        let kv_str = |k: &str, v: &str| {
            IonValue::Struct(vec![
                (492, IonValue::String(k.to_string())),
                (307, IonValue::String(v.to_string())),
            ])
        };
        let kv_bool = |k: &str, v: bool| {
            IonValue::Struct(vec![
                (492, IonValue::String(k.to_string())),
                (307, IonValue::Bool(v)),
            ])
        };
        let mut title_meta = vec![
            kv_str("ASIN", &asin),
            kv_str("asset_id", &container_id),
            kv_str("author", author),
            kv_str("book_id", &book_id),
            kv_str("cde_content_type", "PDOC"),
            kv_str("content_id", &asin),
            kv_bool("is_sample", false),
            kv_str("language", lang),
            kv_bool("override_kindle_font", false),
            kv_str("title", title),
        ];
        if let Some(pub_name) = meta
            .publishers
            .first()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            title_meta.push(kv_str("publisher", pub_name));
        }
        if let Some(desc) = meta
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            title_meta.push(kv_str("description", desc));
        }

        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            SYM_BOOK_METADATA,
            IonValue::Struct(vec![(
                491,
                IonValue::List(vec![
                    IonValue::Struct(vec![
                        (495, IonValue::String("kindle_audit_metadata".to_string())),
                        (
                            258,
                            IonValue::List(vec![
                                kv_str("file_creator", "KPR"),
                                kv_str("creator_version", "3.80.0"),
                            ]),
                        ),
                    ]),
                    IonValue::Struct(vec![
                        (495, IonValue::String("kindle_title_metadata".to_string())),
                        (258, IonValue::List(title_meta)),
                    ]),
                    IonValue::Struct(vec![
                        (495, IonValue::String("kindle_ebook_metadata".to_string())),
                        (
                            258,
                            IonValue::List(vec![
                                kv_str("selection", "enabled"),
                                kv_str("nested_span", "enabled"),
                            ]),
                        ),
                    ]),
                    IonValue::Struct(vec![
                        (
                            495,
                            IonValue::String("kindle_capability_metadata".to_string()),
                        ),
                        (258, IonValue::List(Vec::new())),
                    ]),
                ]),
            )]),
        );

        let sec_sym_list: Vec<IonValue> = sec_syms
            .iter()
            .map(|(sec_s, _)| IonValue::Symbol(u64::from(*sec_s)))
            .collect();
        let reading_orders = IonValue::List(vec![IonValue::Struct(vec![
            (178, IonValue::Symbol(351)),
            (170, IonValue::List(sec_sym_list)),
        ])]);

        // 3. $258 metadata
        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            SYM_METADATA_RECORD,
            IonValue::Struct(vec![(169, reading_orders.clone())]),
        );

        // 4. $538 document_data
        let direction_sym = if meta.direction == PageProgressionDirection::Rtl {
            375u64
        } else {
            376u64
        };
        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            SYM_DOCUMENT_DATA,
            IonValue::Struct(vec![
                (560, IonValue::Symbol(557)),
                (192, IonValue::Symbol(direction_sym)),
                (112, IonValue::Symbol(383)),
                (
                    16,
                    IonValue::Struct(vec![
                        (307, IonValue::Decimal(vec![0x80, 0x01])),
                        (306, IonValue::Symbol(308)),
                    ]),
                ),
                (436, IonValue::Symbol(441)),
                (8, IonValue::Int(i64::from(max_sym_id))),
                (
                    42,
                    IonValue::Struct(vec![
                        (307, IonValue::Decimal(vec![0xC1, 0x0C])),
                        (306, IonValue::Symbol(308)),
                    ]),
                ),
                (477, IonValue::Symbol(56)),
                (169, reading_orders),
            ]),
        );

        // 5. $389 book_navigation
        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            SYM_NAVIGATION_TOC,
            IonValue::List(vec![IonValue::Struct(vec![
                (178, IonValue::Symbol(351)),
                (392, IonValue::List(nav_containers)),
            ])]),
        );

        // 6. Sections ($260), Storylines ($259), Content ($145)
        for (sec_s, val) in section_entities {
            push_ion_entity(&mut entries, &mut payload, sec_s, SYM_SECTION_BLOCK, val);
        }
        for (story_s, val) in storyline_entities {
            push_ion_entity(
                &mut entries,
                &mut payload,
                story_s,
                SYM_STORYLINE_FRAGMENT,
                val,
            );
        }
        for (c_sym, strings) in &content_chunks {
            push_ion_entity(
                &mut entries,
                &mut payload,
                *c_sym,
                SYM_CONTENT_BODY,
                IonValue::Struct(vec![
                    (4, IonValue::Symbol(u64::from(*c_sym))),
                    (
                        146,
                        IonValue::List(strings.iter().cloned().map(IonValue::String).collect()),
                    ),
                ]),
            );
        }

        // 7. Embedded images ($164 external_resource & $417 bcRawMedia)
        for (i, img) in images.iter().enumerate() {
            let (e_sym, rsrc_sym, rsrc_name) = &img_syms[i];
            let mut ext_fields = Vec::new();
            if let Some(fmt_sym) = img.format_sym {
                ext_fields.push((161, IonValue::Symbol(u64::from(fmt_sym))));
            }
            ext_fields.push((162, IonValue::String(img.mime.clone())));
            ext_fields.push((165, IonValue::String(rsrc_name.clone())));
            ext_fields.push((175, IonValue::Symbol(u64::from(*e_sym))));
            if img.width > 0 && img.height > 0 {
                ext_fields.push((422, IonValue::Int(i64::from(img.width))));
                ext_fields.push((423, IonValue::Int(i64::from(img.height))));
            }
            push_ion_entity(
                &mut entries,
                &mut payload,
                *e_sym,
                164,
                IonValue::Struct(ext_fields),
            );
            push_raw_entity(&mut entries, &mut payload, *rsrc_sym, 417, &img.data);
        }

        // 8. Anchors ($266)
        for (a_sym, val) in &anchor_entities {
            push_ion_entity(&mut entries, &mut payload, *a_sym, 266, val.clone());
        }

        // 9. $157 style
        push_ion_entity(
            &mut entries,
            &mut payload,
            style_sym,
            157,
            IonValue::Struct(vec![
                (
                    42,
                    IonValue::Struct(vec![
                        (307, IonValue::Decimal(vec![0x80, 0x01])),
                        (306, IonValue::Symbol(310)),
                    ]),
                ),
                (173, IonValue::Symbol(u64::from(style_sym))),
            ]),
        );

        // 10. $264 section_position_id_map, $265 position_id_map, $550 location_map, $395 resource_path
        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            264,
            IonValue::List(sec_pos_map),
        );
        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            265,
            IonValue::List(pos_id_map),
        );
        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            550,
            IonValue::List(vec![IonValue::Struct(vec![(
                182,
                IonValue::List(loc_items),
            )])]),
        );
        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            395,
            IonValue::Struct(vec![(247, IonValue::List(Vec::new()))]),
        );

        // 11. $419 container_entity_map
        let mut all_custom_eids = Vec::new();
        for (sec_s, story_s) in &sec_syms {
            all_custom_eids.push(IonValue::Symbol(u64::from(*sec_s)));
            all_custom_eids.push(IonValue::Symbol(u64::from(*story_s)));
        }
        for (c_sym, _) in &content_chunks {
            all_custom_eids.push(IonValue::Symbol(u64::from(*c_sym)));
        }
        for (e_sym, rsrc_sym, _) in &img_syms {
            all_custom_eids.push(IonValue::Symbol(u64::from(*e_sym)));
            all_custom_eids.push(IonValue::Symbol(u64::from(*rsrc_sym)));
        }
        for (a_sym, _) in &anchor_entities {
            all_custom_eids.push(IonValue::Symbol(u64::from(*a_sym)));
        }
        all_custom_eids.push(IonValue::Symbol(u64::from(style_sym)));

        push_ion_entity(
            &mut entries,
            &mut payload,
            SYM_NULL_EID,
            419,
            IonValue::Struct(vec![(
                252,
                IonValue::List(vec![IonValue::Struct(vec![
                    (155, IonValue::String(container_id.clone())),
                    (181, IonValue::List(all_custom_eids)),
                ])]),
            )]),
        );

        // Build header components: index_table, doc_symbol, format_capabilities, container_info, kfxgen_info
        let index_table_offset = KFX_HEADER_LEN;
        let index_table_length = entries.len() * 24;

        let doc_symbol_bytes = encode_stream(&[IonValue::Annotated(
            vec![3],
            Box::new(IonValue::Struct(vec![
                (8, IonValue::Int(i64::from(max_sym_id))),
                (
                    6,
                    IonValue::List(vec![IonValue::Struct(vec![
                        (4, IonValue::String("YJ_symbols".to_string())),
                        (5, IonValue::Int(10)),
                        (8, IonValue::Int(i64::from(YJ_MAX_ID))),
                    ])]),
                ),
                (
                    7,
                    IonValue::List(
                        sym_alloc
                            .symbols
                            .into_iter()
                            .map(IonValue::String)
                            .collect(),
                    ),
                ),
            ])),
        )]);

        let fcap_bytes = encode_stream(&[IonValue::Annotated(
            vec![593],
            Box::new(IonValue::List(vec![IonValue::Struct(vec![
                (492, IonValue::String("kfxgen.textBlock".to_string())),
                (5, IonValue::Int(1)),
            ])])),
        )]);

        let doc_symbol_offset = index_table_offset + index_table_length;
        let doc_symbol_length = doc_symbol_bytes.len();
        let fcap_offset = doc_symbol_offset + doc_symbol_length;
        let fcap_length = fcap_bytes.len();
        let info_offset = fcap_offset + fcap_length;

        let info_bytes = encode_stream(&[IonValue::Struct(vec![
            (409, IonValue::String(container_id.clone())),
            (410, IonValue::Int(0)),
            (411, IonValue::Int(0)),
            (
                SYM_INDEX_TABLE_OFFSET as u64,
                IonValue::Int(index_table_offset as i64),
            ),
            (
                SYM_INDEX_TABLE_LENGTH as u64,
                IonValue::Int(index_table_length as i64),
            ),
            (
                SYM_DOC_SYMBOL_OFFSET as u64,
                IonValue::Int(doc_symbol_offset as i64),
            ),
            (
                SYM_DOC_SYMBOL_LENGTH as u64,
                IonValue::Int(doc_symbol_length as i64),
            ),
            (412, IonValue::Int(4096)),
            (594, IonValue::Int(fcap_offset as i64)),
            (595, IonValue::Int(fcap_length as i64)),
        ])]);

        let mut sha1 = Sha1::new();
        sha1.update(&payload);
        let payload_sha1_hex = sha1.digest().to_string();
        let kfxgen_trailer = format!(
            "[{{key:\"kfxgen_package_version\",value:\"\"}},{{key:\"kfxgen_application_version\",value:\"ebook-rs\"}},{{key:\"kfxgen_payload_sha1\",value:\"{payload_sha1_hex}\"}},{{key:\"kfxgen_acr\",value:\"{container_id}\"}}]"
        );

        let header_len = info_offset + info_bytes.len() + kfxgen_trailer.len();
        let mut out = Vec::with_capacity(header_len + payload.len());
        out.extend_from_slice(KFX_MAGIC_CONT);
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&(header_len as u32).to_le_bytes());
        out.extend_from_slice(&(info_offset as u32).to_le_bytes());
        out.extend_from_slice(&(info_bytes.len() as u32).to_le_bytes());

        for entry in &entries {
            out.extend_from_slice(&entry.entity_id.to_le_bytes());
            out.extend_from_slice(&entry.type_id.to_le_bytes());
            out.extend_from_slice(&entry.offset.to_le_bytes());
            out.extend_from_slice(&entry.length.to_le_bytes());
        }

        out.extend_from_slice(&doc_symbol_bytes);
        out.extend_from_slice(&fcap_bytes);
        out.extend_from_slice(&info_bytes);
        out.extend_from_slice(kfxgen_trailer.as_bytes());
        debug_assert_eq!(out.len(), header_len);
        out.extend_from_slice(&payload);

        Ok(out)
    }
}

enum AssignedBlockEids {
    Single(i64),
    Table {
        table_eid: i64,
        rows: Vec<(i64, Vec<i64>)>,
    },
}

enum ResolvedHref {
    External(String),
    Internal(i64, i64),
}

fn build_style_events(
    from_psec: usize,
    links: &[LinkSpan],
    sym_alloc: &mut SymbolAlloc,
    get_or_create_anchor: &mut impl FnMut(usize, &str, &mut SymbolAlloc) -> Option<u32>,
) -> Vec<IonValue> {
    let mut events = Vec::new();
    for link in links {
        if link.len == 0 {
            continue;
        }
        if let Some(a_sym) = get_or_create_anchor(from_psec, &link.href, sym_alloc) {
            events.push(IonValue::Struct(vec![
                (143, IonValue::Int(link.start as i64)),
                (144, IonValue::Int(link.len as i64)),
                (179, IonValue::Symbol(u64::from(a_sym))),
            ]));
        }
    }
    events
}

fn build_toc_nav_units(
    points: &[NavPoint],
    resolve_target: &impl Fn(&NavPoint) -> (i64, i64),
) -> Vec<IonValue> {
    let mut out = Vec::new();
    for pt in points {
        let label = pt.label.trim();
        let label = if label.is_empty() {
            "Section".to_string()
        } else {
            label.to_string()
        };
        let (eid, offset) = resolve_target(pt);
        let mut fields = vec![
            (241, IonValue::Struct(vec![(244, IonValue::String(label))])),
            (
                246,
                IonValue::Struct(vec![
                    (155, IonValue::Int(eid)),
                    (143, IonValue::Int(offset)),
                ]),
            ),
        ];
        if !pt.subitems.is_empty() {
            let children = build_toc_nav_units(&pt.subitems, resolve_target);
            if !children.is_empty() {
                fields.push((247, IonValue::List(children)));
            }
        }
        out.push(IonValue::Annotated(
            vec![393],
            Box::new(IonValue::Struct(fields)),
        ));
    }
    out
}

fn compute_publication_ids(
    book: &Book,
    title: &str,
    author: &str,
    lang: &str,
) -> (String, String, String) {
    let meta = book.metadata();
    let feed = |sha: &mut Sha1, domain: &[u8]| {
        sha.update(domain);
        if let Some(id) = &meta.identifier {
            sha.update(id.as_bytes());
        }
        sha.update(b"\x00");
        sha.update(title.as_bytes());
        sha.update(b"\x00");
        sha.update(author.as_bytes());
        sha.update(b"\x00");
        sha.update(lang.as_bytes());
        sha.update(b"\x00");
        for c in &meta.creators {
            sha.update(c.as_bytes());
            sha.update(b"\x01");
        }
        for p in &meta.publishers {
            sha.update(p.as_bytes());
            sha.update(b"\x01");
        }
        sha.update(match meta.direction {
            PageProgressionDirection::Rtl => b"rtl",
            PageProgressionDirection::Ltr => b"ltr",
            PageProgressionDirection::Default => b"def",
        });
        for idx in 0..book.spine().len() {
            if let Ok(sec) = book.get_section(idx) {
                sha.update(sec.full_path.as_bytes());
                sha.update(b"\x02");
                sha.update(sec.raw_html.as_bytes());
                sha.update(b"\x02");
                sha.update(sec.plain_text.as_bytes());
                sha.update(b"\x03");
            }
        }
    };

    let mut s1 = Sha1::new();
    feed(&mut s1, b"ebook-rs-kfx-id-v1-a:");
    let h1 = s1.digest().to_string();

    let mut s2 = Sha1::new();
    feed(&mut s2, b"ebook-rs-kfx-id-v1-b:");
    let h2 = s2.digest().to_string();

    let hex_lower = format!("{h1}{h2}");
    let hex_upper = hex_lower.to_ascii_uppercase();

    let asin = format!("B0{}", &hex_upper[..30]);
    let container_id = format!("CR!{}", &hex_upper[30..58]);
    let book_id = format!("ebookrs_{}", &hex_lower[..24]);
    (container_id, asin, book_id)
}

fn normalize_href_path(path: &str) -> String {
    let no_frag = path.split('#').next().unwrap_or(path);
    let no_query = no_frag.split('?').next().unwrap_or(no_frag);
    let slash_norm = no_query.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for seg in slash_norm.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn push_ion_entity(
    entries: &mut Vec<KfxIndexEntry>,
    payload: &mut Vec<u8>,
    entity_id: u32,
    type_id: u32,
    val: IonValue,
) {
    let ion_bytes = encode_stream(&[val]);
    let offset = payload.len() as u64;
    let length = (ENTY_PREAMBLE.len() + ion_bytes.len()) as u64;
    payload.extend_from_slice(&ENTY_PREAMBLE);
    payload.extend_from_slice(&ion_bytes);
    entries.push(KfxIndexEntry {
        entity_id,
        type_id,
        offset,
        length,
    });
}

fn push_raw_entity(
    entries: &mut Vec<KfxIndexEntry>,
    payload: &mut Vec<u8>,
    entity_id: u32,
    type_id: u32,
    raw_bytes: &[u8],
) {
    let offset = payload.len() as u64;
    let length = (ENTY_PREAMBLE.len() + raw_bytes.len()) as u64;
    payload.extend_from_slice(&ENTY_PREAMBLE);
    payload.extend_from_slice(raw_bytes);
    entries.push(KfxIndexEntry {
        entity_id,
        type_id,
        offset,
        length,
    });
}

fn is_skipped_tag(tag: &str) -> bool {
    matches!(
        tag,
        "head" | "script" | "style" | "noscript" | "title" | "meta" | "link" | "template"
    )
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "div"
            | "section"
            | "article"
            | "aside"
            | "header"
            | "footer"
            | "main"
            | "nav"
            | "blockquote"
            | "pre"
            | "figure"
            | "figcaption"
            | "li"
            | "dt"
            | "dd"
            | "address"
            | "center"
            | "ul"
            | "ol"
            | "dl"
            | "body"
            | "html"
    )
}

fn get_attr_ci<'a>(
    attrs: &'a ahash::AHashMap<compact_str::CompactString, compact_str::CompactString>,
    key: &str,
) -> Option<&'a str> {
    if let Some(v) = attrs.get(key) {
        return Some(v.as_str());
    }
    attrs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

fn push_elem_ids(
    attrs: &ahash::AHashMap<compact_str::CompactString, compact_str::CompactString>,
    pending_ids: &mut Vec<String>,
) {
    for key in ["id", "name"] {
        if let Some(val) = get_attr_ci(attrs, key) {
            let decoded = decode_html_entities(val.trim());
            if !decoded.is_empty() && !pending_ids.contains(&decoded) {
                pending_ids.push(decoded);
            }
        }
    }
}

fn extract_blocks_from_nodes(
    nodes: &[DomNode],
    sec_dir: &str,
    img_collector: &mut ImageCollector<'_>,
    blocks: &mut Vec<ParsedBlock>,
    pending_ids: &mut Vec<String>,
    active_href: Option<&str>,
    in_pre: bool,
) {
    let mut acc = InlineAcc::new();
    walk_nodes_into(
        nodes,
        sec_dir,
        img_collector,
        blocks,
        &mut acc,
        pending_ids,
        active_href,
        in_pre,
    );
    if !acc.is_empty()
        && let Some(tb) = acc.take(pending_ids)
    {
        blocks.push(ParsedBlock::Text(tb));
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_nodes_into(
    nodes: &[DomNode],
    sec_dir: &str,
    img_collector: &mut ImageCollector<'_>,
    blocks: &mut Vec<ParsedBlock>,
    acc: &mut InlineAcc,
    pending_ids: &mut Vec<String>,
    active_href: Option<&str>,
    in_pre: bool,
) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                acc.push_text(t, in_pre);
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                if is_skipped_tag(&tag) {
                    continue;
                }

                if tag == "img" || tag == "image" {
                    if !acc.is_empty()
                        && let Some(tb) = acc.take(pending_ids)
                    {
                        blocks.push(ParsedBlock::Text(tb));
                    }
                    push_elem_ids(attributes, pending_ids);
                    let src = get_attr_ci(attributes, "src")
                        .or_else(|| get_attr_ci(attributes, "xlink:href"))
                        .or_else(|| get_attr_ci(attributes, "href"));
                    if let Some(src_val) = src
                        && let Some(res_idx) = img_collector.collect(sec_dir, src_val)
                    {
                        let alt = get_attr_ci(attributes, "alt")
                            .map(|a| decode_html_entities(a.trim()))
                            .unwrap_or_default();
                        blocks.push(ParsedBlock::Image(ImageBlockData {
                            res_idx,
                            alt,
                            href: active_href.map(ToOwned::to_owned),
                            ids: std::mem::take(pending_ids),
                        }));
                    }
                    continue;
                }

                if tag == "table" {
                    if !acc.is_empty()
                        && let Some(tb) = acc.take(pending_ids)
                    {
                        blocks.push(ParsedBlock::Text(tb));
                    }
                    push_elem_ids(attributes, pending_ids);
                    let table_ids = std::mem::take(pending_ids);
                    let mut rows = Vec::new();
                    collect_table_rows(children, &mut rows);
                    if !rows.is_empty() {
                        blocks.push(ParsedBlock::Table {
                            rows,
                            ids: table_ids,
                        });
                    }
                    continue;
                }

                if tag == "br" {
                    if !acc.is_empty() {
                        acc.push_newline();
                    }
                    continue;
                }

                if is_block_tag(&tag) {
                    if !acc.is_empty()
                        && let Some(tb) = acc.take(pending_ids)
                    {
                        blocks.push(ParsedBlock::Text(tb));
                    }
                    push_elem_ids(attributes, pending_ids);
                    let child_pre = in_pre || tag == "pre";
                    extract_blocks_from_nodes(
                        children,
                        sec_dir,
                        img_collector,
                        blocks,
                        pending_ids,
                        active_href,
                        child_pre,
                    );
                    continue;
                }

                // Inline element (a, span, em, strong, svg, etc.)
                for key in ["id", "name"] {
                    if let Some(val) = get_attr_ci(attributes, key) {
                        let decoded = decode_html_entities(val.trim());
                        if !decoded.is_empty() {
                            if acc.is_empty() {
                                if !pending_ids.contains(&decoded) {
                                    pending_ids.push(decoded);
                                }
                            } else {
                                acc.record_id(&decoded);
                            }
                        }
                    }
                }

                if tag == "a"
                    && let Some(href_raw) = get_attr_ci(attributes, "href")
                {
                    let href = decode_html_entities(href_raw.trim());
                    if !href.is_empty() {
                        let start = acc.char_len;
                        walk_nodes_into(
                            children,
                            sec_dir,
                            img_collector,
                            blocks,
                            acc,
                            pending_ids,
                            Some(&href),
                            in_pre,
                        );
                        let end = acc.char_len;
                        if end > start {
                            acc.links.push(LinkSpan {
                                start,
                                len: end - start,
                                href,
                            });
                        }
                        continue;
                    }
                }

                walk_nodes_into(
                    children,
                    sec_dir,
                    img_collector,
                    blocks,
                    acc,
                    pending_ids,
                    active_href,
                    in_pre,
                );
            }
        }
    }
}

fn collect_table_rows(nodes: &[DomNode], rows: &mut Vec<RowData>) {
    for node in nodes {
        let DomNode::Element {
            tag_name,
            attributes,
            children,
        } = node
        else {
            continue;
        };
        let tag = tag_name.to_ascii_lowercase();
        if tag == "tr" {
            let mut row_ids = Vec::new();
            push_elem_ids(attributes, &mut row_ids);
            let mut cells = Vec::new();
            for cell_node in children {
                let DomNode::Element {
                    tag_name: c_tag_name,
                    attributes: c_attrs,
                    children: c_children,
                } = cell_node
                else {
                    continue;
                };
                let c_tag = c_tag_name.to_ascii_lowercase();
                if c_tag != "td" && c_tag != "th" {
                    continue;
                }
                let mut cell_ids = Vec::new();
                push_elem_ids(c_attrs, &mut cell_ids);
                let colspan = get_attr_ci(c_attrs, "colspan")
                    .and_then(|v| v.trim().parse::<i64>().ok())
                    .filter(|&n| n > 1)
                    .unwrap_or(1);
                let rowspan = get_attr_ci(c_attrs, "rowspan")
                    .and_then(|v| v.trim().parse::<i64>().ok())
                    .filter(|&n| n > 1)
                    .unwrap_or(1);

                let mut acc = InlineAcc::new();
                collect_cell_inline(c_children, &mut acc, &mut cell_ids);
                let (text, links, inline_ids) = match acc.take(&mut cell_ids) {
                    Some(tb) => (tb.text, tb.links, tb.inline_ids),
                    None => (String::new(), Vec::new(), Vec::new()),
                };
                cells.push(CellData {
                    text,
                    links,
                    inline_ids,
                    ids: cell_ids,
                    colspan,
                    rowspan,
                });
            }
            if !cells.is_empty() {
                rows.push(RowData {
                    cells,
                    ids: row_ids,
                });
            }
        } else if matches!(
            tag.as_str(),
            "thead" | "tbody" | "tfoot" | "colgroup" | "caption"
        ) {
            collect_table_rows(children, rows);
        }
    }
}

fn collect_cell_inline(nodes: &[DomNode], acc: &mut InlineAcc, cell_ids: &mut Vec<String>) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => acc.push_text(t, false),
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                if is_skipped_tag(&tag) {
                    continue;
                }
                for key in ["id", "name"] {
                    if let Some(val) = get_attr_ci(attributes, key) {
                        let decoded = decode_html_entities(val.trim());
                        if !decoded.is_empty() {
                            if acc.is_empty() {
                                if !cell_ids.contains(&decoded) {
                                    cell_ids.push(decoded);
                                }
                            } else {
                                acc.record_id(&decoded);
                            }
                        }
                    }
                }
                if tag == "br" {
                    acc.push_newline();
                    continue;
                }
                if is_block_tag(&tag) && !acc.is_empty() {
                    acc.push_newline();
                }
                if tag == "a"
                    && let Some(href_raw) = get_attr_ci(attributes, "href")
                {
                    let href = decode_html_entities(href_raw.trim());
                    if !href.is_empty() {
                        let start = acc.char_len;
                        collect_cell_inline(children, acc, cell_ids);
                        let end = acc.char_len;
                        if end > start {
                            acc.links.push(LinkSpan {
                                start,
                                len: end - start,
                                href,
                            });
                        }
                        continue;
                    }
                }
                collect_cell_inline(children, acc, cell_ids);
            }
        }
    }
}

fn decode_data_uri(uri: &str) -> Option<(Option<String>, Vec<u8>)> {
    let rest = uri.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    let mime = meta
        .split(';')
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);
    if meta
        .split(';')
        .any(|p| p.trim().eq_ignore_ascii_case("base64"))
    {
        let cleaned: String = payload
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .collect();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(cleaned)
            .ok()?;
        Some((mime, bytes))
    } else {
        let decoded = percent_encoding::percent_decode_str(payload).collect::<Vec<u8>>();
        Some((mime, decoded))
    }
}

fn inspect_image(data: &[u8], hint: Option<&str>) -> Option<(String, Option<u32>, u32, u32)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        let (w, h) = parse_png_dims(data).unwrap_or((0, 0));
        return Some(("image/png".to_string(), Some(284), w, h));
    }
    if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        let (w, h) = parse_jpeg_dims(data).unwrap_or((0, 0));
        return Some(("image/jpeg".to_string(), Some(285), w, h));
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        let (w, h) = if data.len() >= 10 {
            (
                u32::from(u16::from_le_bytes([data[6], data[7]])),
                u32::from(u16::from_le_bytes([data[8], data[9]])),
            )
        } else {
            (0, 0)
        };
        return Some(("image/gif".to_string(), Some(286), w, h));
    }
    if data.starts_with(b"BM") && data.len() >= 26 {
        let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]).unsigned_abs();
        let h = i32::from_le_bytes([data[22], data[23], data[24], data[25]]).unsigned_abs();
        return Some(("image/bmp".to_string(), Some(599), w, h));
    }
    if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some(("image/webp".to_string(), None, 0, 0));
    }
    let hint_lower = hint.unwrap_or("").to_ascii_lowercase();
    if hint_lower.ends_with(".svg") || hint_lower.contains("image/svg") {
        return Some(("image/svg+xml".to_string(), Some(287), 0, 0));
    }
    if let Ok(text) = std::str::from_utf8(&data[..data.len().min(512)])
        && (text.contains("<svg") || text.contains("http://www.w3.org/2000/svg"))
    {
        return Some(("image/svg+xml".to_string(), Some(287), 0, 0));
    }
    None
}

fn parse_png_dims(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || &data[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((w, h))
}

fn parse_jpeg_dims(data: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2usize;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        while i < data.len() && data[i] == 0xFF {
            i += 1;
        }
        if i >= data.len() {
            break;
        }
        let marker = data[i];
        i += 1;
        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        if marker == 0x00 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        if i + 2 > data.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
        if seg_len < 2 || i + seg_len > data.len() {
            break;
        }
        let is_sof = matches!(
            marker,
            0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
        );
        if is_sof && i + 7 < data.len() {
            let h = u32::from(u16::from_be_bytes([data[i + 3], data[i + 4]]));
            let w = u32::from(u16::from_be_bytes([data[i + 5], data[i + 6]]));
            return Some((w, h));
        }
        i += seg_len;
    }
    None
}
