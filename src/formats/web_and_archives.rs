//! Pure-Rust Web, Archive, and eBook Package Codecs (Input & Output):
//! - `kepub` (Kobo EPUB 3 Output; Input handled by EPUB 3 parser)
//! - `txt` (Plain Text Output; Input handled by `TxtBook`)
//! - `md` (Markdown Output; Input handled by `TxtBook`)
//! - `fb2` (FictionBook 2.0 Output; Input handled by `Fb2Book`)
//! - `html` (Standalone HTML5 Input & Output with inlined images)
//! - `htmlz` (Calibre HTMLZ ZIP Input & Output)
//! - `txtz` (Calibre TXTZ ZIP Input & Output)
//! - `oeb` (Open eBook Directory / OPF / ZIP Input & Output)
//! - `zip` (Generic HTML/TXT/Image ZIP Input & Output)
//! - `tcr` (Psion Text Compression for Readers `!!8-Bit!!` Input & Output)

use crate::archive::{EpubArchive, resolve_relative_path};
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::dom::{
    DomNode, EbookDomTree, decode_bytes_with_encoding, decode_html_entities, xml_escape,
};
use crate::error::EbookError;
use crate::layout::RenditionLayout;
use crate::metadata::{ManifestItem, Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::{OpfPackage, parse_opf};
use crate::section::{Section, extract_plain_text};
use ahash::{AHashMap, AHashSet};
use base64::Engine as _;
use std::io::{Cursor, Read as _, Write as _};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const TCR_MAGIC: &[u8; 9] = b"!!8-Bit!!";
const MAX_TCR_DECOMPRESSED_BYTES: usize = 64 * 1024 * 1024;

// ============================================================================
// 1. KEPUB (Kobo EPUB 3) Output
// ============================================================================

/// Export any `Book` to a Kobo-instrumented EPUB 3 (`.kepub` / `.kepub.epub`) byte buffer.
/// Wraps chapter content in `#book-columns > #book-inner` and instruments text sentences with
/// `<span class="koboSpan" id="kobo.P.S">` spans for native Kobo pagination & highlighting.
pub fn export_kepub_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let files = build_externalized_oeb_files(book, true, true)?;
    let mut out_buf = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut out_buf));
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        for (name, data) in files {
            let opts = if name == "mimetype" { stored } else { deflated };
            writer
                .start_file(&name, opts)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            writer
                .write_all(&data)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }
        writer
            .finish()
            .map_err(|e| EbookError::Io(format!("KEPUB finalize error: {e}")))?;
    }
    Ok(out_buf)
}

fn build_externalized_oeb_files(
    book: &Book,
    keep_epub_wrapper: bool,
    kepub_instrument: bool,
) -> Result<Vec<(String, Vec<u8>)>, EbookError> {
    let epub_bytes = book.export_epub3_bytes()?;
    let mut reader = ZipArchive::new(Cursor::new(&epub_bytes))
        .map_err(|e| EbookError::InvalidFormat(format!("EPUB staging read failed: {e}")))?;

    let mut ext_assets: Vec<(String, String, Vec<u8>)> = Vec::new(); // (rel_href, mime, bytes)
    let mut ext_by_uri: AHashMap<String, String> = AHashMap::new();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut opf_idx: Option<usize> = None;

    for i in 0..reader.len() {
        let mut entry = reader
            .by_index(i)
            .map_err(|e| EbookError::InvalidFormat(format!("ZIP entry error: {e}")))?;
        let name = entry
            .name()
            .map_err(|e| EbookError::InvalidFormat(format!("ZIP name error: {e}")))?
            .into_owned();
        if name.ends_with('/') {
            continue;
        }
        if !keep_epub_wrapper && (name == "mimetype" || name.starts_with("META-INF/")) {
            continue;
        }
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|e| EbookError::Io(format!("ZIP read error: {e}")))?;

        let out_name = if keep_epub_wrapper {
            name.clone()
        } else {
            let rel = name.strip_prefix("OEBPS/").unwrap_or(&name);
            if rel.is_empty() || rel.contains("..") {
                continue;
            }
            rel.to_string()
        };

        let lower = out_name.to_ascii_lowercase();
        let is_chapter_xhtml = (lower.ends_with(".xhtml") || lower.ends_with(".html"))
            && !lower.ends_with("nav.xhtml")
            && !lower.ends_with("toc.xhtml");

        let final_data = if is_chapter_xhtml {
            if let Ok(xhtml) = std::str::from_utf8(&data) {
                let externalized =
                    externalize_data_image_uris(xhtml, &mut ext_assets, &mut ext_by_uri);
                if kepub_instrument {
                    instrument_kepub_xhtml(&externalized).into_bytes()
                } else {
                    externalized.into_bytes()
                }
            } else {
                data
            }
        } else {
            data
        };

        if lower.ends_with(".opf") {
            opf_idx = Some(files.len());
        }
        files.push((out_name, final_data));
    }

    if !ext_assets.is_empty() {
        let prefix = if keep_epub_wrapper { "OEBPS/" } else { "" };
        if let Some(idx) = opf_idx
            && let Ok(opf_str) = std::str::from_utf8(&files[idx].1)
            && let Some(close_pos) = opf_str.rfind("</manifest>")
        {
            let mut extra_items = String::new();
            for (n, (rel_href, mime, _)) in ext_assets.iter().enumerate() {
                extra_items.push_str(&format!(
                    "    <item id=\"ext_img_{n}\" href=\"{}\" media-type=\"{}\"/>\n",
                    xml_escape(rel_href),
                    xml_escape(mime)
                ));
            }
            let mut updated = String::with_capacity(opf_str.len() + extra_items.len());
            updated.push_str(&opf_str[..close_pos]);
            updated.push_str(&extra_items);
            updated.push_str(&opf_str[close_pos..]);
            files[idx].1 = updated.into_bytes();
        }
        for (rel_href, _, bytes) in ext_assets {
            files.push((format!("{prefix}{rel_href}"), bytes));
        }
    }

    Ok(files)
}

fn externalize_data_image_uris(
    xhtml: &str,
    ext_assets: &mut Vec<(String, String, Vec<u8>)>,
    ext_by_uri: &mut AHashMap<String, String>,
) -> String {
    let mut out = String::with_capacity(xhtml.len());
    let mut pos = 0usize;
    while let Some(rel) = xhtml[pos..].find("data:image/") {
        let start = pos + rel;
        let quote_char = start
            .checked_sub(1)
            .and_then(|idx| xhtml.as_bytes().get(idx).copied())
            .filter(|&b| b == b'"' || b == b'\'');
        let Some(q) = quote_char else {
            out.push_str(&xhtml[pos..start + 11]);
            pos = start + 11;
            continue;
        };
        let Some(end_rel) = xhtml[start..].find(q as char) else {
            break;
        };
        let uri = &xhtml[start..start + end_rel];
        out.push_str(&xhtml[pos..start]);

        if let Some(existing_href) = ext_by_uri.get(uri) {
            out.push_str(existing_href);
        } else if let Some((meta, payload)) =
            uri.strip_prefix("data:").and_then(|r| r.split_once(','))
            && meta
                .split(';')
                .any(|p| p.trim().eq_ignore_ascii_case("base64"))
            && let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(
                payload
                    .chars()
                    .filter(|c| !c.is_ascii_whitespace())
                    .collect::<String>(),
            )
        {
            let mime = meta.split(';').next().unwrap_or("image/jpeg").trim();
            let ext = ext_from_mime(mime);
            let rel_href = format!("images/ext_{}.{ext}", ext_assets.len());
            ext_assets.push((rel_href.clone(), mime.to_string(), decoded));
            ext_by_uri.insert(uri.to_string(), rel_href.clone());
            out.push_str(&rel_href);
        } else {
            out.push_str(uri);
        }
        pos = start + end_rel;
    }
    out.push_str(&xhtml[pos..]);
    out
}

fn instrument_kepub_xhtml(xhtml: &str) -> String {
    let lower = xhtml.to_ascii_lowercase();
    let Some(body_open_start) = lower.find("<body") else {
        return xhtml.to_string();
    };
    let Some(body_open_rel_end) = xhtml[body_open_start..].find('>') else {
        return xhtml.to_string();
    };
    let body_content_start = body_open_start + body_open_rel_end + 1;
    let Some(body_close_start) = lower.rfind("</body>") else {
        return xhtml.to_string();
    };
    if body_close_start < body_content_start {
        return xhtml.to_string();
    }

    let prefix = &xhtml[..body_content_start];
    let inner = &xhtml[body_content_start..body_close_start];
    let suffix = &xhtml[body_close_start..];

    let dom = EbookDomTree::parse(inner);
    let mut para_idx = 0usize;
    let mut rendered = String::with_capacity(inner.len() + inner.len() / 2);
    for node in &dom.root_nodes {
        render_kepub_node(node, &mut para_idx, &mut 0usize, false, &mut rendered);
    }

    format!(
        "{prefix}<div id=\"book-columns\"><div id=\"book-inner\">{rendered}</div></div>{suffix}"
    )
}

fn render_kepub_node(
    node: &DomNode,
    para_idx: &mut usize,
    sent_idx: &mut usize,
    in_kobo_block: bool,
    out: &mut String,
) {
    match node {
        DomNode::Comment(c) => {
            out.push_str("<!--");
            out.push_str(c);
            out.push_str("-->");
        }
        DomNode::Text(t) => {
            if !in_kobo_block || t.trim().is_empty() {
                out.push_str(&xml_escape(t));
                return;
            }
            for seg in split_sentences_preserving_ws(t) {
                if seg.trim().is_empty() {
                    out.push_str(&xml_escape(seg));
                } else {
                    *sent_idx += 1;
                    out.push_str(&format!(
                        "<span class=\"koboSpan\" id=\"kobo.{}.{}\">{}</span>",
                        *para_idx,
                        *sent_idx,
                        xml_escape(seg)
                    ));
                }
            }
        }
        DomNode::Element {
            tag_name,
            attributes,
            children,
        } => {
            let tag_l = tag_name.to_ascii_lowercase();
            let is_void = matches!(
                tag_l.as_str(),
                "area"
                    | "base"
                    | "br"
                    | "col"
                    | "embed"
                    | "hr"
                    | "img"
                    | "input"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            );
            let is_skip_kobo =
                matches!(tag_l.as_str(), "script" | "style" | "pre" | "code" | "svg");
            let starts_block = matches!(
                tag_l.as_str(),
                "p" | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "li"
                    | "dt"
                    | "dd"
                    | "blockquote"
                    | "td"
                    | "th"
                    | "figcaption"
            );

            out.push('<');
            out.push_str(tag_name);
            let mut attr_pairs: Vec<(&str, &str)> = attributes
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            attr_pairs.sort_by_key(|(k, _)| *k);
            for (k, v) in attr_pairs {
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                out.push_str(&xml_escape(v));
                out.push('"');
            }
            if is_void {
                out.push_str("/>");
                return;
            }
            out.push('>');

            if is_skip_kobo {
                for ch in children {
                    render_kepub_node(ch, para_idx, sent_idx, false, out);
                }
            } else if starts_block {
                *para_idx += 1;
                let mut local_sent = 0usize;
                for ch in children {
                    render_kepub_node(ch, para_idx, &mut local_sent, true, out);
                }
            } else {
                for ch in children {
                    render_kepub_node(ch, para_idx, sent_idx, in_kobo_block, out);
                }
            }

            out.push_str("</");
            out.push_str(tag_name);
            out.push('>');
        }
    }
}

fn split_sentences_preserving_ws(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if matches!(b, b'.' | b'!' | b'?')
            && i + 1 < bytes.len()
            && bytes[i + 1].is_ascii_whitespace()
        {
            let end = i + 1;
            out.push(&text[start..end]);
            start = end;
        }
        i += 1;
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

// ============================================================================
// 2. Plain Text (.txt) Output
// ============================================================================

/// Export any `Book` to clean UTF-8 plain text (`.txt`).
pub fn export_txt_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let mut out = String::new();
    let meta = book.metadata();
    if !meta.title.trim().is_empty() {
        out.push_str(meta.title.trim());
        out.push('\n');
        if let Some(author) = meta
            .creators
            .first()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            out.push_str(author);
            out.push('\n');
        }
        out.push('\n');
    }

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let mut sec_text = String::new();
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_plain_text(&dom.root_nodes, &mut sec_text);
        }
        let cleaned = if sec_text.trim().is_empty() {
            sec.plain_text.trim().to_string()
        } else {
            normalize_blank_lines(&sec_text)
        };
        if !cleaned.is_empty() {
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            out.push_str(&cleaned);
            out.push_str("\n\n");
        }
    }

    if out.trim().is_empty() {
        out.push_str(meta.title.trim());
        out.push('\n');
    }

    Ok(out.into_bytes())
}

fn dom_to_plain_text(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let decoded = decode_html_entities(t);
                for ch in decoded.chars() {
                    if ch.is_ascii_whitespace() {
                        if !out.is_empty() && !out.ends_with([' ', '\n']) {
                            out.push(' ');
                        }
                    } else {
                        out.push(ch);
                    }
                }
            }
            DomNode::Element {
                tag_name, children, ..
            } => {
                let tag = tag_name.to_ascii_lowercase();
                if matches!(
                    tag.as_str(),
                    "head" | "script" | "style" | "noscript" | "title"
                ) {
                    continue;
                }
                if tag == "br" {
                    out.push('\n');
                    continue;
                }
                if tag == "hr" {
                    out.push_str("\n---\n\n");
                    continue;
                }
                let is_block = matches!(
                    tag.as_str(),
                    "p" | "div"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "li"
                        | "blockquote"
                        | "pre"
                        | "tr"
                        | "section"
                        | "article"
                );
                if is_block && !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                if tag == "li" {
                    out.push_str("- ");
                }
                dom_to_plain_text(children, out);
                if matches!(tag.as_str(), "td" | "th") {
                    out.push('\t');
                } else if is_block {
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                    if matches!(
                        tag.as_str(),
                        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote"
                    ) && !out.ends_with("\n\n")
                    {
                        out.push('\n');
                    }
                }
            }
        }
    }
}

fn normalize_blank_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut blank_run = 0usize;
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run == 1 && !out.is_empty() {
                out.push('\n');
            }
        } else {
            blank_run = 0;
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    out.trim().to_string()
}

// ============================================================================
// 3. Markdown (.md) Output
// ============================================================================

/// Export any `Book` to structured Markdown (`.md`) with YAML frontmatter.
pub fn export_markdown_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let meta = book.metadata();
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!(
        "title: \"{}\"\n",
        meta.title.trim().replace('"', "\\\"")
    ));
    if let Some(author) = meta
        .creators
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        out.push_str(&format!("author: \"{}\"\n", author.replace('"', "\\\"")));
    }
    if !meta.language().trim().is_empty() {
        out.push_str(&format!("language: \"{}\"\n", meta.language().trim()));
    }
    if let Some(pub_name) = meta
        .publishers
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        out.push_str(&format!(
            "publisher: \"{}\"\n",
            pub_name.replace('"', "\\\"")
        ));
    }
    if let Some(desc) = meta
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let single_line = desc.replace(['\r', '\n'], " ");
        out.push_str(&format!(
            "description: \"{}\"\n",
            single_line.replace('"', "\\\"")
        ));
    }
    out.push_str("---\n\n");

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let mut md = String::new();
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_markdown(&dom.root_nodes, &mut md, 0);
        }
        let cleaned = if md.trim().is_empty() {
            sec.plain_text.trim().to_string()
        } else {
            normalize_blank_lines(&md)
        };
        if !cleaned.is_empty() {
            out.push_str(&cleaned);
            out.push_str("\n\n");
        }
    }

    Ok(out.into_bytes())
}

fn dom_to_markdown(nodes: &[DomNode], out: &mut String, list_depth: usize) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let decoded = decode_html_entities(t);
                for ch in decoded.chars() {
                    if ch.is_ascii_whitespace() {
                        if !out.is_empty() && !out.ends_with([' ', '\n']) {
                            out.push(' ');
                        }
                    } else {
                        out.push(ch);
                    }
                }
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                if matches!(
                    tag.as_str(),
                    "head" | "script" | "style" | "noscript" | "title"
                ) {
                    continue;
                }
                match tag.as_str() {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        let lvl = tag[1..].parse::<usize>().unwrap_or(1);
                        let mut inner = String::new();
                        dom_to_markdown(children, &mut inner, list_depth);
                        let trimmed = inner.trim();
                        if !trimmed.is_empty() {
                            if !out.is_empty() && !out.ends_with("\n\n") {
                                out.push_str("\n\n");
                            }
                            out.push_str(&"#".repeat(lvl));
                            out.push(' ');
                            out.push_str(trimmed);
                            out.push_str("\n\n");
                        }
                    }
                    "p" => {
                        let mut inner = String::new();
                        dom_to_markdown(children, &mut inner, list_depth);
                        let trimmed = inner.trim();
                        if !trimmed.is_empty() {
                            if !out.is_empty() && !out.ends_with("\n\n") {
                                out.push_str("\n\n");
                            }
                            out.push_str(trimmed);
                            out.push_str("\n\n");
                        }
                    }
                    "strong" | "b" => {
                        let mut inner = String::new();
                        dom_to_markdown(children, &mut inner, list_depth);
                        let trimmed = inner.trim();
                        if !trimmed.is_empty() {
                            out.push_str("**");
                            out.push_str(trimmed);
                            out.push_str("**");
                        }
                    }
                    "em" | "i" => {
                        let mut inner = String::new();
                        dom_to_markdown(children, &mut inner, list_depth);
                        let trimmed = inner.trim();
                        if !trimmed.is_empty() {
                            out.push('*');
                            out.push_str(trimmed);
                            out.push('*');
                        }
                    }
                    "code" => {
                        let mut inner = String::new();
                        dom_to_plain_text(children, &mut inner);
                        let trimmed = inner.trim();
                        if !trimmed.is_empty() {
                            out.push('`');
                            out.push_str(trimmed);
                            out.push('`');
                        }
                    }
                    "pre" => {
                        let mut inner = String::new();
                        dom_to_plain_text(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str("\n\n```\n");
                            out.push_str(inner.trim_end());
                            out.push_str("\n```\n\n");
                        }
                    }
                    "a" => {
                        let href = attributes.get("href").map(|s| s.as_str()).unwrap_or("");
                        let mut inner = String::new();
                        dom_to_markdown(children, &mut inner, list_depth);
                        let trimmed = inner.trim();
                        if !trimmed.is_empty() {
                            if !href.is_empty() {
                                out.push('[');
                                out.push_str(trimmed);
                                out.push_str("](");
                                out.push_str(href);
                                out.push(')');
                            } else {
                                out.push_str(trimmed);
                            }
                        }
                    }
                    "img" => {
                        let src = attributes.get("src").map(|s| s.as_str()).unwrap_or("");
                        let alt = attributes.get("alt").map(|s| s.as_str()).unwrap_or("");
                        if !src.is_empty() {
                            out.push_str(&format!("![{alt}]({src})"));
                        }
                    }
                    "blockquote" => {
                        let mut inner = String::new();
                        dom_to_markdown(children, &mut inner, list_depth);
                        let cleaned = normalize_blank_lines(&inner);
                        if !cleaned.is_empty() {
                            out.push_str("\n\n");
                            for line in cleaned.lines() {
                                out.push_str("> ");
                                out.push_str(line);
                                out.push('\n');
                            }
                            out.push('\n');
                        }
                    }
                    "ul" | "ol" => {
                        if !out.is_empty() && !out.ends_with('\n') {
                            out.push('\n');
                        }
                        dom_to_markdown(children, out, list_depth + 1);
                        if !out.ends_with("\n\n") {
                            out.push('\n');
                        }
                    }
                    "li" => {
                        let mut inner = String::new();
                        dom_to_markdown(children, &mut inner, list_depth);
                        let trimmed = inner.trim();
                        if !trimmed.is_empty() {
                            let indent = "  ".repeat(list_depth.saturating_sub(1));
                            out.push_str(&format!("{indent}- {trimmed}\n"));
                        }
                    }
                    "table" => {
                        let mut rows: Vec<Vec<String>> = Vec::new();
                        collect_md_table_rows(children, &mut rows);
                        if !rows.is_empty() {
                            let cols = rows.iter().map(Vec::len).max().unwrap_or(1);
                            out.push_str("\n\n");
                            for (r_idx, row) in rows.iter().enumerate() {
                                out.push('|');
                                for c in 0..cols {
                                    let cell = row.get(c).map(String::as_str).unwrap_or("");
                                    out.push(' ');
                                    out.push_str(&cell.replace('|', "\\|"));
                                    out.push_str(" |");
                                }
                                out.push('\n');
                                if r_idx == 0 {
                                    out.push('|');
                                    for _ in 0..cols {
                                        out.push_str(" --- |");
                                    }
                                    out.push('\n');
                                }
                            }
                            out.push('\n');
                        }
                    }
                    "br" => out.push_str("  \n"),
                    "hr" => out.push_str("\n\n---\n\n"),
                    _ => dom_to_markdown(children, out, list_depth),
                }
            }
        }
    }
}

fn collect_md_table_rows(nodes: &[DomNode], rows: &mut Vec<Vec<String>>) {
    for node in nodes {
        if let DomNode::Element {
            tag_name, children, ..
        } = node
        {
            let tag = tag_name.to_ascii_lowercase();
            if tag == "tr" {
                let mut cells = Vec::new();
                for ch in children {
                    if let DomNode::Element {
                        tag_name: ct,
                        children: cc,
                        ..
                    } = ch
                        && (ct.eq_ignore_ascii_case("td") || ct.eq_ignore_ascii_case("th"))
                    {
                        let mut txt = String::new();
                        dom_to_plain_text(cc, &mut txt);
                        cells.push(txt.trim().replace('\n', " "));
                    }
                }
                if !cells.is_empty() {
                    rows.push(cells);
                }
            } else {
                collect_md_table_rows(children, rows);
            }
        }
    }
}

// ============================================================================
// 4. FictionBook 2.0 (.fb2) Output
// ============================================================================

/// Export any `Book` to valid FictionBook 2.0 (`.fb2`) XML with base64 `<binary>` images.
pub fn export_fb2_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let meta = book.metadata();
    let title = if meta.title.trim().is_empty() {
        "Untitled"
    } else {
        meta.title.trim()
    };
    let lang = if meta.language().trim().is_empty() {
        "en"
    } else {
        meta.language().trim()
    };

    let mut binaries: Vec<(String, String, Vec<u8>)> = Vec::new(); // (id, mime, data)
    let mut bin_by_path: AHashMap<String, String> = AHashMap::new();

    let register_image = |full_path: &str,
                          data: Vec<u8>,
                          binaries: &mut Vec<(String, String, Vec<u8>)>,
                          bin_by_path: &mut AHashMap<String, String>|
     -> String {
        if let Some(existing) = bin_by_path.get(full_path) {
            return existing.clone();
        }
        let mime = guess_image_mime(full_path, &data);
        let ext = ext_from_mime(mime);
        let id = format!("img_{}.{ext}", binaries.len());
        bin_by_path.insert(full_path.to_string(), id.clone());
        binaries.push((id.clone(), mime.to_string(), data));
        id
    };

    let mut cover_bin_id: Option<String> = None;
    if let Some((cov_data, cov_mime)) = book.cover_image() {
        let ext = ext_from_mime(cov_mime);
        let id = format!("cover.{ext}");
        let key = meta
            .cover_href
            .clone()
            .unwrap_or_else(|| format!("__cover__.{ext}"));
        bin_by_path.insert(key, id.clone());
        binaries.push((id.clone(), cov_mime.to_string(), cov_data));
        cover_bin_id = Some(id);
    }

    let mut body_sections_xml = String::new();
    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let sec_dir = sec.full_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let mut sec_xml = String::new();
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_fb2(
                &dom.root_nodes,
                book,
                sec_dir,
                &mut binaries,
                &mut bin_by_path,
                &register_image,
                &mut sec_xml,
            );
        }
        if sec_xml.trim().is_empty() && !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                sec_xml.push_str(&format!("<p>{}</p>\n", xml_escape(para)));
            }
        }
        if !sec_xml.trim().is_empty() {
            body_sections_xml.push_str("<section>\n");
            body_sections_xml.push_str(&sec_xml);
            body_sections_xml.push_str("</section>\n");
        }
    }

    if body_sections_xml.is_empty() {
        body_sections_xml.push_str(&format!(
            "<section><p>{}</p></section>\n",
            xml_escape(title)
        ));
    }

    let mut xml = String::with_capacity(body_sections_xml.len() + 4096);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\" xmlns:l=\"http://www.w3.org/1999/xlink\">\n");
    xml.push_str("<description>\n  <title-info>\n    <genre>prose</genre>\n");
    if meta.creators.is_empty() {
        xml.push_str(
            "    <author><first-name>Unknown</first-name><last-name>Author</last-name></author>\n",
        );
    } else {
        for creator in &meta.creators {
            let c = creator.trim();
            if c.is_empty() {
                continue;
            }
            if let Some((first, last)) = c.rsplit_once(' ') {
                xml.push_str(&format!(
                    "    <author><first-name>{}</first-name><last-name>{}</last-name></author>\n",
                    xml_escape(first.trim()),
                    xml_escape(last.trim())
                ));
            } else {
                xml.push_str(&format!(
                    "    <author><last-name>{}</last-name></author>\n",
                    xml_escape(c)
                ));
            }
        }
    }
    xml.push_str(&format!(
        "    <book-title>{}</book-title>\n",
        xml_escape(title)
    ));
    if let Some(desc) = meta
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        xml.push_str(&format!(
            "    <annotation><p>{}</p></annotation>\n",
            xml_escape(desc)
        ));
    }
    if let Some(cov_id) = &cover_bin_id {
        xml.push_str(&format!(
            "    <coverpage><image l:href=\"#{}\"/></coverpage>\n",
            xml_escape(cov_id)
        ));
    }
    xml.push_str(&format!("    <lang>{}</lang>\n", xml_escape(lang)));
    xml.push_str("  </title-info>\n");
    xml.push_str("  <document-info>\n");
    xml.push_str("    <author><nickname>ebook-rs</nickname></author>\n");
    xml.push_str("    <program-used>ebook-rs</program-used>\n");
    let doc_id = meta
        .identifier
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("ebook-rs-fb2");
    xml.push_str(&format!("    <id>{}</id>\n", xml_escape(doc_id)));
    xml.push_str("    <version>1.0</version>\n");
    xml.push_str("  </document-info>\n");
    xml.push_str("</description>\n<body>\n");
    xml.push_str(&body_sections_xml);
    xml.push_str("</body>\n");

    for (id, mime, data) in binaries {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
        xml.push_str(&format!(
            "<binary id=\"{}\" content-type=\"{}\">{}</binary>\n",
            xml_escape(&id),
            xml_escape(&mime),
            b64
        ));
    }
    xml.push_str("</FictionBook>\n");

    Ok(xml.into_bytes())
}

#[allow(clippy::too_many_arguments)]
fn dom_to_fb2<F>(
    nodes: &[DomNode],
    book: &Book,
    sec_dir: &str,
    binaries: &mut Vec<(String, String, Vec<u8>)>,
    bin_by_path: &mut AHashMap<String, String>,
    register_image: &F,
    out: &mut String,
) where
    F: Fn(
        &str,
        Vec<u8>,
        &mut Vec<(String, String, Vec<u8>)>,
        &mut AHashMap<String, String>,
    ) -> String,
{
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(_) => {}
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                if matches!(
                    tag.as_str(),
                    "head" | "script" | "style" | "noscript" | "title"
                ) {
                    continue;
                }
                match tag.as_str() {
                    "h1" | "h2" => {
                        let mut txt = String::new();
                        dom_to_plain_text(children, &mut txt);
                        let t = txt.trim();
                        if !t.is_empty() {
                            out.push_str(&format!("<title><p>{}</p></title>\n", xml_escape(t)));
                        }
                    }
                    "h3" | "h4" | "h5" | "h6" => {
                        let mut txt = String::new();
                        dom_to_plain_text(children, &mut txt);
                        let t = txt.trim();
                        if !t.is_empty() {
                            out.push_str(&format!("<subtitle>{}</subtitle>\n", xml_escape(t)));
                        }
                    }
                    "img" | "image" => {
                        let src = attributes
                            .get("src")
                            .or_else(|| attributes.get("xlink:href"))
                            .or_else(|| attributes.get("href"))
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        if let Some((key, data)) = resolve_book_image(book, sec_dir, src) {
                            let id = register_image(&key, data, binaries, bin_by_path);
                            out.push_str(&format!("<image l:href=\"#{}\"/>\n", xml_escape(&id)));
                        }
                    }
                    "p" | "li" | "dt" | "dd" | "figcaption" => {
                        // Extract any nested images as block images + inline formatted text
                        let mut inline_xml = String::new();
                        let mut block_imgs = Vec::new();
                        render_fb2_inline(
                            children,
                            book,
                            sec_dir,
                            binaries,
                            bin_by_path,
                            register_image,
                            &mut inline_xml,
                            &mut block_imgs,
                        );
                        if !inline_xml.trim().is_empty() {
                            out.push_str("<p>");
                            out.push_str(inline_xml.trim());
                            out.push_str("</p>\n");
                        }
                        for img_id in block_imgs {
                            out.push_str(&format!(
                                "<image l:href=\"#{}\"/>\n",
                                xml_escape(&img_id)
                            ));
                        }
                    }
                    "blockquote" => {
                        let mut inner = String::new();
                        dom_to_fb2(
                            children,
                            book,
                            sec_dir,
                            binaries,
                            bin_by_path,
                            register_image,
                            &mut inner,
                        );
                        if !inner.trim().is_empty() {
                            out.push_str("<cite>\n");
                            out.push_str(&inner);
                            out.push_str("</cite>\n");
                        }
                    }
                    "table" => {
                        let mut rows: Vec<Vec<String>> = Vec::new();
                        collect_md_table_rows(children, &mut rows);
                        if !rows.is_empty() {
                            out.push_str("<table>\n");
                            for row in rows {
                                out.push_str("  <tr>");
                                for cell in row {
                                    out.push_str(&format!("<td>{}</td>", xml_escape(&cell)));
                                }
                                out.push_str("</tr>\n");
                            }
                            out.push_str("</table>\n");
                        }
                    }
                    _ => {
                        // Container elements (div, section, body, etc.)
                        let has_block_child = children.iter().any(|c| {
                            matches!(
                                c,
                                DomNode::Element { tag_name: ct, .. }
                                if matches!(
                                    ct.to_ascii_lowercase().as_str(),
                                    "p" | "div" | "section" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                                        | "ul" | "ol" | "table" | "blockquote" | "body" | "article"
                                )
                            )
                        });
                        if has_block_child {
                            dom_to_fb2(
                                children,
                                book,
                                sec_dir,
                                binaries,
                                bin_by_path,
                                register_image,
                                out,
                            );
                        } else {
                            let mut inline_xml = String::new();
                            let mut block_imgs = Vec::new();
                            render_fb2_inline(
                                children,
                                book,
                                sec_dir,
                                binaries,
                                bin_by_path,
                                register_image,
                                &mut inline_xml,
                                &mut block_imgs,
                            );
                            if !inline_xml.trim().is_empty() {
                                out.push_str("<p>");
                                out.push_str(inline_xml.trim());
                                out.push_str("</p>\n");
                            }
                            for img_id in block_imgs {
                                out.push_str(&format!(
                                    "<image l:href=\"#{}\"/>\n",
                                    xml_escape(&img_id)
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_fb2_inline<F>(
    nodes: &[DomNode],
    book: &Book,
    sec_dir: &str,
    binaries: &mut Vec<(String, String, Vec<u8>)>,
    bin_by_path: &mut AHashMap<String, String>,
    register_image: &F,
    out: &mut String,
    block_imgs: &mut Vec<String>,
) where
    F: Fn(
        &str,
        Vec<u8>,
        &mut Vec<(String, String, Vec<u8>)>,
        &mut AHashMap<String, String>,
    ) -> String,
{
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let decoded = decode_html_entities(t);
                out.push_str(&xml_escape(&decoded));
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                match tag.as_str() {
                    "strong" | "b" => {
                        out.push_str("<strong>");
                        render_fb2_inline(
                            children,
                            book,
                            sec_dir,
                            binaries,
                            bin_by_path,
                            register_image,
                            out,
                            block_imgs,
                        );
                        out.push_str("</strong>");
                    }
                    "em" | "i" => {
                        out.push_str("<emphasis>");
                        render_fb2_inline(
                            children,
                            book,
                            sec_dir,
                            binaries,
                            bin_by_path,
                            register_image,
                            out,
                            block_imgs,
                        );
                        out.push_str("</emphasis>");
                    }
                    "sup" => {
                        out.push_str("<sup>");
                        render_fb2_inline(
                            children,
                            book,
                            sec_dir,
                            binaries,
                            bin_by_path,
                            register_image,
                            out,
                            block_imgs,
                        );
                        out.push_str("</sup>");
                    }
                    "sub" => {
                        out.push_str("<sub>");
                        render_fb2_inline(
                            children,
                            book,
                            sec_dir,
                            binaries,
                            bin_by_path,
                            register_image,
                            out,
                            block_imgs,
                        );
                        out.push_str("</sub>");
                    }
                    "code" => {
                        out.push_str("<code>");
                        render_fb2_inline(
                            children,
                            book,
                            sec_dir,
                            binaries,
                            bin_by_path,
                            register_image,
                            out,
                            block_imgs,
                        );
                        out.push_str("</code>");
                    }
                    "img" | "image" => {
                        let src = attributes
                            .get("src")
                            .or_else(|| attributes.get("xlink:href"))
                            .or_else(|| attributes.get("href"))
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        if let Some((key, data)) = resolve_book_image(book, sec_dir, src) {
                            let id = register_image(&key, data, binaries, bin_by_path);
                            block_imgs.push(id);
                        }
                    }
                    "br" => out.push(' '),
                    _ => render_fb2_inline(
                        children,
                        book,
                        sec_dir,
                        binaries,
                        bin_by_path,
                        register_image,
                        out,
                        block_imgs,
                    ),
                }
            }
        }
    }
}

// ============================================================================
// 5. Standalone HTML (.html / .htm / .xhtml) Input & Output
// ============================================================================

/// Detect whether raw bytes represent a standalone HTML/XHTML document (not FB2/OPF XML).
pub fn is_standalone_html(bytes: &[u8]) -> bool {
    let prefix_len = bytes.len().min(2048);
    let head = String::from_utf8_lossy(&bytes[..prefix_len]).to_ascii_lowercase();
    let trimmed = head.trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n']);
    if trimmed.contains("<fictionbook") || trimmed.contains("<package") {
        return false;
    }
    trimmed.starts_with("<!doctype html")
        || trimmed.starts_with("<html")
        || (trimmed.starts_with("<?xml") && trimmed.contains("<html"))
        || (trimmed.contains("<html") && trimmed.contains("<body"))
}

/// Parse a standalone HTML/XHTML file into a `Book`.
pub fn parse_html(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
    let html_str = decode_bytes_with_encoding(bytes, None);
    if html_str.trim().is_empty() {
        return Err(EbookError::InvalidFormat("HTML file is empty".to_string()));
    }
    let dom = EbookDomTree::parse(&html_str);
    let mut title = String::new();
    let mut creators = Vec::new();
    let mut description = None;
    let mut languages = Vec::new();
    let mut direction = PageProgressionDirection::Ltr;

    extract_html_head_meta(
        &dom.root_nodes,
        &mut title,
        &mut creators,
        &mut description,
        &mut languages,
        &mut direction,
    );

    if title.trim().is_empty() {
        title = title_fallback.to_string();
    }
    if languages.is_empty() {
        languages.push("en".to_string());
    }

    let body_html = extract_body_inner_html(&html_str).unwrap_or_else(|| html_str.clone());
    let plain_text = extract_plain_text(&body_html);
    let plain_text_lower = plain_text.to_lowercase();
    let char_count = plain_text.chars().count();

    let sec_href = "index.xhtml".to_string();
    let section = Section {
        index: 0,
        idref: "sec_0".to_string(),
        href: sec_href.clone(),
        full_path: sec_href.clone(),
        raw_html: html_str.clone(),
        processed_html: body_html,
        plain_text,
        plain_text_lower,
        char_count,
        viewport_width: None,
        viewport_height: None,
    };

    let spine = vec![SpineItem {
        idref: "sec_0".to_string(),
        linear: true,
        properties: Vec::new(),
        index: 0,
        href: sec_href.clone(),
        media_type: "application/xhtml+xml".to_string(),
    }];

    let toc = vec![NavPoint {
        id: "nav_1".to_string(),
        label: title.clone(),
        href: sec_href.clone(),
        full_path: sec_href.clone(),
        subitems: Vec::new(),
    }];

    let mut archive = EpubArchive::empty();
    archive.insert(sec_href.clone(), html_str.into_bytes());

    let mut manifest = AHashMap::new();
    manifest.insert(
        "sec_0".to_string(),
        ManifestItem {
            id: "sec_0".to_string(),
            href: sec_href.clone(),
            full_path: sec_href,
            media_type: "application/xhtml+xml".to_string(),
            properties: Vec::new(),
            fallback: None,
            media_overlay: None,
        },
    );

    let metadata = Metadata {
        title,
        creators,
        publishers: Vec::new(),
        languages,
        rights: None,
        description,
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
        version: "3.0".to_string(),
        opf_path: "content.opf".to_string(),
        opf_dir: String::new(),
        metadata,
        manifest,
        spine,
        guide: Vec::new(),
        toc_item_id: None,
        nav_item_id: None,
    };

    Ok(build_synthetic_book(archive, opf, toc, vec![section]))
}

/// Export any `Book` to a self-contained single-file HTML5 document with inlined images and CSS.
pub fn export_html_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let meta = book.metadata();
    let title = if meta.title.trim().is_empty() {
        "Untitled"
    } else {
        meta.title.trim()
    };
    let lang = if meta.language().trim().is_empty() {
        "en"
    } else {
        meta.language().trim()
    };
    let dir_attr = if meta.direction == PageProgressionDirection::Rtl {
        " dir=\"rtl\""
    } else {
        ""
    };

    let merged_css = collect_book_css(book);

    let mut out = String::with_capacity(65536);
    out.push_str("<!DOCTYPE html>\n");
    out.push_str(&format!(
        "<html lang=\"{}\"{dir_attr}>\n<head>\n<meta charset=\"UTF-8\"/>\n",
        xml_escape(lang)
    ));
    out.push_str(&format!("<title>{}</title>\n", xml_escape(title)));
    if let Some(author) = meta
        .creators
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        out.push_str(&format!(
            "<meta name=\"author\" content=\"{}\"/>\n",
            xml_escape(author)
        ));
    }
    if let Some(desc) = meta
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        out.push_str(&format!(
            "<meta name=\"description\" content=\"{}\"/>\n",
            xml_escape(desc)
        ));
    }
    out.push_str("<style>\nbody { max-width: 44em; margin: 2em auto; padding: 0 1.25em; line-height: 1.6; }\nimg { max-width: 100%; height: auto; }\n");
    if !merged_css.is_empty() {
        out.push_str(&merged_css);
        out.push('\n');
    }
    out.push_str("</style>\n</head>\n<body>\n");

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let sec_dir = sec.full_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let body_inner =
            extract_body_inner_html(&sec.raw_html).unwrap_or_else(|| sec.raw_html.clone());
        let inlined = rewrite_html_images(&body_inner, |raw_src| {
            let (key, data) = resolve_book_image(book, sec_dir, raw_src)?;
            let mime = guess_image_mime(&key, &data);
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            Some(format!("data:{mime};base64,{b64}"))
        });
        out.push_str(&format!(
            "<section id=\"sec_{idx}\">\n{inlined}\n</section>\n"
        ));
    }

    out.push_str("</body>\n</html>\n");
    Ok(out.into_bytes())
}

// ============================================================================
// 6 & 7 & 8 & 9. HTMLZ, TXTZ, OEB (Dir/OPF/ZIP), and Generic ZIP (Input & Output)
// ============================================================================

/// Export any `Book` to Calibre-compatible HTMLZ (`.htmlz`) archive:
/// `index.html`, `style.css`, `metadata.opf`, optional `cover.*`, and `images/*`.
pub fn export_htmlz_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    export_html_zip_bundle(book, true)
}

/// Export any `Book` to a generic HTML/CSS/Images `.zip` archive (Calibre ZIP compatible).
pub fn export_zip_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    export_html_zip_bundle(book, false)
}

fn export_html_zip_bundle(book: &Book, include_opf: bool) -> Result<Vec<u8>, EbookError> {
    let meta = book.metadata();
    let title = if meta.title.trim().is_empty() {
        "Untitled"
    } else {
        meta.title.trim()
    };
    let lang = if meta.language().trim().is_empty() {
        "en"
    } else {
        meta.language().trim()
    };
    let dir_attr = if meta.direction == PageProgressionDirection::Rtl {
        " dir=\"rtl\""
    } else {
        ""
    };

    let mut img_files: Vec<(String, Vec<u8>)> = Vec::new(); // ("images/000000.jpg", bytes)
    let mut img_by_key: AHashMap<String, String> = AHashMap::new();

    let mut body_buf = String::new();
    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let sec_dir = sec.full_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let body_inner =
            extract_body_inner_html(&sec.raw_html).unwrap_or_else(|| sec.raw_html.clone());
        let rewritten = rewrite_html_images(&body_inner, |raw_src| {
            let (key, data) = resolve_book_image(book, sec_dir, raw_src)?;
            if let Some(rel) = img_by_key.get(&key) {
                return Some(rel.clone());
            }
            let mime = guess_image_mime(&key, &data);
            let ext = ext_from_mime(mime);
            let rel = format!("images/{:06}.{ext}", img_files.len());
            img_by_key.insert(key, rel.clone());
            img_files.push((rel.clone(), data));
            Some(rel)
        });
        body_buf.push_str(&format!("<div id=\"sec_{idx}\">\n{rewritten}\n</div>\n"));
    }

    let css = collect_book_css(book);
    let index_html = format!(
        "<!DOCTYPE html>\n<html lang=\"{}\"{dir_attr}>\n<head>\n<meta charset=\"UTF-8\"/>\n<title>{}</title>\n<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/>\n</head>\n<body>\n{body_buf}</body>\n</html>\n",
        xml_escape(lang),
        xml_escape(title)
    );

    let mut cover_name: Option<String> = None;
    let cover_entry = book.cover_image().map(|(data, mime)| {
        let ext = ext_from_mime(mime);
        let cname = format!("cover.{ext}");
        cover_name = Some(cname.clone());
        (cname, data)
    });

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("index.html", deflated)
            .map_err(|e| EbookError::Io(e.to_string()))?;
        zip.write_all(index_html.as_bytes())
            .map_err(|e| EbookError::Io(e.to_string()))?;

        zip.start_file("style.css", deflated)
            .map_err(|e| EbookError::Io(e.to_string()))?;
        zip.write_all(css.as_bytes())
            .map_err(|e| EbookError::Io(e.to_string()))?;

        if include_opf {
            let opf_xml = build_metadata_opf(meta, cover_name.as_deref());
            zip.start_file("metadata.opf", deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(opf_xml.as_bytes())
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }

        if let Some((cname, cdata)) = cover_entry {
            zip.start_file(&cname, deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(&cdata)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }

        for (rel, data) in img_files {
            zip.start_file(&rel, deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(&data)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }

        zip.finish()
            .map_err(|e| EbookError::Io(format!("ZIP finalize error: {e}")))?;
    }
    Ok(buf)
}

/// Export any `Book` to Calibre-compatible TXTZ (`.txtz`) archive:
/// `index.txt`, `metadata.opf`, and optional `cover.*`.
pub fn export_txtz_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let txt_bytes = export_txt_bytes(book)?;
    let meta = book.metadata();
    let mut cover_name: Option<String> = None;
    let cover_entry = book.cover_image().map(|(data, mime)| {
        let ext = ext_from_mime(mime);
        let cname = format!("cover.{ext}");
        cover_name = Some(cname.clone());
        (cname, data)
    });
    let opf_xml = build_metadata_opf(meta, cover_name.as_deref());

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("index.txt", deflated)
            .map_err(|e| EbookError::Io(e.to_string()))?;
        zip.write_all(&txt_bytes)
            .map_err(|e| EbookError::Io(e.to_string()))?;

        zip.start_file("metadata.opf", deflated)
            .map_err(|e| EbookError::Io(e.to_string()))?;
        zip.write_all(opf_xml.as_bytes())
            .map_err(|e| EbookError::Io(e.to_string()))?;

        if let Some((cname, cdata)) = cover_entry {
            zip.start_file(&cname, deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(&cdata)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }

        zip.finish()
            .map_err(|e| EbookError::Io(format!("TXTZ finalize error: {e}")))?;
    }
    Ok(buf)
}

/// Export any `Book` to an unzipped Open eBook (OEB) directory containing `content.opf`,
/// `toc.ncx`, `nav.xhtml`, XHTML chapters, CSS, and images.
pub fn export_oeb_dir<P: AsRef<Path>>(book: &Book, dir_path: P) -> Result<(), EbookError> {
    let files = build_externalized_oeb_files(book, false, false)?;
    let root = dir_path.as_ref();
    std::fs::create_dir_all(root)
        .map_err(|e| EbookError::Io(format!("Failed to create OEB dir: {e}")))?;

    for (rel, data) in files {
        let target = root.join(&rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| EbookError::Io(format!("Failed to create OEB subdir: {e}")))?;
        }
        std::fs::write(&target, &data)
            .map_err(|e| EbookError::Io(format!("Failed to write {}: {e}", target.display())))?;
    }
    Ok(())
}

/// Export any `Book` to an in-memory OEB ZIP package (flat OPF + chapters + assets, no `META-INF`).
pub fn export_oeb_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let files = build_externalized_oeb_files(book, false, false)?;
    let mut out_buf = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut out_buf));
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (rel, data) in files {
            writer
                .start_file(&rel, deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            writer
                .write_all(&data)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }
        writer.finish().map_err(|e| EbookError::Io(e.to_string()))?;
    }
    Ok(out_buf)
}

/// Load an Open eBook (OEB) from a directory path or `.opf` file path.
pub fn parse_oeb_path(path: &Path) -> Result<Book, EbookError> {
    let (root_dir, explicit_opf) = if path.is_dir() {
        (path.to_path_buf(), None)
    } else if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("opf"))
    {
        let parent = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let rel_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(ToOwned::to_owned);
        (parent, rel_name)
    } else {
        return Err(EbookError::InvalidFormat(
            "Expected an OEB directory or .opf file".to_string(),
        ));
    };

    let mut archive = EpubArchive::empty();
    let mut opf_path: Option<String> = explicit_opf;
    collect_dir_into_archive(&root_dir, &root_dir, &mut archive, &mut opf_path)?;

    let opf_rel = opf_path.ok_or_else(|| {
        EbookError::InvalidFormat(format!("No .opf package found in {}", root_dir.display()))
    })?;

    let container_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n  <rootfiles>\n    <rootfile full-path=\"{}\" media-type=\"application/oebps-package+xml\"/>\n  </rootfiles>\n</container>\n",
        xml_escape(&opf_rel)
    );
    archive.insert("META-INF/container.xml", container_xml.into_bytes());

    Book::from_archive_with_passphrase(archive, None)
}

fn collect_dir_into_archive(
    root: &Path,
    current: &Path,
    archive: &mut EpubArchive,
    opf_path: &mut Option<String>,
) -> Result<(), EbookError> {
    let entries = std::fs::read_dir(current)
        .map_err(|e| EbookError::Io(format!("Failed to read dir {}: {e}", current.display())))?;
    let mut sorted: Vec<_> = entries.filter_map(Result::ok).collect();
    sorted.sort_by_key(|e| e.file_name());

    for entry in sorted {
        let p = entry.path();
        if p.is_dir() {
            collect_dir_into_archive(root, &p, archive, opf_path)?;
        } else if p.is_file() {
            let Ok(rel) = p.strip_prefix(root) else {
                continue;
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if opf_path.is_none() && rel_str.to_ascii_lowercase().ends_with(".opf") {
                *opf_path = Some(rel_str.clone());
            }
            if let Ok(bytes) = std::fs::read(&p) {
                archive.insert(rel_str, bytes);
            }
        }
    }
    Ok(())
}

/// Probe a non-EPUB/DOCX/ODT ZIP archive for OEB (`.opf`), HTMLZ (`index.html`),
/// TXTZ (`index.txt`), or generic HTML/TXT ZIP books. Returns `Err` if it looks like
/// an image-only CBZ archive so the caller can fall back to `CbzBook`.
pub fn parse_zip_ebook(archive: &EpubArchive, title_fallback: &str) -> Result<Book, EbookError> {
    let mut files = archive.list_files();
    files.sort();

    // 1. Check for OEB ZIP: has an .opf file with a <spine> (not just metadata.opf)
    if let Some(opf_file) = files
        .iter()
        .find(|f| f.to_ascii_lowercase().ends_with(".opf"))
        && let Ok(opf_str) = archive.read_string(opf_file)
        && let Ok(parsed_opf) = parse_opf(&opf_str, opf_file)
        && !parsed_opf.spine.is_empty()
    {
        let mut oeb_archive = archive.clone();
        let container_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n  <rootfiles>\n    <rootfile full-path=\"{}\" media-type=\"application/oebps-package+xml\"/>\n  </rootfiles>\n</container>\n",
            xml_escape(opf_file)
        );
        oeb_archive.insert("META-INF/container.xml", container_xml.into_bytes());
        if let Ok(book) = Book::from_archive_with_passphrase(oeb_archive, None) {
            return Ok(book);
        }
    }

    // Optional Calibre metadata.opf
    let opf_meta: Option<Metadata> = files
        .iter()
        .find(|f| f.to_ascii_lowercase().ends_with("metadata.opf"))
        .and_then(|f| archive.read_string(f).ok())
        .and_then(|s| parse_opf(&s, "metadata.opf").ok())
        .map(|p| p.metadata);

    // 2. HTMLZ / generic HTML ZIP
    let mut html_files: Vec<String> = files
        .iter()
        .filter(|f| {
            let l = f.to_ascii_lowercase();
            (l.ends_with(".html") || l.ends_with(".xhtml") || l.ends_with(".htm"))
                && !l.starts_with("__macosx/")
        })
        .cloned()
        .collect();

    if !html_files.is_empty() {
        // Put index.html / index.xhtml first if present
        html_files.sort_by(|a, b| {
            let a_idx = a
                .rsplit('/')
                .next()
                .unwrap_or(a)
                .to_ascii_lowercase()
                .starts_with("index.");
            let b_idx = b
                .rsplit('/')
                .next()
                .unwrap_or(b)
                .to_ascii_lowercase()
                .starts_with("index.");
            b_idx.cmp(&a_idx).then_with(|| a.cmp(b))
        });

        let mut sections = Vec::new();
        let mut spine = Vec::new();
        let mut toc = Vec::new();
        let mut manifest = AHashMap::new();
        let mut inferred_title = String::new();
        let mut inferred_creators = Vec::new();
        let mut inferred_desc = None;
        let mut inferred_langs = Vec::new();
        let mut inferred_dir = PageProgressionDirection::Ltr;

        for (idx, hpath) in html_files.iter().enumerate() {
            let Ok(bytes) = archive.read_bytes(hpath) else {
                continue;
            };
            let raw_html = decode_bytes_with_encoding(&bytes, None);
            if idx == 0 {
                let dom = EbookDomTree::parse(&raw_html);
                extract_html_head_meta(
                    &dom.root_nodes,
                    &mut inferred_title,
                    &mut inferred_creators,
                    &mut inferred_desc,
                    &mut inferred_langs,
                    &mut inferred_dir,
                );
            }
            let body_inner = extract_body_inner_html(&raw_html).unwrap_or_else(|| raw_html.clone());
            let plain_text = extract_plain_text(&body_inner);
            let plain_text_lower = plain_text.to_lowercase();
            let char_count = plain_text.chars().count();
            let idref = format!("sec_{idx}");

            manifest.insert(
                idref.clone(),
                ManifestItem {
                    id: idref.clone(),
                    href: hpath.clone(),
                    full_path: hpath.clone(),
                    media_type: "application/xhtml+xml".to_string(),
                    properties: Vec::new(),
                    fallback: None,
                    media_overlay: None,
                },
            );
            spine.push(SpineItem {
                idref: idref.clone(),
                linear: true,
                properties: Vec::new(),
                index: idx,
                href: hpath.clone(),
                media_type: "application/xhtml+xml".to_string(),
            });
            let label = hpath
                .rsplit('/')
                .next()
                .unwrap_or(hpath)
                .rsplit_once('.')
                .map(|(s, _)| s)
                .unwrap_or("Section")
                .to_string();
            toc.push(NavPoint {
                id: format!("nav_{idx}"),
                label,
                href: hpath.clone(),
                full_path: hpath.clone(),
                subitems: Vec::new(),
            });
            sections.push(Section {
                index: idx,
                idref,
                href: hpath.clone(),
                full_path: hpath.clone(),
                raw_html,
                processed_html: body_inner,
                plain_text,
                plain_text_lower,
                char_count,
                viewport_width: None,
                viewport_height: None,
            });
        }

        if sections.is_empty() {
            return Err(EbookError::InvalidFormat(
                "No readable HTML in ZIP".to_string(),
            ));
        }

        // Register all non-HTML image/CSS assets in manifest
        let mut cover_id = None;
        let mut cover_href = None;
        for (a_idx, f) in files.iter().enumerate() {
            let l = f.to_ascii_lowercase();
            if l.starts_with("__macosx/") {
                continue;
            }
            if let Some(mime) = mime_for_asset_path(&l) {
                let id = format!("asset_{a_idx}");
                let base = l.rsplit('/').next().unwrap_or(&l);
                if cover_href.is_none() && base.starts_with("cover.") && mime.starts_with("image/")
                {
                    cover_id = Some(id.clone());
                    cover_href = Some(f.clone());
                }
                manifest.insert(
                    id.clone(),
                    ManifestItem {
                        id,
                        href: f.clone(),
                        full_path: f.clone(),
                        media_type: mime.to_string(),
                        properties: Vec::new(),
                        fallback: None,
                        media_overlay: None,
                    },
                );
            }
        }

        let mut metadata = opf_meta.unwrap_or_else(|| Metadata {
            title: if inferred_title.trim().is_empty() {
                title_fallback.to_string()
            } else {
                inferred_title
            },
            creators: inferred_creators,
            publishers: Vec::new(),
            languages: if inferred_langs.is_empty() {
                vec!["en".to_string()]
            } else {
                inferred_langs
            },
            rights: None,
            description: inferred_desc,
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects: Vec::new(),
            cover_id: None,
            cover_href: None,
            direction: inferred_dir,
            meta_properties: AHashMap::new(),
            accessibility: Default::default(),
        });
        if metadata.title.trim().is_empty() {
            metadata.title = title_fallback.to_string();
        }
        if metadata.cover_href.is_none() && cover_href.is_some() {
            metadata.cover_id = cover_id;
            metadata.cover_href = cover_href;
        }

        let opf = OpfPackage {
            version: "3.0".to_string(),
            opf_path: "metadata.opf".to_string(),
            opf_dir: String::new(),
            metadata,
            manifest,
            spine,
            guide: Vec::new(),
            toc_item_id: None,
            nav_item_id: None,
        };
        return Ok(build_synthetic_book(archive.clone(), opf, toc, sections));
    }

    // 3. TXTZ / generic TXT/MD ZIP
    let txt_files: Vec<String> = files
        .iter()
        .filter(|f| {
            let l = f.to_ascii_lowercase();
            (l.ends_with(".txt") || l.ends_with(".md")) && !l.starts_with("__macosx/")
        })
        .cloned()
        .collect();

    if !txt_files.is_empty() {
        let mut combined = Vec::new();
        for tpath in &txt_files {
            if let Ok(b) = archive.read_bytes(tpath) {
                if !combined.is_empty() {
                    combined.extend_from_slice(b"\n\n");
                }
                combined.extend_from_slice(&b);
            }
        }
        let is_md = txt_files
            .iter()
            .any(|f| f.to_ascii_lowercase().ends_with(".md"));
        let mut book = crate::txt::TxtBook::parse(&combined, title_fallback, is_md)?;
        if let Some(om) = opf_meta {
            if !om.title.trim().is_empty() {
                book.opf.metadata.title = om.title;
            }
            if !om.creators.is_empty() {
                book.opf.metadata.creators = om.creators;
            }
            if !om.languages.is_empty() {
                book.opf.metadata.languages = om.languages;
            }
            if om.description.is_some() {
                book.opf.metadata.description = om.description;
            }
        }
        // Preserve cover & images from TXTZ archive
        for (a_idx, f) in files.iter().enumerate() {
            let l = f.to_ascii_lowercase();
            if let Some(mime) = mime_for_asset_path(&l)
                && let Ok(bytes) = archive.read_bytes(f)
            {
                book.archive.insert(f.clone(), bytes);
                let id = format!("asset_{a_idx}");
                let base = l.rsplit('/').next().unwrap_or(&l);
                if book.opf.metadata.cover_href.is_none()
                    && base.starts_with("cover")
                    && mime.starts_with("image/")
                {
                    book.opf.metadata.cover_id = Some(id.clone());
                    book.opf.metadata.cover_href = Some(f.clone());
                }
                book.opf.manifest.insert(
                    id.clone(),
                    ManifestItem {
                        id,
                        href: f.clone(),
                        full_path: f.clone(),
                        media_type: mime.to_string(),
                        properties: Vec::new(),
                        fallback: None,
                        media_overlay: None,
                    },
                );
            }
        }
        return Ok(book);
    }

    Err(EbookError::InvalidFormat(
        "ZIP does not contain HTML/TXT/OPF eBook content".to_string(),
    ))
}

// ============================================================================
// 10. Psion TCR (`!!8-Bit!!`) Input & Output
// ============================================================================

/// Detect Psion TCR (`!!8-Bit!!`) magic header.
pub fn is_tcr_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(TCR_MAGIC)
}

/// Parse a Psion TCR (`.tcr`) compressed text eBook into a `Book`.
pub fn parse_tcr(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
    if !bytes.starts_with(TCR_MAGIC) {
        return Err(EbookError::InvalidFormat(
            "Invalid TCR header (expected !!8-Bit!!)".to_string(),
        ));
    }
    let mut pos = TCR_MAGIC.len();
    let mut dict: Vec<&[u8]> = Vec::with_capacity(256);
    for _ in 0..256 {
        if pos >= bytes.len() {
            return Err(EbookError::InvalidFormat(
                "Truncated TCR dictionary".to_string(),
            ));
        }
        let len = bytes[pos] as usize;
        pos += 1;
        if pos + len > bytes.len() {
            return Err(EbookError::InvalidFormat(
                "Truncated TCR dictionary entry".to_string(),
            ));
        }
        dict.push(&bytes[pos..pos + len]);
        pos += len;
    }

    let mut out = Vec::with_capacity((bytes.len().saturating_sub(pos)).saturating_mul(2));
    for &code in &bytes[pos..] {
        let entry = dict[code as usize];
        if out.len() + entry.len() > MAX_TCR_DECOMPRESSED_BYTES {
            return Err(EbookError::InvalidFormat(
                "TCR decompressed payload exceeds 64 MiB safety budget".to_string(),
            ));
        }
        out.extend_from_slice(entry);
    }

    let decoded = decode_bytes_with_encoding(&out, None);
    crate::txt::TxtBook::parse(decoded.as_bytes(), title_fallback, false)
}

/// Export any `Book` to a Psion TCR (`.tcr`) compressed file using a 256-entry dictionary.
pub fn export_tcr_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let raw = export_txt_bytes(book)?;

    // Build initial dictionary from all unique bytes present in `raw`
    let mut dict: Vec<Vec<u8>> = Vec::with_capacity(256);
    let mut byte_to_code: [Option<u8>; 256] = [None; 256];
    let mut tokens: Vec<u8> = Vec::with_capacity(raw.len());

    for &b in &raw {
        let code = if let Some(c) = byte_to_code[b as usize] {
            c
        } else {
            let c = dict.len() as u8;
            byte_to_code[b as usize] = Some(c);
            dict.push(vec![b]);
            c
        };
        tokens.push(code);
    }

    // Greedily merge most frequent token pairs while dictionary has room (< 256)
    while dict.len() < 256 && tokens.len() >= 4 {
        let mut counts: AHashMap<(u8, u8), usize> = AHashMap::new();
        for w in tokens.windows(2) {
            *counts.entry((w[0], w[1])).or_insert(0) += 1;
        }
        let best = counts
            .into_iter()
            .filter(|&((a, b), cnt)| {
                cnt >= 3 && dict[a as usize].len() + dict[b as usize].len() <= 255
            })
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)));
        let Some(((a, b), _)) = best else {
            break;
        };

        let new_code = dict.len() as u8;
        let mut merged = dict[a as usize].clone();
        merged.extend_from_slice(&dict[b as usize]);
        dict.push(merged);

        let mut next_tokens = Vec::with_capacity(tokens.len());
        let mut i = 0usize;
        while i < tokens.len() {
            if i + 1 < tokens.len() && tokens[i] == a && tokens[i + 1] == b {
                next_tokens.push(new_code);
                i += 2;
            } else {
                next_tokens.push(tokens[i]);
                i += 1;
            }
        }
        tokens = next_tokens;
    }

    let mut out = Vec::with_capacity(TCR_MAGIC.len() + 512 + tokens.len());
    out.extend_from_slice(TCR_MAGIC);
    for idx in 0..256 {
        if let Some(entry) = dict.get(idx) {
            out.push(entry.len() as u8);
            out.extend_from_slice(entry);
        } else {
            out.push(0);
        }
    }
    out.extend_from_slice(&tokens);
    Ok(out)
}

// ============================================================================
// Shared Helpers
// ============================================================================

fn build_synthetic_book(
    archive: EpubArchive,
    opf: OpfPackage,
    toc: Vec<NavPoint>,
    sections: Vec<Section>,
) -> Book {
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
    book
}

fn build_metadata_opf(meta: &Metadata, cover_href: Option<&str>) -> String {
    let title = if meta.title.trim().is_empty() {
        "Untitled"
    } else {
        meta.title.trim()
    };
    let lang = if meta.language().trim().is_empty() {
        "en"
    } else {
        meta.language().trim()
    };
    let uid = meta
        .identifier
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("urn:uuid:ebook-rs-0000");

    let mut opf = String::new();
    opf.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    opf.push_str("<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"2.0\" unique-identifier=\"uuid_id\">\n");
    opf.push_str("  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:opf=\"http://www.idpf.org/2007/opf\">\n");
    opf.push_str(&format!(
        "    <dc:identifier id=\"uuid_id\">{}</dc:identifier>\n",
        xml_escape(uid)
    ));
    opf.push_str(&format!("    <dc:title>{}</dc:title>\n", xml_escape(title)));
    for c in &meta.creators {
        if !c.trim().is_empty() {
            opf.push_str(&format!(
                "    <dc:creator opf:role=\"aut\">{}</dc:creator>\n",
                xml_escape(c.trim())
            ));
        }
    }
    opf.push_str(&format!(
        "    <dc:language>{}</dc:language>\n",
        xml_escape(lang)
    ));
    if let Some(desc) = meta
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        opf.push_str(&format!(
            "    <dc:description>{}</dc:description>\n",
            xml_escape(desc)
        ));
    }
    if let Some(pub_name) = meta
        .publishers
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        opf.push_str(&format!(
            "    <dc:publisher>{}</dc:publisher>\n",
            xml_escape(pub_name)
        ));
    }
    if let Some(ch) = cover_href {
        opf.push_str(&format!(
            "    <meta name=\"cover\" content=\"{}\"/>\n",
            xml_escape(ch)
        ));
    }
    opf.push_str("  </metadata>\n</package>\n");
    opf
}

fn collect_book_css(book: &Book) -> String {
    let mut css = String::new();
    let mut seen: AHashSet<String> = AHashSet::new();
    let mut paths: Vec<String> = book
        .opf()
        .manifest
        .values()
        .filter(|i| {
            i.media_type.eq_ignore_ascii_case("text/css")
                || i.full_path.to_ascii_lowercase().ends_with(".css")
        })
        .map(|i| i.full_path.clone())
        .collect();
    paths.sort();
    for p in paths {
        if !seen.insert(p.clone()) {
            continue;
        }
        if let Ok(s) = book.archive().read_string(&p) {
            css.push_str(&s);
            css.push('\n');
        }
    }
    css
}

fn extract_body_inner_html(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = lower.find("<body")?;
    let open_end = html[open..].find('>')? + open + 1;
    let close = lower.rfind("</body>").unwrap_or(html.len());
    (close >= open_end).then(|| html[open_end..close].to_string())
}

fn extract_html_head_meta(
    nodes: &[DomNode],
    title: &mut String,
    creators: &mut Vec<String>,
    description: &mut Option<String>,
    languages: &mut Vec<String>,
    direction: &mut PageProgressionDirection,
) {
    for node in nodes {
        if let DomNode::Element {
            tag_name,
            attributes,
            children,
        } = node
        {
            let tag = tag_name.to_ascii_lowercase();
            if tag == "html" || tag == "body" {
                if let Some(d) = attributes.get("dir")
                    && d.eq_ignore_ascii_case("rtl")
                {
                    *direction = PageProgressionDirection::Rtl;
                }
                if languages.is_empty()
                    && let Some(l) = attributes
                        .get("lang")
                        .or_else(|| attributes.get("xml:lang"))
                    && !l.trim().is_empty()
                {
                    languages.push(l.trim().to_string());
                }
            }
            if tag == "title" && title.is_empty() {
                let mut t = String::new();
                dom_to_plain_text(children, &mut t);
                let trimmed = t.trim();
                if !trimmed.is_empty() {
                    *title = trimmed.to_string();
                }
            } else if tag == "meta" {
                let name = attributes
                    .get("name")
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or_default();
                let content = attributes
                    .get("content")
                    .map(|s| decode_html_entities(s.trim()))
                    .unwrap_or_default();
                if !content.is_empty() {
                    match name.as_str() {
                        "author" | "dc.creator" => {
                            if !creators.contains(&content) {
                                creators.push(content);
                            }
                        }
                        "description" | "dc.description" => {
                            if description.is_none() {
                                *description = Some(content);
                            }
                        }
                        "language" | "dc.language" if languages.is_empty() => {
                            languages.push(content);
                        }
                        _ => {}
                    }
                }
            }
            extract_html_head_meta(children, title, creators, description, languages, direction);
        }
    }
}

fn rewrite_html_images<F>(html: &str, mut map_src: F) -> String
where
    F: FnMut(&str) -> Option<String>,
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
                if let Some(new_src) = map_src(&raw_src) {
                    out.push_str(&tag_slice[..val_start]);
                    out.push(quote);
                    out.push_str(&xml_escape(&new_src));
                    out.push(quote);
                    out.push_str(&after_q[q_end + 1..]);
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

fn guess_image_mime(path: &str, data: &[u8]) -> &'static str {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return "image/png";
    }
    if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        return "image/jpeg";
    }
    if data.starts_with(b"GIF8") {
        return "image/gif";
    }
    if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return "image/webp";
    }
    let l = path.to_ascii_lowercase();
    if l.ends_with(".svg") {
        "image/svg+xml"
    } else if l.ends_with(".png") {
        "image/png"
    } else if l.ends_with(".gif") {
        "image/gif"
    } else if l.ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg"
    }
}

fn ext_from_mime(mime: &str) -> &'static str {
    match mime.to_ascii_lowercase().as_str() {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        _ => "jpg",
    }
}

fn mime_for_asset_path(lower: &str) -> Option<&'static str> {
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        Some("image/jpeg")
    } else if lower.ends_with(".png") {
        Some("image/png")
    } else if lower.ends_with(".gif") {
        Some("image/gif")
    } else if lower.ends_with(".webp") {
        Some("image/webp")
    } else if lower.ends_with(".svg") {
        Some("image/svg+xml")
    } else if lower.ends_with(".css") {
        Some("text/css")
    } else {
        None
    }
}
