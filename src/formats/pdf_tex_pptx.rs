//! Pure-Rust PDF Output, LaTeX (.tex) Input & Output, and PowerPoint (.pptx) Input & Output Codecs.

use crate::archive::EpubArchive;
use crate::book::Book;
use crate::deobfuscate::FontDeobfuscator;
use crate::dom::{
    DomNode, EbookDomTree, decode_bytes_with_encoding, decode_html_entities, xml_escape,
};
use crate::error::EbookError;
use crate::layout::RenditionLayout;
use crate::metadata::{Metadata, PageProgressionDirection, SpineItem};
use crate::nav::NavPoint;
use crate::opf::OpfPackage;
use crate::section::{Section, extract_plain_text};
use ahash::AHashMap;
use std::io::{Cursor, Write as _};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

// ============================================================================
// 1. Pure-Rust Reflowable PDF 1.4 Writer
// ============================================================================

/// Export any `Book` to a clean, paginated, zero-dependency pure-Rust PDF 1.4 document.
/// Uses standard PDF Type1 Helvetica / Helvetica-Bold fonts with word-wrapping, chapter headings,
/// page numbers, and PDF Info metadata (`/Title`, `/Author`, `/Creator`).
pub fn export_pdf_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
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

    // Extract styled lines: (is_heading, text)
    let mut blocks: Vec<(bool, String)> = Vec::new();
    blocks.push((true, title.to_string()));
    if author != "Unknown Author" {
        blocks.push((false, format!("By {author}")));
    }

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            extract_pdf_blocks(&dom.root_nodes, &mut blocks);
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                blocks.push((false, para.to_string()));
            }
        }
    }

    // Paginate onto US Letter (612 x 792 pt), 54pt margins
    const PAGE_W: f32 = 612.0;
    const PAGE_H: f32 = 792.0;
    const MARGIN_X: f32 = 54.0;
    const TOP_Y: f32 = 738.0;
    const BOTTOM_Y: f32 = 64.0;

    let mut pages_streams: Vec<String> = Vec::new();
    let mut cur_stream = String::new();
    let mut y = TOP_Y;

    let flush_page = |pages: &mut Vec<String>, stream: &mut String| {
        if !stream.is_empty() {
            let page_num = pages.len() + 1;
            stream.push_str(&format!(
                "BT /F1 9 Tf {:.1} 32 Td ({}) Tj ET\n",
                PAGE_W / 2.0 - 12.0,
                page_num
            ));
            pages.push(std::mem::take(stream));
        }
    };

    for (is_heading, text) in blocks {
        let font = if is_heading { "/F2" } else { "/F1" };
        let font_size: f32 = if is_heading { 15.0 } else { 11.0 };
        let line_h: f32 = if is_heading { 21.0 } else { 15.5 };
        let max_chars = if is_heading { 58 } else { 82 };

        if is_heading && y < TOP_Y - 10.0 {
            y -= 8.0;
        }
        let wrapped = wrap_text_lines(&text, max_chars);
        for line in wrapped {
            if y - line_h < BOTTOM_Y {
                flush_page(&mut pages_streams, &mut cur_stream);
                y = TOP_Y;
            }
            let escaped = pdf_escape_winansi(&line);
            cur_stream.push_str(&format!(
                "BT {font} {font_size:.1} Tf {MARGIN_X:.1} {y:.1} Td ({escaped}) Tj ET\n"
            ));
            y -= line_h;
        }
        y -= if is_heading { 6.0 } else { 5.0 };
    }
    flush_page(&mut pages_streams, &mut cur_stream);
    if pages_streams.is_empty() {
        pages_streams.push(format!(
            "BT /F2 15.0 Tf {MARGIN_X:.1} {TOP_Y:.1} Td ({}) Tj ET\n",
            pdf_escape_winansi(title)
        ));
    }

    // Object numbering:
    // 1: Catalog
    // 2: Pages
    // 3: Font F1 (Helvetica)
    // 4: Font F2 (Helvetica-Bold)
    // 5: Info
    // For each page i (0..N):
    //   page_obj = 6 + 2*i
    //   content_obj = 7 + 2*i
    let num_pages = pages_streams.len();
    let total_objs = 5 + num_pages * 2;
    let mut offsets: Vec<usize> = vec![0; total_objs + 1];
    let mut pdf = Vec::with_capacity(pages_streams.iter().map(String::len).sum::<usize>() + 4096);
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let mut write_obj = |id: usize, body: &[u8]| {
        offsets[id] = pdf.len();
        pdf.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    };

    write_obj(1, b"<< /Type /Catalog /Pages 2 0 R >>");

    let mut kids = String::new();
    for i in 0..num_pages {
        let p_id = 6 + i * 2;
        kids.push_str(&format!("{p_id} 0 R "));
    }
    write_obj(
        2,
        format!("<< /Type /Pages /Kids [{kids}] /Count {num_pages} >>").as_bytes(),
    );
    write_obj(
        3,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    write_obj(
        4,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>",
    );
    write_obj(
        5,
        format!(
            "<< /Title ({}) /Author ({}) /Creator (ebook-rs) >>",
            pdf_escape_winansi(title),
            pdf_escape_winansi(author)
        )
        .as_bytes(),
    );

    for (i, stream) in pages_streams.iter().enumerate() {
        let p_id = 6 + i * 2;
        let c_id = 7 + i * 2;
        write_obj(
            p_id,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W:.0} {PAGE_H:.0}] \
/Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {c_id} 0 R >>"
            )
            .as_bytes(),
        );
        let s_bytes = stream.as_bytes();
        let mut c_body = Vec::with_capacity(s_bytes.len() + 64);
        c_body.extend_from_slice(format!("<< /Length {} >>\nstream\n", s_bytes.len()).as_bytes());
        c_body.extend_from_slice(s_bytes);
        c_body.extend_from_slice(b"endstream");
        write_obj(c_id, &c_body);
    }

    let xref_off = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", total_objs + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets[1..] {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref_off}\n%%EOF\n",
            total_objs + 1
        )
        .as_bytes(),
    );

    Ok(pdf)
}

fn extract_pdf_blocks(nodes: &[DomNode], out: &mut Vec<(bool, String)>) {
    for node in nodes {
        let DomNode::Element {
            tag_name, children, ..
        } = node
        else {
            continue;
        };
        let tag = tag_name.to_ascii_lowercase();
        if matches!(
            tag.as_str(),
            "head" | "script" | "style" | "noscript" | "title"
        ) {
            continue;
        }
        match tag.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let mut t = String::new();
                collect_text(children, &mut t);
                let clean = collapse_ws(&t).trim().to_string();
                if !clean.is_empty() {
                    out.push((true, clean));
                }
            }
            "p" | "li" | "blockquote" | "pre" | "tr" | "dt" | "dd" | "figcaption" => {
                let mut t = String::new();
                collect_text(children, &mut t);
                let clean = collapse_ws(&t).trim().to_string();
                if !clean.is_empty() {
                    let prefix = if tag == "li" { "- " } else { "" };
                    out.push((false, format!("{prefix}{clean}")));
                }
            }
            _ => {
                if has_block_child(children) {
                    extract_pdf_blocks(children, out);
                } else {
                    let mut t = String::new();
                    collect_text(children, &mut t);
                    let clean = collapse_ws(&t).trim().to_string();
                    if !clean.is_empty() {
                        out.push((false, clean));
                    }
                }
            }
        }
    }
}

fn wrap_text_lines(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.chars().count() + 1 + word.chars().count() <= max_chars {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur.push_str(word);
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn pdf_escape_winansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let mapped = match ch {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            '\u{2013}' | '\u{2014}' => '-',
            '\u{2026}' => '.',
            '\u{00A0}' => ' ',
            c if (c as u32) >= 32 && (c as u32) <= 126 => c,
            c if (c as u32) >= 160 && (c as u32) <= 255 => c,
            _ => ' ',
        };
        match mapped {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            c if (c as u32) > 126 => out.push_str(&format!("\\{:03o}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

// ============================================================================
// 2. LaTeX (.tex) Input & Output
// ============================================================================

/// Detect LaTeX (`.tex`) input heuristics (or `.tex` extension).
pub fn is_tex_input(text: &str, title_fallback: &str) -> bool {
    if title_fallback.to_ascii_lowercase().ends_with(".tex") {
        return true;
    }
    text.contains("\\documentclass") || text.contains("\\begin{document}")
}

/// Parse a LaTeX (`.tex`) document subset into a `Book`.
pub fn parse_tex(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
    let text = decode_bytes_with_encoding(bytes, None);
    let mut title = title_fallback.to_string();
    let mut creators = Vec::new();
    let mut html = String::new();

    if let Some(t) = extract_tex_braced(&text, "\\title{") {
        title = clean_tex_inline(&t);
    }
    if let Some(a) = extract_tex_braced(&text, "\\author{") {
        let ca = clean_tex_inline(&a);
        if !ca.is_empty() {
            creators.push(ca);
        }
    }

    let body = if let Some(start) = text.find("\\begin{document}") {
        let after = &text[start + "\\begin{document}".len()..];
        if let Some(end) = after.find("\\end{document}") {
            &after[..end]
        } else {
            after
        }
    } else {
        text.as_str()
    };

    let mut in_verbatim = false;
    let mut code_buf = String::new();
    let mut para_buf = String::new();

    let flush_para = |para: &mut String, out: &mut String| {
        let trimmed = para.trim();
        if !trimmed.is_empty() {
            out.push_str(&format!("<p>{}</p>\n", render_tex_inline(trimmed)));
        }
        para.clear();
    };

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\\begin{verbatim}") {
            flush_para(&mut para_buf, &mut html);
            in_verbatim = true;
            continue;
        }
        if trimmed.starts_with("\\end{verbatim}") {
            html.push_str(&format!(
                "<pre><code>{}</code></pre>\n",
                xml_escape(&code_buf)
            ));
            code_buf.clear();
            in_verbatim = false;
            continue;
        }
        if in_verbatim {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }
        if trimmed.starts_with('%')
            || trimmed == "\\maketitle"
            || trimmed == "\\tableofcontents"
            || trimmed.starts_with("\\begin{itemize}")
            || trimmed.starts_with("\\end{itemize}")
            || trimmed.starts_with("\\begin{enumerate}")
            || trimmed.starts_with("\\end{enumerate}")
            || trimmed.starts_with("\\begin{quote}")
            || trimmed.starts_with("\\end{quote}")
        {
            flush_para(&mut para_buf, &mut html);
            continue;
        }
        if trimmed.is_empty() {
            flush_para(&mut para_buf, &mut html);
            continue;
        }
        if let Some(h) = extract_tex_braced(trimmed, "\\chapter{")
            .or_else(|| extract_tex_braced(trimmed, "\\chapter*{"))
        {
            flush_para(&mut para_buf, &mut html);
            html.push_str(&format!("<h1>{}</h1>\n", render_tex_inline(&h)));
            continue;
        }
        if let Some(h) = extract_tex_braced(trimmed, "\\section{")
            .or_else(|| extract_tex_braced(trimmed, "\\section*{"))
        {
            flush_para(&mut para_buf, &mut html);
            html.push_str(&format!("<h2>{}</h2>\n", render_tex_inline(&h)));
            continue;
        }
        if let Some(h) = extract_tex_braced(trimmed, "\\subsection{")
            .or_else(|| extract_tex_braced(trimmed, "\\subsection*{"))
        {
            flush_para(&mut para_buf, &mut html);
            html.push_str(&format!("<h3>{}</h3>\n", render_tex_inline(&h)));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("\\item") {
            flush_para(&mut para_buf, &mut html);
            html.push_str(&format!(
                "<ul><li>{}</li></ul>\n",
                render_tex_inline(rest.trim())
            ));
            continue;
        }
        if !para_buf.is_empty() {
            para_buf.push(' ');
        }
        para_buf.push_str(trimmed);
    }
    flush_para(&mut para_buf, &mut html);

    build_book_from_html(&title, creators, &html)
}

fn extract_tex_braced(s: &str, cmd: &str) -> Option<String> {
    let idx = s.find(cmd)?;
    let rest = &s[idx + cmd.len()..];
    let mut depth = 1usize;
    for (byte_i, ch) in rest.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[..byte_i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn clean_tex_inline(s: &str) -> String {
    s.replace("\\&", "&")
        .replace("\\%", "%")
        .replace("\\_", "_")
        .replace("\\#", "#")
        .replace("\\$", "$")
        .trim()
        .to_string()
}

fn render_tex_inline(s: &str) -> String {
    let mut cur = s.to_string();
    for (cmd, open, close) in [
        ("\\textbf{", "<strong>", "</strong>"),
        ("\\emph{", "<em>", "</em>"),
        ("\\textit{", "<em>", "</em>"),
        ("\\texttt{", "<code>", "</code>"),
    ] {
        while let Some(idx) = cur.find(cmd) {
            let after = &cur[idx + cmd.len()..];
            let mut depth = 1usize;
            let mut close_pos = None;
            for (bi, ch) in after.char_indices() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        close_pos = Some(bi);
                        break;
                    }
                }
            }
            let Some(cp) = close_pos else {
                break;
            };
            let inner = clean_tex_inline(&after[..cp]);
            let replacement = format!("{open}{}{close}", xml_escape(&inner));
            cur.replace_range(idx..idx + cmd.len() + cp + 1, &replacement);
        }
    }
    clean_tex_inline(&cur)
}

/// Export any `Book` to a clean LaTeX (`.tex`) document.
pub fn export_tex_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
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

    let mut out = String::with_capacity(32768);
    out.push_str("\\documentclass[11pt]{book}\n");
    out.push_str("\\usepackage[utf8]{inputenc}\n");
    out.push_str("\\usepackage[T1]{fontenc}\n");
    out.push_str(&format!("\\title{{{}}}\n", tex_escape(title)));
    out.push_str(&format!("\\author{{{}}}\n", tex_escape(author)));
    out.push_str("\\begin{document}\n\\maketitle\n\n");

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_tex(&dom.root_nodes, &mut out);
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                out.push_str(&tex_escape(para));
                out.push_str("\n\n");
            }
        }
    }

    out.push_str("\\end{document}\n");
    Ok(out.into_bytes())
}

fn tex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\textbackslash{}"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '#' => out.push_str("\\#"),
            '$' => out.push_str("\\$"),
            '%' => out.push_str("\\%"),
            '&' => out.push_str("\\&"),
            '_' => out.push_str("\\_"),
            '^' => out.push_str("\\textasciicircum{}"),
            '~' => out.push_str("\\textasciitilde{}"),
            c => out.push(c),
        }
    }
    out
}

fn dom_to_tex(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        let DomNode::Element {
            tag_name, children, ..
        } = node
        else {
            continue;
        };
        let tag = tag_name.to_ascii_lowercase();
        if matches!(
            tag.as_str(),
            "head" | "script" | "style" | "noscript" | "title"
        ) {
            continue;
        }
        match tag.as_str() {
            "h1" => {
                let mut inner = String::new();
                render_tex_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("\\chapter{{{}}}\n\n", inner.trim()));
                }
            }
            "h2" => {
                let mut inner = String::new();
                render_tex_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("\\section{{{}}}\n\n", inner.trim()));
                }
            }
            "h3" | "h4" | "h5" | "h6" => {
                let mut inner = String::new();
                render_tex_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("\\subsection{{{}}}\n\n", inner.trim()));
                }
            }
            "p" | "dt" | "dd" | "figcaption" => {
                let mut inner = String::new();
                render_tex_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(inner.trim());
                    out.push_str("\n\n");
                }
            }
            "blockquote" => {
                let mut inner = String::new();
                render_tex_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!(
                        "\\begin{{quote}}\n{}\n\\end{{quote}}\n\n",
                        inner.trim()
                    ));
                }
            }
            "ul" | "ol" => {
                let env = if tag == "ol" { "enumerate" } else { "itemize" };
                let mut inner = String::new();
                dom_to_tex(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("\\begin{{{env}}}\n{inner}\\end{{{env}}}\n\n"));
                }
            }
            "li" => {
                let mut inner = String::new();
                render_tex_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("  \\item {}\n", inner.trim()));
                }
            }
            "pre" => {
                let mut t = String::new();
                collect_text(children, &mut t);
                if !t.trim().is_empty() {
                    out.push_str(&format!(
                        "\\begin{{verbatim}}\n{}\n\\end{{verbatim}}\n\n",
                        t.trim_end()
                    ));
                }
            }
            _ => {
                if has_block_child(children) {
                    dom_to_tex(children, out);
                } else {
                    let mut inner = String::new();
                    render_tex_runs(children, &mut inner);
                    if !inner.trim().is_empty() {
                        out.push_str(inner.trim());
                        out.push_str("\n\n");
                    }
                }
            }
        }
    }
}

fn render_tex_runs(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let norm = collapse_ws(&decode_html_entities(t));
                if !norm.is_empty() {
                    out.push_str(&tex_escape(&norm));
                }
            }
            DomNode::Element {
                tag_name, children, ..
            } => {
                let tag = tag_name.to_ascii_lowercase();
                match tag.as_str() {
                    "strong" | "b" => {
                        let mut inner = String::new();
                        render_tex_runs(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("\\textbf{{{}}}", inner.trim()));
                        }
                    }
                    "em" | "i" => {
                        let mut inner = String::new();
                        render_tex_runs(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("\\emph{{{}}}", inner.trim()));
                        }
                    }
                    "code" => {
                        let mut t = String::new();
                        collect_text(children, &mut t);
                        if !t.trim().is_empty() {
                            out.push_str(&format!("\\texttt{{{}}}", tex_escape(t.trim())));
                        }
                    }
                    _ => render_tex_runs(children, out),
                }
            }
        }
    }
}

// ============================================================================
// 3. PowerPoint OOXML (.pptx) Input & Output
// ============================================================================

/// Parse a PowerPoint OOXML (`.pptx`) archive into a `Book` (one section per slide).
pub fn parse_pptx(archive: &EpubArchive, title_fallback: &str) -> Result<Book, EbookError> {
    let mut slide_paths: Vec<String> = archive
        .list_files()
        .into_iter()
        .filter(|f| {
            let l = f.to_ascii_lowercase();
            l.starts_with("ppt/slides/slide") && l.ends_with(".xml")
        })
        .collect();
    if slide_paths.is_empty() {
        return Err(EbookError::InvalidFormat(
            "PPTX archive contains no slides".to_string(),
        ));
    }
    slide_paths.sort_by_key(|p| {
        p.rsplit('/')
            .next()
            .unwrap_or("")
            .trim_start_matches("slide")
            .trim_end_matches(".xml")
            .parse::<usize>()
            .unwrap_or(usize::MAX)
    });

    let mut title = title_fallback.to_string();
    let mut creators = Vec::new();
    if let Ok(core_xml) = archive.read_string("docProps/core.xml")
        && let Ok(doc) = roxmltree::Document::parse(&core_xml)
    {
        for node in doc.descendants().filter(|n| n.is_element()) {
            let name = node.tag_name().name();
            if let Some(t) = node.text().map(str::trim).filter(|s| !s.is_empty()) {
                if name.eq_ignore_ascii_case("title") {
                    title = t.to_string();
                } else if name.eq_ignore_ascii_case("creator") {
                    creators.push(t.to_string());
                }
            }
        }
    }

    let mut sections = Vec::new();
    let mut spine = Vec::new();
    let mut toc = Vec::new();
    for (idx, spath) in slide_paths.iter().enumerate() {
        let Ok(xml) = archive.read_string(spath) else {
            continue;
        };
        let mut slide_html = String::new();
        let mut first_line = String::new();
        if let Ok(doc) = roxmltree::Document::parse(&xml) {
            for p_node in doc
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "p")
            {
                let mut para = String::new();
                for t_node in p_node
                    .descendants()
                    .filter(|n| n.is_element() && n.tag_name().name() == "t")
                {
                    if let Some(txt) = t_node.text() {
                        para.push_str(txt);
                    }
                }
                let clean = para.trim();
                if !clean.is_empty() {
                    if first_line.is_empty() {
                        first_line = clean.to_string();
                        slide_html.push_str(&format!("<h2>{}</h2>\n", xml_escape(clean)));
                    } else {
                        slide_html.push_str(&format!("<p>{}</p>\n", xml_escape(clean)));
                    }
                }
            }
        }
        let raw_html = format!("<section class=\"pptx-slide\">\n{slide_html}</section>");
        let plain_text = extract_plain_text(&raw_html);
        let plain_text_lower = plain_text.to_lowercase();
        let char_count = plain_text.chars().count();
        let href = format!("slide_{}.xhtml", idx + 1);
        let idref = format!("slide_{}", idx + 1);
        let label = if first_line.is_empty() {
            format!("Slide {}", idx + 1)
        } else {
            first_line
        };

        spine.push(SpineItem {
            idref: idref.clone(),
            linear: true,
            properties: Vec::new(),
            index: idx,
            href: href.clone(),
            media_type: "application/xhtml+xml".to_string(),
        });
        toc.push(NavPoint {
            id: format!("nav_{}", idx + 1),
            label,
            href: href.clone(),
            full_path: href.clone(),
            subitems: Vec::new(),
        });
        sections.push(Section {
            index: idx,
            idref,
            href: href.clone(),
            full_path: href,
            raw_html: raw_html.clone(),
            processed_html: raw_html,
            plain_text,
            plain_text_lower,
            char_count,
            viewport_width: None,
            viewport_height: None,
        });
    }

    let mut metadata = Metadata {
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
    if metadata.title.trim().is_empty() {
        metadata.title = title_fallback.to_string();
    }

    let opf = OpfPackage {
        version: "3.0".to_string(),
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
        archive: archive.clone(),
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

/// Export any `Book` to a valid PowerPoint OOXML (`.pptx`) presentation (one slide per section/chapter).
pub fn export_pptx_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
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

    let mut slides: Vec<(String, Vec<String>)> = Vec::new();
    slides.push((title.to_string(), vec![format!("By {author}")]));

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let mut paras: Vec<String> = sec
            .plain_text
            .split('\n')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if paras.is_empty() {
            continue;
        }
        let heading = paras.remove(0);
        // Chunk long sections into readable slides (max ~8 paragraphs per slide)
        if paras.is_empty() {
            slides.push((heading, Vec::new()));
        } else {
            for (c_idx, chunk) in paras.chunks(8).enumerate() {
                let stitle = if c_idx == 0 {
                    heading.clone()
                } else {
                    format!("{heading} ({})", c_idx + 1)
                };
                slides.push((stitle, chunk.to_vec()));
            }
        }
    }

    let mut content_types = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\n\
  <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\n\
  <Default Extension=\"xml\" ContentType=\"application/xml\"/>\n\
  <Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>\n\
  <Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\n",
    );
    let mut pres_rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n",
    );
    let mut sld_id_lst = String::new();

    for i in 1..=slides.len() {
        content_types.push_str(&format!(
            "  <Override PartName=\"/ppt/slides/slide{i}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>\n"
        ));
        pres_rels.push_str(&format!(
            "  <Relationship Id=\"rId{i}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{i}.xml\"/>\n"
        ));
        sld_id_lst.push_str(&format!(
            "    <p:sldId id=\"{}\" r:id=\"rId{i}\"/>\n",
            255 + i
        ));
    }
    content_types.push_str("</Types>\n");
    pres_rels.push_str("</Relationships>\n");

    let root_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
  <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/>\n\
  <Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/>\n\
</Relationships>\n";

    let presentation_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<p:presentation xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\n\
  <p:sldIdLst>\n{sld_id_lst}  </p:sldIdLst>\n\
  <p:sldSz cx=\"9144000\" cy=\"6858000\" type=\"screen4x3\"/>\n\
  <p:notesSz cx=\"6858000\" cy=\"9144000\"/>\n\
</p:presentation>\n"
    );

    let core_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
  <dc:title>{}</dc:title>\n\
  <dc:creator>{}</dc:creator>\n\
</cp:coreProperties>\n",
        xml_escape(title),
        xml_escape(author)
    );

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (path, data) in [
            ("[Content_Types].xml", content_types.as_bytes()),
            ("_rels/.rels", root_rels.as_bytes()),
            ("docProps/core.xml", core_xml.as_bytes()),
            ("ppt/presentation.xml", presentation_xml.as_bytes()),
            ("ppt/_rels/presentation.xml.rels", pres_rels.as_bytes()),
        ] {
            zip.start_file(path, deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(data)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }
        for (i, (stitle, body_lines)) in slides.iter().enumerate() {
            let mut tx_body = format!(
                "<a:p><a:r><a:rPr lang=\"en-US\" sz=\"2400\" b=\"1\"/><a:t>{}</a:t></a:r></a:p>",
                xml_escape(stitle)
            );
            for line in body_lines {
                tx_body.push_str(&format!(
                    "<a:p><a:r><a:rPr lang=\"en-US\" sz=\"1400\"/><a:t>{}</a:t></a:r></a:p>",
                    xml_escape(line)
                ));
            }
            let slide_xml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<p:sld xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" \
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">\n\
  <p:cSld><p:spTree>\n\
    <p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\n\
    <p:grpSpPr/>\n\
    <p:sp>\n\
      <p:nvSpPr><p:cNvPr id=\"2\" name=\"TextBox\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr>\n\
      <p:spPr><a:xfrm><a:off x=\"457200\" y=\"457200\"/><a:ext cx=\"8229600\" cy=\"5943600\"/></a:xfrm></p:spPr>\n\
      <p:txBody><a:bodyPr wrap=\"square\"/><a:lstStyle/>{tx_body}</p:txBody>\n\
    </p:sp>\n\
  </p:spTree></p:cSld>\n\
</p:sld>\n"
            );
            zip.start_file(format!("ppt/slides/slide{}.xml", i + 1), deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(slide_xml.as_bytes())
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }
        zip.finish()
            .map_err(|e| EbookError::Io(format!("PPTX finalize error: {e}")))?;
    }
    Ok(buf)
}

// ============================================================================
// Helpers
// ============================================================================

fn build_book_from_html(
    title: &str,
    creators: Vec<String>,
    body_html: &str,
) -> Result<Book, EbookError> {
    let raw_html = format!("<div class=\"tex-content\">\n{body_html}</div>");
    let plain_text = extract_plain_text(&raw_html);
    let plain_text_lower = plain_text.to_lowercase();
    let char_count = plain_text.chars().count();
    let section = Section {
        index: 0,
        idref: "sec_0".to_string(),
        href: "section_0.html".to_string(),
        full_path: "section_0.html".to_string(),
        raw_html: raw_html.clone(),
        processed_html: raw_html,
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
        href: "section_0.html".to_string(),
        media_type: "application/xhtml+xml".to_string(),
    }];
    let toc = vec![NavPoint {
        id: "nav_1".to_string(),
        label: title.to_string(),
        href: "section_0.html".to_string(),
        full_path: "section_0.html".to_string(),
        subitems: Vec::new(),
    }];
    let metadata = Metadata {
        title: title.to_string(),
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
        version: "3.0".to_string(),
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
        archive: EpubArchive::empty(),
        opf,
        layout: RenditionLayout::default(),
        toc,
        landmarks: Vec::new(),
        page_list: Vec::new(),
        sections: vec![section],
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

fn collect_text(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        match node {
            DomNode::Text(t) => out.push_str(&decode_html_entities(t)),
            DomNode::Element { children, .. } => collect_text(children, out),
            DomNode::Comment(_) => {}
        }
    }
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for ch in s.chars() {
        if ch.is_ascii_whitespace() {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(ch);
            prev_ws = false;
        }
    }
    out
}

fn has_block_child(nodes: &[DomNode]) -> bool {
    nodes.iter().any(|c| {
        matches!(
            c,
            DomNode::Element { tag_name, .. }
            if matches!(
                tag_name.to_ascii_lowercase().as_str(),
                "p" | "div" | "section" | "article" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                    | "ul" | "ol" | "table" | "blockquote" | "pre" | "body" | "html"
            )
        )
    })
}
