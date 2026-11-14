use crate::book::Book;
use crate::error::EbookError;

#[cfg(feature = "pdf")]
use crate::{
    archive::EpubArchive,
    deobfuscate::FontDeobfuscator,
    layout::RenditionLayout,
    metadata::{Metadata, PageProgressionDirection, SpineItem},
    nav::NavPoint,
    opf::OpfPackage,
    section::{Section, extract_plain_text},
};
#[cfg(feature = "pdf")]
use ahash::AHashMap;

/// PDF Document Parser using pre-extracted text / markdown via `unpdf`.
pub struct PdfBook;

impl PdfBook {
    /// Parse raw PDF byte data into a `Book` struct.
    #[cfg(feature = "pdf")]
    pub fn parse(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
        let mut doc = unpdf::parse_bytes(bytes).map_err(|e| {
            EbookError::InvalidFormat(format!("Failed to parse PDF document with unpdf: {}", e))
        })?;

        if doc.page_count() == 0
            && let Some(patched) = inline_indirect_page_tree_kids(bytes)
            && let Ok(reparsed) = unpdf::parse_bytes(&patched)
        {
            doc = reparsed;
        }

        let page_count = doc.page_count() as usize;
        if page_count == 0 {
            return Err(EbookError::InvalidFormat(
                "PDF document contains 0 pages".to_string(),
            ));
        }

        let capacity = page_count.min(1024);
        let mut sections = Vec::with_capacity(capacity);
        let mut spine = Vec::with_capacity(capacity);
        let mut toc = Vec::with_capacity(capacity);

        let render_options = unpdf::RenderOptions::default();

        for (page_idx, page) in doc.pages.iter().enumerate() {
            let page_options = render_options
                .clone()
                .with_pages(unpdf::PageSelection::Pages(vec![page.number]));

            let raw_md = unpdf::render::to_markdown(&doc, &page_options)
                .unwrap_or_else(|_| format!("<p>Page {}</p>", page.number));

            let markdown_content = reflow_two_column_markdown(&raw_md);
            let raw_html = markdown_to_html(&markdown_content, page_idx + 1);
            let idref = format!("page_{}", page_idx + 1);
            let href = format!("page_{}.html", page_idx + 1);
            let plain_text = extract_plain_text(&raw_html);
            let plain_text_lower = plain_text.to_lowercase();
            let char_count = plain_text.chars().count();

            sections.push(Section {
                index: page_idx,
                idref: idref.clone(),
                href: href.clone(),
                full_path: href.clone(),
                raw_html: raw_html.clone(),
                processed_html: raw_html,
                plain_text,
                plain_text_lower,
                char_count,
                viewport_width: if page.width > 0.0 {
                    Some(page.width as f64)
                } else {
                    None
                },
                viewport_height: if page.height > 0.0 {
                    Some(page.height as f64)
                } else {
                    None
                },
            });

            spine.push(SpineItem {
                index: page_idx,
                idref: idref.clone(),
                href: href.clone(),
                linear: true,
                media_type: "application/xhtml+xml".to_string(),
                properties: Vec::new(),
            });

            toc.push(NavPoint {
                id: format!("toc_{}", page_idx + 1),
                label: format!("Page {}", page_idx + 1),
                href: href.clone(),
                full_path: href,
                subitems: Vec::new(),
            });
        }

        let title = doc
            .metadata
            .title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| title_fallback.to_string());

        let creators = doc
            .metadata
            .author
            .filter(|a| !a.trim().is_empty())
            .map(|a| vec![a])
            .unwrap_or_else(|| vec!["PDF Document".to_string()]);

        let subjects = doc
            .metadata
            .keywords
            .filter(|k| !k.trim().is_empty())
            .map(|k| {
                k.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| vec!["PDF".to_string()]);

        let description = doc
            .metadata
            .subject
            .filter(|s| !s.trim().is_empty())
            .or_else(|| Some(format!("PDF Document ({} pages)", page_count)));

        let metadata = Metadata {
            title,
            creators,
            publishers: Vec::new(),
            languages: vec!["en".to_string()],
            rights: None,
            description,
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects,
            cover_id: None,
            cover_href: None,
            direction: PageProgressionDirection::Ltr,
            meta_properties: AHashMap::new(),
            accessibility: Default::default(),
        };

        let opf = OpfPackage {
            version: "3.0".to_string(),
            opf_path: "content.opf".to_string(),
            opf_dir: "".to_string(),
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

    #[cfg(not(feature = "pdf"))]
    pub fn parse(_bytes: &[u8], _title_fallback: &str) -> Result<Book, EbookError> {
        Err(EbookError::InvalidFormat(
            "PDF support requires the 'pdf' feature flag to be enabled in Cargo.toml".to_string(),
        ))
    }
}

/// Convert Markdown content to structured HTML section for PDF pages.
#[cfg(feature = "pdf")]
fn markdown_to_html(md: &str, page_num: usize) -> String {
    let mut html = format!("<div class=\"pdf-page\" data-page=\"{}\">\n", page_num);
    fn escape_html(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(h1) = trimmed.strip_prefix("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", escape_html(h1.trim())));
        } else if let Some(h2) = trimmed.strip_prefix("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", escape_html(h2.trim())));
        } else if let Some(h3) = trimmed.strip_prefix("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", escape_html(h3.trim())));
        } else if let Some(bullet) = trimmed.strip_prefix("- ") {
            html.push_str(&format!("<li>{}</li>\n", escape_html(bullet.trim())));
        } else {
            html.push_str(&format!("<p>{}</p>\n", escape_html(trimmed)));
        }
    }
    html.push_str("</div>");
    html
}

/// Reflow two-column academic paper Markdown text into clean single-column reading order.
#[cfg(feature = "pdf")]
pub fn reflow_two_column_markdown(md: &str) -> String {
    let lines: Vec<&str> = md.lines().collect();
    if lines.len() < 2 {
        return md.to_string();
    }

    let mut result_lines = Vec::new();
    let mut left_column = Vec::new();
    let mut right_column = Vec::new();
    let mut in_two_column_zone = false;

    for line in lines {
        let trimmed = line.trim();
        let is_column_line = (trimmed.contains("   |   ")
            || (trimmed.contains("      ")
                && !trimmed.starts_with("```")
                && !line.starts_with("    ")))
            && !trimmed.starts_with('#')
            && !trimmed.is_empty();

        if is_column_line {
            let parts: Vec<&str> = if trimmed.contains("   |   ") {
                trimmed.split("   |   ").collect()
            } else {
                trimmed.splitn(2, "      ").collect()
            };
            if parts.len() == 2 {
                let left = parts[0].trim();
                let right = parts[1].trim();
                if left.len() >= 5 && right.len() >= 5 {
                    in_two_column_zone = true;
                    left_column.push(left.to_string());
                    right_column.push(right.to_string());
                    continue;
                }
            }
        }

        if in_two_column_zone && (trimmed.starts_with('#') || trimmed.is_empty()) {
            if !left_column.is_empty() {
                result_lines.append(&mut left_column);
            }
            if !right_column.is_empty() {
                result_lines.append(&mut right_column);
            }
            in_two_column_zone = false;
        }

        result_lines.push(line.to_string());
    }

    if !left_column.is_empty() {
        result_lines.extend(left_column);
    }
    if !right_column.is_empty() {
        result_lines.extend(right_column);
    }

    result_lines.join("\n")
}

/// If a PDF's `/Pages` tree uses indirect `/Kids N G R` references (which `unpdf` 0.51
/// does not dereference in `scan_page_tree`), append a standard PDF incremental update
/// that inlines the resolved `/Kids [...]` array on affected `/Pages` objects.
#[cfg(feature = "pdf")]
fn inline_indirect_page_tree_kids(bytes: &[u8]) -> Option<Vec<u8>> {
    use std::collections::{BTreeMap, HashSet};
    use std::io::Write;
    use unpdf::parser::raw::{PdfDict, PdfObject, RawDocument, tokenizer::dict_get, xref};

    let raw_doc = RawDocument::load(bytes).ok()?;
    let old_startxref = xref::find_startxref(bytes).ok()?;
    let catalog = raw_doc.catalog().ok()?;
    let root_pages_id = dict_get(catalog, b"Pages")?.as_reference()?;

    let mut updated_pages: BTreeMap<(u32, u16), PdfDict> = BTreeMap::new();
    let mut visited: HashSet<(u32, u16)> = HashSet::new();
    let mut stack = vec![root_pages_id];

    while let Some(node_id) = stack.pop() {
        if !visited.insert(node_id) {
            continue;
        }
        let Ok(dict) = raw_doc.get_dict(node_id) else {
            continue;
        };
        let node_type = dict_get(dict, b"Type").and_then(|o| o.as_name());
        if matches!(node_type, Some(b"Pages") | None) {
            let Some(kids_entry) = dict_get(dict, b"Kids") else {
                continue;
            };
            let resolved_kids = raw_doc.resolve(kids_entry);
            let Some(kids_arr) = resolved_kids.as_array() else {
                continue;
            };
            if matches!(kids_entry, PdfObject::Reference(..)) {
                let mut new_dict = dict.clone();
                new_dict.insert(b"Kids".to_vec(), PdfObject::Array(kids_arr.to_vec()));
                updated_pages.insert(node_id, new_dict);
            }
            for kid in kids_arr.iter().rev() {
                if let Some(kid_id) = kid
                    .as_reference()
                    .or_else(|| raw_doc.resolve(kid).as_reference())
                {
                    stack.push(kid_id);
                }
            }
        }
    }

    if updated_pages.is_empty() {
        return None;
    }

    let mut out = Vec::with_capacity(bytes.len() + 4096);
    out.extend_from_slice(bytes);
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }

    let mut offsets: Vec<((u32, u16), usize)> = Vec::with_capacity(updated_pages.len());
    for ((obj_num, gen_num), dict) in &updated_pages {
        let offset = out.len();
        offsets.push(((*obj_num, *gen_num), offset));
        let _ = writeln!(out, "{obj_num} {gen_num} obj");
        write_pdf_object(&PdfObject::Dict(dict.clone()), &mut out);
        out.extend_from_slice(b"\nendobj\n");
    }

    let new_xref_offset = out.len();
    out.extend_from_slice(b"xref\n");
    for ((obj_num, gen_num), offset) in &offsets {
        let _ = writeln!(out, "{obj_num} 1");
        let _ = writeln!(out, "{offset:010} {gen_num:05} n ");
    }

    let mut trailer = PdfDict::new();
    for key in [
        b"Size".as_slice(),
        b"Root".as_slice(),
        b"Info".as_slice(),
        b"ID".as_slice(),
        b"Encrypt".as_slice(),
    ] {
        if let Some(val) = dict_get(raw_doc.trailer(), key) {
            trailer.insert(key.to_vec(), val.clone());
        }
    }
    trailer.insert(b"Prev".to_vec(), PdfObject::Integer(old_startxref as i64));

    out.extend_from_slice(b"trailer\n");
    write_pdf_object(&PdfObject::Dict(trailer), &mut out);
    let _ = write!(out, "\nstartxref\n{new_xref_offset}\n%%EOF\n");

    Some(out)
}

#[cfg(feature = "pdf")]
fn write_pdf_name(name: &[u8], out: &mut Vec<u8>) {
    use std::io::Write;
    out.push(b'/');
    for &b in name {
        if b <= b' '
            || b > b'~'
            || matches!(
                b,
                b'#' | b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
            )
        {
            let _ = write!(out, "#{b:02X}");
        } else {
            out.push(b);
        }
    }
}

#[cfg(feature = "pdf")]
fn write_pdf_object(obj: &unpdf::parser::raw::PdfObject, out: &mut Vec<u8>) {
    use std::io::Write;
    use unpdf::parser::raw::PdfObject;
    match obj {
        PdfObject::Null => out.extend_from_slice(b"null"),
        PdfObject::Bool(true) => out.extend_from_slice(b"true"),
        PdfObject::Bool(false) => out.extend_from_slice(b"false"),
        PdfObject::Integer(i) => {
            let _ = write!(out, "{i}");
        }
        PdfObject::Real(r) => {
            let _ = write!(out, "{r}");
        }
        PdfObject::Name(n) => write_pdf_name(n, out),
        PdfObject::Str(s) => {
            out.push(b'<');
            for &b in s {
                let _ = write!(out, "{b:02X}");
            }
            out.push(b'>');
        }
        PdfObject::Array(items) => {
            out.push(b'[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(b' ');
                }
                write_pdf_object(item, out);
            }
            out.push(b']');
        }
        PdfObject::Dict(dict) => {
            out.extend_from_slice(b"<<");
            for (k, v) in dict {
                write_pdf_name(k, out);
                out.push(b' ');
                write_pdf_object(v, out);
                out.push(b' ');
            }
            out.extend_from_slice(b">>");
        }
        PdfObject::Stream(stream) => {
            write_pdf_object(&PdfObject::Dict(stream.dict.clone()), out);
            out.extend_from_slice(b"\nstream\n");
            out.extend_from_slice(&stream.raw_data);
            out.extend_from_slice(b"\nendstream");
        }
        PdfObject::Reference(n, g) => {
            let _ = write!(out, "{n} {g} R");
        }
    }
}

#[cfg(all(test, feature = "pdf"))]
mod tests {
    use super::*;

    #[test]
    fn test_two_column_pdf_reflow() {
        let raw_two_column = "# Academic Paper Title\n\nAbstract of the research paper.\n\nLeft Column Line 1   |   Right Column Line 1\nLeft Column Line 2   |   Right Column Line 2\n\n## Section 2";
        let reflowed = reflow_two_column_markdown(raw_two_column);
        assert!(reflowed.contains("Left Column Line 1\nLeft Column Line 2"));
        assert!(reflowed.contains("Right Column Line 1\nRight Column Line 2"));
    }

    #[test]
    fn test_indirect_kids_array_in_pages_node() {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        let objects: [&[u8]; 4] = [
            b"<</Type/Catalog/Pages 2 0 R>>",
            b"<</Type/Pages/Count 1/Kids 3 0 R>>",
            b"[4 0 R]",
            b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>",
        ];
        for (idx, body) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n", idx + 1).as_bytes());
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(b"\nendobj\n");
        }
        let xref_offset = pdf.len();
        pdf.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
        );
        for off in offsets {
            pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{xref_offset}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );

        let book = PdfBook::parse(&pdf, "Indirect Kids PDF").expect("Should parse indirect /Kids");
        assert_eq!(book.sections().len(), 1);
    }
}
