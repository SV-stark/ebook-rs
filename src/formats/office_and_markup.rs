//! Pure-Rust Office, Legacy Binary, and Structured Markup Codecs (Input & Output):
//! - `docx` (OOXML WordprocessingML Output; Input handled by `DocxBook`)
//! - `odt` (OpenDocument Text Output; Input handled by `OdtBook`)
//! - `rtf` (Rich Text Format Output; Input handled by `RtfBook`)
//! - `pmlz` (Palm Markup Language ZIP Input & Output)
//! - `rb` (Rocket eBook binary Input & Output)
//! - `typ` / `typst` (Typst markup Input & Output)
//! - `adoc` (AsciiDoc markup Input & Output)
//! - `org` (Emacs Org-mode markup Input & Output)
//! - `rst` (reStructuredText markup Input & Output)

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
use crate::opf::OpfPackage;
use crate::section::{Section, extract_plain_text};
use ahash::AHashMap;
use base64::Engine as _;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{Cursor, Read as _, Write as _};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const RB_MAGIC: &[u8; 14] = b"\xb0\x0c\xb0\x0c\x02\x00NUVO\x00\x00\x00\x00";
const MAX_RB_DECOMPRESSED_BYTES: usize = 64 * 1024 * 1024;

// ============================================================================
// 1. DOCX (Microsoft Word OOXML) Output
// ============================================================================

/// Export any `Book` to a valid Microsoft Word OOXML (`.docx`) archive with metadata,
/// headings, bold/italic inline runs, tables, and embedded `<w:drawing>` images.
pub fn export_docx_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
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

    let mut media_files: Vec<(String, String, Vec<u8>)> = Vec::new(); // (rel_id, media_name, bytes)
    let mut media_by_key: AHashMap<String, (String, u32, u32)> = AHashMap::new(); // key -> (rId, w, h)
    let mut next_docpr_id: u32 = 1;

    let mut body_xml = String::with_capacity(65536);
    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let sec_dir = sec.full_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_docx(
                &dom.root_nodes,
                book,
                sec_dir,
                &mut media_files,
                &mut media_by_key,
                &mut next_docpr_id,
                &mut body_xml,
            );
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                body_xml.push_str(&format!(
                    "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>\n",
                    xml_escape(para)
                ));
            }
        }
    }
    if body_xml.trim().is_empty() {
        body_xml.push_str(&format!(
            "<w:p><w:r><w:t>{}</w:t></w:r></w:p>\n",
            xml_escape(title)
        ));
    }

    let document_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\n\
<w:body>\n{body_xml}<w:sectPr/>\n</w:body>\n</w:document>\n"
    );

    let mut rels_xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
  <Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\n",
    );
    for (rid, mname, _) in &media_files {
        rels_xml.push_str(&format!(
            "  <Relationship Id=\"{rid}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/{mname}\"/>\n"
        ));
    }
    rels_xml.push_str("</Relationships>\n");

    let content_types_xml = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\n\
  <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\n\
  <Default Extension=\"xml\" ContentType=\"application/xml\"/>\n\
  <Default Extension=\"png\" ContentType=\"image/png\"/>\n\
  <Default Extension=\"jpg\" ContentType=\"image/jpeg\"/>\n\
  <Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/>\n\
  <Default Extension=\"gif\" ContentType=\"image/gif\"/>\n\
  <Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\n\
  <Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\n\
  <Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\n\
</Types>\n";

    let root_rels_xml = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
  <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\n\
  <Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/>\n\
</Relationships>\n";

    let desc = meta.description.as_deref().unwrap_or("").trim();
    let core_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
  <dc:title>{}</dc:title>\n\
  <dc:creator>{}</dc:creator>\n\
  <dc:description>{}</dc:description>\n\
  <dc:language>{}</dc:language>\n\
</cp:coreProperties>\n",
        xml_escape(title),
        xml_escape(author),
        xml_escape(desc),
        xml_escape(lang)
    );

    let styles_xml = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\n\
  <w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>\n\
  <w:style w:type=\"paragraph\" w:styleId=\"Heading1\"><w:name w:val=\"heading 1\"/><w:rPr><w:b/><w:sz w:val=\"36\"/></w:rPr></w:style>\n\
  <w:style w:type=\"paragraph\" w:styleId=\"Heading2\"><w:name w:val=\"heading 2\"/><w:rPr><w:b/><w:sz w:val=\"30\"/></w:rPr></w:style>\n\
  <w:style w:type=\"paragraph\" w:styleId=\"Heading3\"><w:name w:val=\"heading 3\"/><w:rPr><w:b/><w:sz w:val=\"26\"/></w:rPr></w:style>\n\
  <w:style w:type=\"paragraph\" w:styleId=\"Quote\"><w:name w:val=\"Quote\"/><w:rPr><w:i/></w:rPr></w:style>\n\
</w:styles>\n";

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (path, content) in [
            ("[Content_Types].xml", content_types_xml.as_bytes()),
            ("_rels/.rels", root_rels_xml.as_bytes()),
            ("docProps/core.xml", core_xml.as_bytes()),
            ("word/styles.xml", styles_xml.as_bytes()),
            ("word/_rels/document.xml.rels", rels_xml.as_bytes()),
            ("word/document.xml", document_xml.as_bytes()),
        ] {
            zip.start_file(path, deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(content)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }
        for (_, mname, data) in media_files {
            zip.start_file(format!("word/media/{mname}"), deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(&data)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }
        zip.finish()
            .map_err(|e| EbookError::Io(format!("DOCX finalize error: {e}")))?;
    }
    Ok(buf)
}

#[allow(clippy::too_many_arguments)]
fn dom_to_docx(
    nodes: &[DomNode],
    book: &Book,
    sec_dir: &str,
    media_files: &mut Vec<(String, String, Vec<u8>)>,
    media_by_key: &mut AHashMap<String, (String, u32, u32)>,
    next_docpr_id: &mut u32,
    out: &mut String,
) {
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
        if matches!(
            tag.as_str(),
            "head" | "script" | "style" | "noscript" | "title"
        ) {
            continue;
        }
        match tag.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let style_id = match tag.as_str() {
                    "h1" => "Heading1",
                    "h2" => "Heading2",
                    _ => "Heading3",
                };
                let mut runs = String::new();
                render_docx_runs(
                    children,
                    book,
                    sec_dir,
                    media_files,
                    media_by_key,
                    next_docpr_id,
                    true,
                    false,
                    &mut runs,
                );
                if !runs.is_empty() {
                    out.push_str(&format!(
                        "<w:p><w:pPr><w:pStyle w:val=\"{style_id}\"/></w:pPr>{runs}</w:p>\n"
                    ));
                }
            }
            "p" | "li" | "dt" | "dd" | "figcaption" => {
                let mut runs = String::new();
                if tag == "li" {
                    runs.push_str("<w:r><w:t xml:space=\"preserve\">• </w:t></w:r>");
                }
                render_docx_runs(
                    children,
                    book,
                    sec_dir,
                    media_files,
                    media_by_key,
                    next_docpr_id,
                    false,
                    false,
                    &mut runs,
                );
                if !runs.is_empty() {
                    out.push_str(&format!("<w:p>{runs}</w:p>\n"));
                }
            }
            "img" | "image" => {
                let src = attributes
                    .get("src")
                    .or_else(|| attributes.get("xlink:href"))
                    .or_else(|| attributes.get("href"))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if let Some(drawing) =
                    make_docx_drawing(book, sec_dir, src, media_files, media_by_key, next_docpr_id)
                {
                    out.push_str(&format!("<w:p><w:r>{drawing}</w:r></w:p>\n"));
                }
            }
            "blockquote" => {
                let mut runs = String::new();
                render_docx_runs(
                    children,
                    book,
                    sec_dir,
                    media_files,
                    media_by_key,
                    next_docpr_id,
                    false,
                    true,
                    &mut runs,
                );
                if !runs.is_empty() {
                    out.push_str(&format!(
                        "<w:p><w:pPr><w:pStyle w:val=\"Quote\"/></w:pPr>{runs}</w:p>\n"
                    ));
                }
            }
            "table" => {
                let mut rows: Vec<Vec<String>> = Vec::new();
                collect_table_rows_text(children, &mut rows);
                if !rows.is_empty() {
                    out.push_str("<w:tbl>\n");
                    for row in rows {
                        out.push_str("  <w:tr>");
                        for cell in row {
                            out.push_str(&format!(
                                "<w:tc><w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p></w:tc>",
                                xml_escape(&cell)
                            ));
                        }
                        out.push_str("</w:tr>\n");
                    }
                    out.push_str("</w:tbl>\n");
                }
            }
            _ => {
                let has_block = has_block_descendant(children);
                if has_block {
                    dom_to_docx(
                        children,
                        book,
                        sec_dir,
                        media_files,
                        media_by_key,
                        next_docpr_id,
                        out,
                    );
                } else {
                    let mut runs = String::new();
                    render_docx_runs(
                        children,
                        book,
                        sec_dir,
                        media_files,
                        media_by_key,
                        next_docpr_id,
                        false,
                        false,
                        &mut runs,
                    );
                    if !runs.is_empty() {
                        out.push_str(&format!("<w:p>{runs}</w:p>\n"));
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_docx_runs(
    nodes: &[DomNode],
    book: &Book,
    sec_dir: &str,
    media_files: &mut Vec<(String, String, Vec<u8>)>,
    media_by_key: &mut AHashMap<String, (String, u32, u32)>,
    next_docpr_id: &mut u32,
    bold: bool,
    italic: bool,
    out: &mut String,
) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let decoded = decode_html_entities(t);
                let normalized = collapse_ws(&decoded);
                if !normalized.is_empty() {
                    let rpr = match (bold, italic) {
                        (true, true) => "<w:rPr><w:b/><w:i/></w:rPr>",
                        (true, false) => "<w:rPr><w:b/></w:rPr>",
                        (false, true) => "<w:rPr><w:i/></w:rPr>",
                        (false, false) => "",
                    };
                    out.push_str(&format!(
                        "<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r>",
                        xml_escape(&normalized)
                    ));
                }
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                if matches!(tag.as_str(), "script" | "style") {
                    continue;
                }
                if tag == "br" {
                    out.push_str("<w:r><w:br/></w:r>");
                    continue;
                }
                if tag == "img" || tag == "image" {
                    let src = attributes
                        .get("src")
                        .or_else(|| attributes.get("xlink:href"))
                        .or_else(|| attributes.get("href"))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    if let Some(drawing) = make_docx_drawing(
                        book,
                        sec_dir,
                        src,
                        media_files,
                        media_by_key,
                        next_docpr_id,
                    ) {
                        out.push_str(&format!("<w:r>{drawing}</w:r>"));
                    }
                    continue;
                }
                let next_b = bold || matches!(tag.as_str(), "strong" | "b");
                let next_i = italic || matches!(tag.as_str(), "em" | "i");
                render_docx_runs(
                    children,
                    book,
                    sec_dir,
                    media_files,
                    media_by_key,
                    next_docpr_id,
                    next_b,
                    next_i,
                    out,
                );
            }
        }
    }
}

fn make_docx_drawing(
    book: &Book,
    sec_dir: &str,
    src: &str,
    media_files: &mut Vec<(String, String, Vec<u8>)>,
    media_by_key: &mut AHashMap<String, (String, u32, u32)>,
    next_docpr_id: &mut u32,
) -> Option<String> {
    let (key, data) = resolve_book_image(book, sec_dir, src)?;
    let (rid, w, h) = if let Some(entry) = media_by_key.get(&key) {
        entry.clone()
    } else {
        let mime = guess_image_mime(&key, &data);
        let ext = ext_from_mime(mime);
        if ext == "svg" || ext == "webp" {
            return None;
        }
        let idx = media_files.len() + 1;
        let rid = format!("rIdImg{idx}");
        let mname = format!("image{idx}.{ext}");
        let (w, h) = parse_image_dims(&data).unwrap_or((300, 300));
        media_files.push((rid.clone(), mname, data));
        media_by_key.insert(key, (rid.clone(), w, h));
        (rid, w, h)
    };

    let docpr_id = *next_docpr_id;
    *next_docpr_id += 1;
    // Scale to max 5.5 inches width (5_029_200 EMUs); 9525 EMUs per pixel at 96 DPI
    let mut cx = u64::from(w.max(1)) * 9525;
    let mut cy = u64::from(h.max(1)) * 9525;
    let max_cx = 5_029_200u64;
    if cx > max_cx {
        cy = cy * max_cx / cx;
        cx = max_cx;
    }

    Some(format!(
        "<w:drawing><wp:inline><wp:extent cx=\"{cx}\" cy=\"{cy}\"/>\
<wp:docPr id=\"{docpr_id}\" name=\"Image {docpr_id}\"/>\
<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic><pic:nvPicPr><pic:cNvPr id=\"{docpr_id}\" name=\"Image {docpr_id}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic>\
</a:graphicData></a:graphic></wp:inline></w:drawing>"
    ))
}

// ============================================================================
// 2. ODT (OpenDocument Text) Output
// ============================================================================

/// Export any `Book` to a valid OpenDocument Text (`.odt`) archive.
pub fn export_odt_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
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
    let desc = meta.description.as_deref().unwrap_or("").trim();

    let mut pics: Vec<(String, String, Vec<u8>)> = Vec::new(); // (Pictures/img_N.ext, mime, bytes)
    let mut pic_by_key: AHashMap<String, String> = AHashMap::new();
    let mut body_xml = String::with_capacity(65536);

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let sec_dir = sec.full_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_odt(
                &dom.root_nodes,
                book,
                sec_dir,
                &mut pics,
                &mut pic_by_key,
                &mut body_xml,
            );
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                body_xml.push_str(&format!("<text:p>{}</text:p>\n", xml_escape(para)));
            }
        }
    }
    if body_xml.trim().is_empty() {
        body_xml.push_str(&format!("<text:p>{}</text:p>\n", xml_escape(title)));
    }

    let content_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" \
xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" \
xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" \
xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\" \
xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" \
xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\" \
xmlns:xlink=\"http://www.w3.org/1999/xlink\" office:version=\"1.2\">\n\
<office:automatic-styles>\n\
  <style:style style:name=\"T_Bold\" style:family=\"text\"><style:text-properties fo:font-weight=\"bold\"/></style:style>\n\
  <style:style style:name=\"T_Italic\" style:family=\"text\"><style:text-properties fo:font-style=\"italic\"/></style:style>\n\
  <style:style style:name=\"T_BoldItalic\" style:family=\"text\"><style:text-properties fo:font-weight=\"bold\" fo:font-style=\"italic\"/></style:style>\n\
</office:automatic-styles>\n\
<office:body><office:text>\n{body_xml}</office:text></office:body>\n</office:document-content>\n"
    );

    let meta_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<office:document-meta xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\" office:version=\"1.2\">\n\
<office:meta>\n\
  <dc:title>{}</dc:title>\n\
  <dc:creator>{}</dc:creator>\n\
  <dc:description>{}</dc:description>\n\
  <dc:language>{}</dc:language>\n\
</office:meta>\n</office:document-meta>\n",
        xml_escape(title),
        xml_escape(author),
        xml_escape(desc),
        xml_escape(lang)
    );

    let styles_xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<office:document-styles xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" office:version=\"1.2\"/>\n";

    let mut manifest_xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.2\">\n\
  <manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"application/vnd.oasis.opendocument.text\"/>\n\
  <manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/>\n\
  <manifest:file-entry manifest:full-path=\"styles.xml\" manifest:media-type=\"text/xml\"/>\n\
  <manifest:file-entry manifest:full-path=\"meta.xml\" manifest:media-type=\"text/xml\"/>\n",
    );
    for (rel, mime, _) in &pics {
        manifest_xml.push_str(&format!(
            "  <manifest:file-entry manifest:full-path=\"{}\" manifest:media-type=\"{}\"/>\n",
            xml_escape(rel),
            xml_escape(mime)
        ));
    }
    manifest_xml.push_str("</manifest:manifest>\n");

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("mimetype", stored)
            .map_err(|e| EbookError::Io(e.to_string()))?;
        zip.write_all(b"application/vnd.oasis.opendocument.text")
            .map_err(|e| EbookError::Io(e.to_string()))?;

        for (path, content) in [
            ("META-INF/manifest.xml", manifest_xml.as_bytes()),
            ("meta.xml", meta_xml.as_bytes()),
            ("styles.xml", styles_xml.as_bytes()),
            ("content.xml", content_xml.as_bytes()),
        ] {
            zip.start_file(path, deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(content)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }
        for (rel, _, data) in pics {
            zip.start_file(&rel, deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(&data)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }
        zip.finish()
            .map_err(|e| EbookError::Io(format!("ODT finalize error: {e}")))?;
    }
    Ok(buf)
}

fn dom_to_odt(
    nodes: &[DomNode],
    book: &Book,
    sec_dir: &str,
    pics: &mut Vec<(String, String, Vec<u8>)>,
    pic_by_key: &mut AHashMap<String, String>,
    out: &mut String,
) {
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
        if matches!(
            tag.as_str(),
            "head" | "script" | "style" | "noscript" | "title"
        ) {
            continue;
        }
        match tag.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let lvl = &tag[1..];
                let mut inner = String::new();
                render_odt_inline(
                    children, book, sec_dir, pics, pic_by_key, false, false, &mut inner,
                );
                if !inner.trim().is_empty() {
                    out.push_str(&format!(
                        "<text:h text:outline-level=\"{lvl}\">{inner}</text:h>\n"
                    ));
                }
            }
            "p" | "li" | "dt" | "dd" | "blockquote" | "figcaption" => {
                let mut inner = String::new();
                if tag == "li" {
                    inner.push_str("• ");
                }
                render_odt_inline(
                    children, book, sec_dir, pics, pic_by_key, false, false, &mut inner,
                );
                if !inner.trim().is_empty() {
                    out.push_str(&format!("<text:p>{inner}</text:p>\n"));
                }
            }
            "img" | "image" => {
                let src = attributes
                    .get("src")
                    .or_else(|| attributes.get("xlink:href"))
                    .or_else(|| attributes.get("href"))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if let Some(frame) = make_odt_frame(book, sec_dir, src, pics, pic_by_key) {
                    out.push_str(&format!("<text:p>{frame}</text:p>\n"));
                }
            }
            "table" => {
                let mut rows: Vec<Vec<String>> = Vec::new();
                collect_table_rows_text(children, &mut rows);
                if !rows.is_empty() {
                    out.push_str("<table:table>\n");
                    for row in rows {
                        out.push_str("  <table:table-row>");
                        for cell in row {
                            out.push_str(&format!(
                                "<table:table-cell><text:p>{}</text:p></table:table-cell>",
                                xml_escape(&cell)
                            ));
                        }
                        out.push_str("</table:table-row>\n");
                    }
                    out.push_str("</table:table>\n");
                }
            }
            _ => {
                if has_block_descendant(children) {
                    dom_to_odt(children, book, sec_dir, pics, pic_by_key, out);
                } else {
                    let mut inner = String::new();
                    render_odt_inline(
                        children, book, sec_dir, pics, pic_by_key, false, false, &mut inner,
                    );
                    if !inner.trim().is_empty() {
                        out.push_str(&format!("<text:p>{inner}</text:p>\n"));
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_odt_inline(
    nodes: &[DomNode],
    book: &Book,
    sec_dir: &str,
    pics: &mut Vec<(String, String, Vec<u8>)>,
    pic_by_key: &mut AHashMap<String, String>,
    bold: bool,
    italic: bool,
    out: &mut String,
) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let norm = collapse_ws(&decode_html_entities(t));
                if !norm.is_empty() {
                    let esc = xml_escape(&norm);
                    match (bold, italic) {
                        (true, true) => {
                            out.push_str(&format!(
                                "<text:span text:style-name=\"T_BoldItalic\">{esc}</text:span>"
                            ));
                        }
                        (true, false) => {
                            out.push_str(&format!(
                                "<text:span text:style-name=\"T_Bold\">{esc}</text:span>"
                            ));
                        }
                        (false, true) => {
                            out.push_str(&format!(
                                "<text:span text:style-name=\"T_Italic\">{esc}</text:span>"
                            ));
                        }
                        (false, false) => out.push_str(&esc),
                    }
                }
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                if matches!(tag.as_str(), "script" | "style") {
                    continue;
                }
                if tag == "br" {
                    out.push_str("<text:line-break/>");
                    continue;
                }
                if tag == "img" || tag == "image" {
                    let src = attributes
                        .get("src")
                        .or_else(|| attributes.get("xlink:href"))
                        .or_else(|| attributes.get("href"))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    if let Some(frame) = make_odt_frame(book, sec_dir, src, pics, pic_by_key) {
                        out.push_str(&frame);
                    }
                    continue;
                }
                let next_b = bold || matches!(tag.as_str(), "strong" | "b");
                let next_i = italic || matches!(tag.as_str(), "em" | "i");
                render_odt_inline(
                    children, book, sec_dir, pics, pic_by_key, next_b, next_i, out,
                );
            }
        }
    }
}

fn make_odt_frame(
    book: &Book,
    sec_dir: &str,
    src: &str,
    pics: &mut Vec<(String, String, Vec<u8>)>,
    pic_by_key: &mut AHashMap<String, String>,
) -> Option<String> {
    let (key, data) = resolve_book_image(book, sec_dir, src)?;
    let (w, h) = parse_image_dims(&data).unwrap_or((300, 300));
    let rel = if let Some(existing) = pic_by_key.get(&key) {
        existing.clone()
    } else {
        let mime = guess_image_mime(&key, &data);
        let ext = ext_from_mime(mime);
        let rel = format!("Pictures/img_{}.{ext}", pics.len());
        pic_by_key.insert(key, rel.clone());
        pics.push((rel.clone(), mime.to_string(), data));
        rel
    };
    let w_cm = (f64::from(w.max(1)) * 2.54 / 96.0).min(14.0);
    let h_cm = w_cm * f64::from(h.max(1)) / f64::from(w.max(1));
    Some(format!(
        "<draw:frame draw:name=\"{}\" text:anchor-type=\"as-char\" svg:width=\"{w_cm:.2}cm\" svg:height=\"{h_cm:.2}cm\">\
<draw:image xlink:href=\"{}\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"/></draw:frame>",
        xml_escape(&rel),
        xml_escape(&rel)
    ))
}

// ============================================================================
// 3. RTF (Rich Text Format) Output
// ============================================================================

/// Export any `Book` to valid Rich Text Format (`.rtf`) with Unicode escapes,
/// headings, bold/italic styling, tables, and embedded `\pict` PNG/JPEG images.
pub fn export_rtf_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
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

    let mut out = String::with_capacity(65536);
    out.push_str("{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1\n");
    out.push_str("{\\fonttbl{\\f0\\froman\\fcharset0 Georgia;}}\n");
    out.push_str(&format!(
        "{{\\info{{\\title {}}}{{\\author {}}}}}\n",
        rtf_escape(title),
        rtf_escape(author)
    ));
    out.push_str("\\f0\\fs24\n");

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let sec_dir = sec.full_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_rtf(&dom.root_nodes, book, sec_dir, &mut out);
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                out.push_str("\\pard ");
                out.push_str(&rtf_escape(para));
                out.push_str("\\par\n");
            }
        }
    }

    out.push_str("}\n");
    Ok(out.into_bytes())
}

fn rtf_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\n' => out.push_str("\\line "),
            '\r' => {}
            c if (c as u32) < 128 => out.push(c),
            c => {
                // UTF-16 code units as signed 16-bit \uN?
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{}?", *u as i16));
                }
            }
        }
    }
    out
}

fn dom_to_rtf(nodes: &[DomNode], book: &Book, sec_dir: &str, out: &mut String) {
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
        if matches!(
            tag.as_str(),
            "head" | "script" | "style" | "noscript" | "title"
        ) {
            continue;
        }
        match tag.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let fs = match tag.as_str() {
                    "h1" => 36,
                    "h2" => 30,
                    _ => 26,
                };
                let mut inner = String::new();
                render_rtf_inline(children, book, sec_dir, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("\\pard\\sa180{{\\b\\fs{fs} {inner}}}\\par\n"));
                }
            }
            "p" | "li" | "dt" | "dd" | "blockquote" | "figcaption" => {
                let mut inner = String::new();
                if tag == "li" {
                    inner.push_str("\\u8226? ");
                }
                render_rtf_inline(children, book, sec_dir, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("\\pard\\sa120 {inner}\\par\n"));
                }
            }
            "img" | "image" => {
                let src = attributes
                    .get("src")
                    .or_else(|| attributes.get("xlink:href"))
                    .or_else(|| attributes.get("href"))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if let Some(pict) = make_rtf_pict(book, sec_dir, src) {
                    out.push_str(&format!("\\pard {pict}\\par\n"));
                }
            }
            "table" => {
                let mut rows: Vec<Vec<String>> = Vec::new();
                collect_table_rows_text(children, &mut rows);
                for row in rows {
                    let cols = row.len().max(1);
                    out.push_str("\\trowd");
                    for c in 1..=cols {
                        out.push_str(&format!("\\cellx{}", c * 2800));
                    }
                    for cell in row {
                        out.push_str(&format!("\\intbl {}\\cell", rtf_escape(&cell)));
                    }
                    out.push_str("\\row\n");
                }
            }
            _ => {
                if has_block_descendant(children) {
                    dom_to_rtf(children, book, sec_dir, out);
                } else {
                    let mut inner = String::new();
                    render_rtf_inline(children, book, sec_dir, &mut inner);
                    if !inner.trim().is_empty() {
                        out.push_str(&format!("\\pard\\sa120 {inner}\\par\n"));
                    }
                }
            }
        }
    }
}

fn render_rtf_inline(nodes: &[DomNode], book: &Book, sec_dir: &str, out: &mut String) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let norm = collapse_ws(&decode_html_entities(t));
                if !norm.is_empty() {
                    out.push_str(&rtf_escape(&norm));
                }
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                if matches!(tag.as_str(), "script" | "style") {
                    continue;
                }
                match tag.as_str() {
                    "strong" | "b" => {
                        out.push_str("{\\b ");
                        render_rtf_inline(children, book, sec_dir, out);
                        out.push('}');
                    }
                    "em" | "i" => {
                        out.push_str("{\\i ");
                        render_rtf_inline(children, book, sec_dir, out);
                        out.push('}');
                    }
                    "br" => out.push_str("\\line "),
                    "img" | "image" => {
                        let src = attributes
                            .get("src")
                            .or_else(|| attributes.get("xlink:href"))
                            .or_else(|| attributes.get("href"))
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        if let Some(pict) = make_rtf_pict(book, sec_dir, src) {
                            out.push_str(&pict);
                        }
                    }
                    _ => render_rtf_inline(children, book, sec_dir, out),
                }
            }
        }
    }
}

fn make_rtf_pict(book: &Book, sec_dir: &str, src: &str) -> Option<String> {
    let (key, data) = resolve_book_image(book, sec_dir, src)?;
    let mime = guess_image_mime(&key, &data);
    let blip = match mime {
        "image/png" => "\\pngblip",
        "image/jpeg" => "\\jpegblip",
        _ => return None,
    };
    let (w, h) = parse_image_dims(&data).unwrap_or((300, 300));
    let w_twips = u64::from(w.max(1)) * 15;
    let h_twips = u64::from(h.max(1)) * 15;
    let mut hex = String::with_capacity(data.len() * 2);
    for b in &data {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    Some(format!(
        "{{\\pict{blip}\\picw{w}\\pich{h}\\picwgoal{w_twips}\\pichgoal{h_twips} {hex}}}"
    ))
}

// ============================================================================
// 4. PMLZ (Palm Markup Language ZIP) Input & Output
// ============================================================================

/// Parse a Calibre-compatible PMLZ archive containing `.pml` and optional `_img/` assets.
pub fn parse_pmlz_archive(archive: &EpubArchive, title_fallback: &str) -> Result<Book, EbookError> {
    let mut files = archive.list_files();
    files.sort();
    let pml_files: Vec<String> = files
        .iter()
        .filter(|f| f.to_ascii_lowercase().ends_with(".pml"))
        .cloned()
        .collect();
    if pml_files.is_empty() {
        return Err(EbookError::InvalidFormat(
            "No .pml file in archive".to_string(),
        ));
    }

    let mut img_by_name: AHashMap<String, String> = AHashMap::new();
    let mut manifest = AHashMap::new();
    for (idx, f) in files.iter().enumerate() {
        let l = f.to_ascii_lowercase();
        if let Some(mime) = mime_for_asset(&l) {
            let base = f.rsplit('/').next().unwrap_or(f).to_string();
            img_by_name.insert(base.to_ascii_lowercase(), f.clone());
            let id = format!("asset_{idx}");
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

    let mut combined_html = String::new();
    for pml_path in &pml_files {
        if let Ok(bytes) = archive.read_bytes(pml_path) {
            let pml_str = decode_bytes_with_encoding(&bytes, None);
            pml_to_html(&pml_str, &img_by_name, &mut combined_html);
        }
    }

    let plain_text = extract_plain_text(&combined_html);
    let plain_text_lower = plain_text.to_lowercase();
    let char_count = plain_text.chars().count();
    let raw_html = format!("<div class=\"pml-content\">\n{combined_html}</div>");

    let section = Section {
        index: 0,
        idref: "sec_0".to_string(),
        href: "index.xhtml".to_string(),
        full_path: "index.xhtml".to_string(),
        raw_html: raw_html.clone(),
        processed_html: raw_html,
        plain_text,
        plain_text_lower,
        char_count,
        viewport_width: None,
        viewport_height: None,
    };

    manifest.insert(
        "sec_0".to_string(),
        ManifestItem {
            id: "sec_0".to_string(),
            href: "index.xhtml".to_string(),
            full_path: "index.xhtml".to_string(),
            media_type: "application/xhtml+xml".to_string(),
            properties: Vec::new(),
            fallback: None,
            media_overlay: None,
        },
    );

    let spine = vec![SpineItem {
        idref: "sec_0".to_string(),
        linear: true,
        properties: Vec::new(),
        index: 0,
        href: "index.xhtml".to_string(),
        media_type: "application/xhtml+xml".to_string(),
    }];

    let toc = vec![NavPoint {
        id: "nav_1".to_string(),
        label: title_fallback.to_string(),
        href: "index.xhtml".to_string(),
        full_path: "index.xhtml".to_string(),
        subitems: Vec::new(),
    }];

    let opf = OpfPackage {
        version: "3.0".to_string(),
        opf_path: "content.opf".to_string(),
        opf_dir: String::new(),
        metadata: default_metadata(title_fallback),
        manifest,
        spine,
        guide: Vec::new(),
        toc_item_id: None,
        nav_item_id: None,
    };
    Ok(build_synthetic_book(
        archive.clone(),
        opf,
        toc,
        vec![section],
    ))
}

fn pml_to_html(pml: &str, img_by_name: &AHashMap<String, String>, out: &mut String) {
    let mut bold = false;
    let mut italic = false;
    let mut cur = String::new();

    let flush_para = |cur: &mut String, bold: &mut bool, italic: &mut bool, out: &mut String| {
        if *italic {
            cur.push_str("</em>");
            *italic = false;
        }
        if *bold {
            cur.push_str("</strong>");
            *bold = false;
        }
        if !cur.trim().is_empty() {
            out.push_str("<p>");
            out.push_str(cur.trim());
            out.push_str("</p>\n");
        }
        cur.clear();
    };

    for line in pml.lines() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0usize;
        if line.trim().is_empty() {
            flush_para(&mut cur, &mut bold, &mut italic, out);
            continue;
        }
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                let code = chars[i + 1];
                match code {
                    '\\' => {
                        cur.push('\\');
                        i += 2;
                    }
                    'b' | 'B' => {
                        if bold {
                            cur.push_str("</strong>");
                        } else {
                            cur.push_str("<strong>");
                        }
                        bold = !bold;
                        i += 2;
                    }
                    'i' => {
                        if italic {
                            cur.push_str("</em>");
                        } else {
                            cur.push_str("<em>");
                        }
                        italic = !italic;
                        i += 2;
                    }
                    'm' if i + 3 < chars.len() && chars[i + 2] == '=' && chars[i + 3] == '"' => {
                        let start = i + 4;
                        let mut end = start;
                        while end < chars.len() && chars[end] != '"' {
                            end += 1;
                        }
                        let name: String = chars[start..end].iter().collect();
                        let resolved = img_by_name
                            .get(&name.to_ascii_lowercase())
                            .cloned()
                            .unwrap_or_else(|| format!("index_img/{name}"));
                        cur.push_str(&format!(
                            "<img src=\"{}\" alt=\"\"/>",
                            xml_escape(&resolved)
                        ));
                        i = (end + 1).min(chars.len());
                    }
                    'a' if i + 4 < chars.len() => {
                        let digits: String = chars[i + 2..i + 5].iter().collect();
                        if let Ok(n) = digits.parse::<u32>()
                            && let Some(ch) = char::from_u32(n)
                        {
                            cur.push_str(&xml_escape(&ch.to_string()));
                            i += 5;
                            continue;
                        }
                        i += 2;
                    }
                    'u' if i + 5 < chars.len() => {
                        let hex: String = chars[i + 2..i + 6].iter().collect();
                        if let Ok(n) = u32::from_str_radix(&hex, 16)
                            && let Some(ch) = char::from_u32(n)
                        {
                            cur.push_str(&xml_escape(&ch.to_string()));
                            i += 6;
                            continue;
                        }
                        i += 2;
                    }
                    'X' | 'x' if i + 2 < chars.len() && chars[i + 2].is_ascii_digit() => {
                        i += 3;
                    }
                    'q' | 'Q' | 'T'
                        if i + 3 < chars.len() && chars[i + 2] == '=' && chars[i + 3] == '"' =>
                    {
                        let mut end = i + 4;
                        while end < chars.len() && chars[end] != '"' {
                            end += 1;
                        }
                        i = (end + 1).min(chars.len());
                    }
                    _ => {
                        i += 2;
                    }
                }
            } else {
                cur.push_str(&xml_escape(&chars[i].to_string()));
                i += 1;
            }
        }
        flush_para(&mut cur, &mut bold, &mut italic, out);
    }
    flush_para(&mut cur, &mut bold, &mut italic, out);
}

/// Export any `Book` to a Calibre-compatible PMLZ (`.pmlz`) archive (`index.pml` + `index_img/*`).
pub fn export_pmlz_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let mut pml = String::with_capacity(32768);
    let mut img_files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut img_by_key: AHashMap<String, String> = AHashMap::new();

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let sec_dir = sec.full_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_pml(
                &dom.root_nodes,
                book,
                sec_dir,
                &mut img_files,
                &mut img_by_key,
                &mut pml,
            );
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                pml.push_str(&pml_escape(para));
                pml.push_str("\n\n");
            }
        }
    }
    if pml.trim().is_empty() {
        pml.push_str(&pml_escape(book.metadata().title.trim()));
        pml.push('\n');
    }

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("index.pml", deflated)
            .map_err(|e| EbookError::Io(e.to_string()))?;
        zip.write_all(pml.as_bytes())
            .map_err(|e| EbookError::Io(e.to_string()))?;

        for (name, data) in img_files {
            zip.start_file(format!("index_img/{name}"), deflated)
                .map_err(|e| EbookError::Io(e.to_string()))?;
            zip.write_all(&data)
                .map_err(|e| EbookError::Io(e.to_string()))?;
        }
        zip.finish()
            .map_err(|e| EbookError::Io(format!("PMLZ finalize error: {e}")))?;
    }
    Ok(buf)
}

fn pml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 128 => out.push(c),
            c => {
                let code = c as u32;
                if code <= 0xFFFF {
                    out.push_str(&format!("\\u{code:04x}"));
                } else {
                    out.push('?');
                }
            }
        }
    }
    out
}

fn dom_to_pml(
    nodes: &[DomNode],
    book: &Book,
    sec_dir: &str,
    img_files: &mut Vec<(String, Vec<u8>)>,
    img_by_key: &mut AHashMap<String, String>,
    out: &mut String,
) {
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
        if matches!(
            tag.as_str(),
            "head" | "script" | "style" | "noscript" | "title"
        ) {
            continue;
        }
        match tag.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let mut inner = String::new();
                render_pml_inline(children, book, sec_dir, img_files, img_by_key, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("\\c\\B{}\\B\\c\n\n", inner.trim()));
                }
            }
            "p" | "li" | "blockquote" | "dt" | "dd" | "figcaption" => {
                let mut inner = String::new();
                render_pml_inline(children, book, sec_dir, img_files, img_by_key, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(inner.trim());
                    out.push_str("\n\n");
                }
            }
            "img" | "image" => {
                let src = attributes
                    .get("src")
                    .or_else(|| attributes.get("xlink:href"))
                    .or_else(|| attributes.get("href"))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if let Some(name) = register_pml_image(book, sec_dir, src, img_files, img_by_key) {
                    out.push_str(&format!("\\m=\"{name}\"\n\n"));
                }
            }
            _ => {
                if has_block_descendant(children) {
                    dom_to_pml(children, book, sec_dir, img_files, img_by_key, out);
                } else {
                    let mut inner = String::new();
                    render_pml_inline(children, book, sec_dir, img_files, img_by_key, &mut inner);
                    if !inner.trim().is_empty() {
                        out.push_str(inner.trim());
                        out.push_str("\n\n");
                    }
                }
            }
        }
    }
}

fn render_pml_inline(
    nodes: &[DomNode],
    book: &Book,
    sec_dir: &str,
    img_files: &mut Vec<(String, Vec<u8>)>,
    img_by_key: &mut AHashMap<String, String>,
    out: &mut String,
) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let norm = collapse_ws(&decode_html_entities(t));
                if !norm.is_empty() {
                    out.push_str(&pml_escape(&norm));
                }
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                if matches!(tag.as_str(), "script" | "style") {
                    continue;
                }
                match tag.as_str() {
                    "strong" | "b" => {
                        out.push_str("\\B");
                        render_pml_inline(children, book, sec_dir, img_files, img_by_key, out);
                        out.push_str("\\B");
                    }
                    "em" | "i" => {
                        out.push_str("\\i");
                        render_pml_inline(children, book, sec_dir, img_files, img_by_key, out);
                        out.push_str("\\i");
                    }
                    "img" | "image" => {
                        let src = attributes
                            .get("src")
                            .or_else(|| attributes.get("xlink:href"))
                            .or_else(|| attributes.get("href"))
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        if let Some(name) =
                            register_pml_image(book, sec_dir, src, img_files, img_by_key)
                        {
                            out.push_str(&format!("\\m=\"{name}\""));
                        }
                    }
                    _ => render_pml_inline(children, book, sec_dir, img_files, img_by_key, out),
                }
            }
        }
    }
}

fn register_pml_image(
    book: &Book,
    sec_dir: &str,
    src: &str,
    img_files: &mut Vec<(String, Vec<u8>)>,
    img_by_key: &mut AHashMap<String, String>,
) -> Option<String> {
    let (key, data) = resolve_book_image(book, sec_dir, src)?;
    if let Some(existing) = img_by_key.get(&key) {
        return Some(existing.clone());
    }
    let mime = guess_image_mime(&key, &data);
    let ext = ext_from_mime(mime);
    let name = format!("{}.{ext}", img_files.len() + 1);
    img_by_key.insert(key, name.clone());
    img_files.push((name.clone(), data));
    Some(name)
}

// ============================================================================
// 5. Rocket eBook (.rb) Binary Input & Output
// ============================================================================

/// Detect Rocket eBook (`.rb`) magic bytes (`\xb0\x0c\xb0\x0c\x02\x00NUVO\x00\x00\x00\x00`).
pub fn is_rb_bytes(bytes: &[u8]) -> bool {
    bytes.len() >= 32 && bytes.starts_with(RB_MAGIC)
}

/// Parse a Rocket eBook (`.rb`) binary container into a `Book`.
pub fn parse_rb(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
    if !is_rb_bytes(bytes) {
        return Err(EbookError::InvalidFormat(
            "Invalid Rocket eBook (.rb) header".to_string(),
        ));
    }
    let toc_off = u32::from_le_bytes([bytes[0x18], bytes[0x19], bytes[0x1A], bytes[0x1B]]) as usize;
    if toc_off + 4 > bytes.len() {
        return Err(EbookError::InvalidFormat(
            "Truncated RB TOC offset".to_string(),
        ));
    }
    let count = u32::from_le_bytes([
        bytes[toc_off],
        bytes[toc_off + 1],
        bytes[toc_off + 2],
        bytes[toc_off + 3],
    ]) as usize;
    let toc_end = toc_off
        .checked_add(4)
        .and_then(|v| v.checked_add(count.checked_mul(44)?))
        .ok_or_else(|| EbookError::InvalidFormat("RB TOC overflow".to_string()))?;
    if toc_end > bytes.len() {
        return Err(EbookError::InvalidFormat(
            "Truncated RB TOC table".to_string(),
        ));
    }

    let mut title = String::new();
    let mut creators = Vec::new();
    let mut html_docs: Vec<(String, String)> = Vec::new();
    let mut archive = EpubArchive::empty();
    let mut manifest = AHashMap::new();

    for i in 0..count {
        let base = toc_off + 4 + i * 44;
        let raw_name = &bytes[base..base + 32];
        let name_end = raw_name.iter().position(|&b| b == 0).unwrap_or(32);
        let name = String::from_utf8_lossy(&raw_name[..name_end]).to_string();
        let length = u32::from_le_bytes([
            bytes[base + 32],
            bytes[base + 33],
            bytes[base + 34],
            bytes[base + 35],
        ]) as usize;
        let offset = u32::from_le_bytes([
            bytes[base + 36],
            bytes[base + 37],
            bytes[base + 38],
            bytes[base + 39],
        ]) as usize;
        let flags = u32::from_le_bytes([
            bytes[base + 40],
            bytes[base + 41],
            bytes[base + 42],
            bytes[base + 43],
        ]);

        if offset
            .checked_add(length)
            .is_none_or(|end| end > bytes.len())
        {
            continue;
        }
        let payload = &bytes[offset..offset + length];

        if flags == 2 || name.eq_ignore_ascii_case("info.info") {
            let info_str = decode_bytes_with_encoding(payload, None);
            for line in info_str.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    let val = v.trim();
                    if k.trim().eq_ignore_ascii_case("TITLE") && !val.is_empty() {
                        title = val.to_string();
                    } else if k.trim().eq_ignore_ascii_case("AUTHOR") && !val.is_empty() {
                        creators.push(val.to_string());
                    }
                }
            }
        } else if flags == 8 {
            if let Ok(decompressed) = decompress_rb_chunks(payload) {
                let html = decode_bytes_with_encoding(&decompressed, None);
                html_docs.push((name, html));
            }
        } else if let Some(mime) = mime_for_asset(&name.to_ascii_lowercase()) {
            archive.insert(name.clone(), payload.to_vec());
            let id = format!("rb_asset_{i}");
            manifest.insert(
                id.clone(),
                ManifestItem {
                    id,
                    href: name.clone(),
                    full_path: name,
                    media_type: mime.to_string(),
                    properties: Vec::new(),
                    fallback: None,
                    media_overlay: None,
                },
            );
        }
    }

    if html_docs.is_empty() {
        return Err(EbookError::InvalidFormat(
            "RB container contains no HTML pages".to_string(),
        ));
    }
    if title.trim().is_empty() {
        title = title_fallback.to_string();
    }

    let mut sections = Vec::new();
    let mut spine = Vec::new();
    let mut toc = Vec::new();
    for (idx, (name, raw_html)) in html_docs.into_iter().enumerate() {
        let plain_text = extract_plain_text(&raw_html);
        let plain_text_lower = plain_text.to_lowercase();
        let char_count = plain_text.chars().count();
        let idref = format!("sec_{idx}");
        archive.insert(name.clone(), raw_html.as_bytes().to_vec());
        manifest.insert(
            idref.clone(),
            ManifestItem {
                id: idref.clone(),
                href: name.clone(),
                full_path: name.clone(),
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
            href: name.clone(),
            media_type: "application/xhtml+xml".to_string(),
        });
        toc.push(NavPoint {
            id: format!("nav_{idx}"),
            label: title.clone(),
            href: name.clone(),
            full_path: name.clone(),
            subitems: Vec::new(),
        });
        sections.push(Section {
            index: idx,
            idref,
            href: name.clone(),
            full_path: name,
            raw_html: raw_html.clone(),
            processed_html: raw_html,
            plain_text,
            plain_text_lower,
            char_count,
            viewport_width: None,
            viewport_height: None,
        });
    }

    let mut metadata = default_metadata(&title);
    metadata.creators = creators;
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
    Ok(build_synthetic_book(archive, opf, toc, sections))
}

fn decompress_rb_chunks(payload: &[u8]) -> Result<Vec<u8>, EbookError> {
    if payload.len() < 8 {
        return Err(EbookError::InvalidFormat(
            "Truncated RB compressed block".to_string(),
        ));
    }
    let n_chunks = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
    let _uncomp_sz = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]) as usize;
    let table_end = 8usize
        .checked_add(
            n_chunks
                .checked_mul(4)
                .ok_or_else(|| EbookError::InvalidFormat("RB chunk count overflow".to_string()))?,
        )
        .ok_or_else(|| EbookError::InvalidFormat("RB chunk table overflow".to_string()))?;
    if table_end > payload.len() {
        return Err(EbookError::InvalidFormat(
            "Truncated RB chunk size table".to_string(),
        ));
    }
    let mut chunk_sizes = Vec::with_capacity(n_chunks);
    for i in 0..n_chunks {
        let off = 8 + i * 4;
        chunk_sizes.push(u32::from_le_bytes([
            payload[off],
            payload[off + 1],
            payload[off + 2],
            payload[off + 3],
        ]) as usize);
    }
    let mut pos = table_end;
    let mut out = Vec::new();
    for sz in chunk_sizes {
        if pos.checked_add(sz).is_none_or(|end| end > payload.len()) {
            return Err(EbookError::InvalidFormat(
                "Truncated RB zlib chunk".to_string(),
            ));
        }
        let mut decoder = ZlibDecoder::new(&payload[pos..pos + sz]);
        let mut chunk = Vec::new();
        decoder
            .read_to_end(&mut chunk)
            .map_err(|e| EbookError::InvalidFormat(format!("RB zlib error: {e}")))?;
        if out.len() + chunk.len() > MAX_RB_DECOMPRESSED_BYTES {
            return Err(EbookError::InvalidFormat(
                "RB decompressed payload exceeds 64 MiB safety budget".to_string(),
            ));
        }
        out.extend_from_slice(&chunk);
        pos += sz;
    }
    Ok(out)
}

/// Export any `Book` to a valid Rocket eBook (`.rb`) binary container with chunked zlib HTML
/// and embedded images, interoperable with Calibre `ebook-convert`.
pub fn export_rb_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
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

    let mut img_entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut img_by_key: AHashMap<String, String> = AHashMap::new();
    let mut html_body = String::new();

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        let sec_dir = sec.full_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let body_inner =
            extract_body_inner_html(&sec.raw_html).unwrap_or_else(|| sec.raw_html.clone());
        let rewritten = rewrite_html_img_srcs(&body_inner, |raw_src| {
            let (key, data) = resolve_book_image(book, sec_dir, raw_src)?;
            if let Some(existing) = img_by_key.get(&key) {
                return Some(existing.clone());
            }
            let mime = guess_image_mime(&key, &data);
            let ext = ext_from_mime(mime);
            let name = format!("{}.{ext}", img_entries.len());
            img_by_key.insert(key, name.clone());
            img_entries.push((name.clone(), data));
            Some(name)
        });
        html_body.push_str(&rewritten);
        html_body.push('\n');
    }

    let full_html = format!(
        "<HTML><HEAD><TITLE>{}</TITLE></HEAD><BODY>{html_body}</BODY></HTML>",
        xml_escape(title)
    );
    let compressed_html = compress_rb_chunks(full_html.as_bytes())?;

    let info_bytes = format!(
        "TYPE=2\nTITLE={title}\nAUTHOR={author}\nGENERATOR=ebook-rs\nPARSE=1\nOUTPUT=1\nBODY=index.html\n"
    )
    .into_bytes();

    // Assemble TOC entries: (name, flags, data)
    let mut items: Vec<(String, u32, Vec<u8>)> = Vec::with_capacity(3 + img_entries.len());
    items.push(("info.info".to_string(), 2, info_bytes));
    items.push(("index.html".to_string(), 8, compressed_html));
    items.push(("index.hidx".to_string(), 0, b" ".to_vec()));
    for (name, data) in img_entries {
        items.push((name, 0, data));
    }

    let toc_off: usize = 0x128; // 296 bytes standard RB header
    let toc_len: usize = 4 + items.len() * 44;
    let data_start: usize = toc_off + toc_len;

    let mut total_data_len = 0usize;
    for (_, _, d) in &items {
        total_data_len += d.len();
    }
    let total_size = data_start + total_data_len;

    let mut out = vec![0u8; data_start];
    out[..14].copy_from_slice(RB_MAGIC);
    out[0x18..0x1C].copy_from_slice(&(toc_off as u32).to_le_bytes());
    out[0x1C..0x20].copy_from_slice(&(total_size as u32).to_le_bytes());
    out[toc_off..toc_off + 4].copy_from_slice(&(items.len() as u32).to_le_bytes());

    let mut cur_off = data_start;
    for (idx, (name, flags, data)) in items.iter().enumerate() {
        let entry_pos = toc_off + 4 + idx * 44;
        let name_bytes = name.as_bytes();
        let copy_len = name_bytes.len().min(31);
        out[entry_pos..entry_pos + copy_len].copy_from_slice(&name_bytes[..copy_len]);
        out[entry_pos + 32..entry_pos + 36].copy_from_slice(&(data.len() as u32).to_le_bytes());
        out[entry_pos + 36..entry_pos + 40].copy_from_slice(&(cur_off as u32).to_le_bytes());
        out[entry_pos + 40..entry_pos + 44].copy_from_slice(&flags.to_le_bytes());
        cur_off += data.len();
    }
    for (_, _, data) in items {
        out.extend_from_slice(&data);
    }

    Ok(out)
}

fn compress_rb_chunks(raw: &[u8]) -> Result<Vec<u8>, EbookError> {
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    for slice in raw.chunks(4096) {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(slice)
            .map_err(|e| EbookError::Io(e.to_string()))?;
        let compressed = enc.finish().map_err(|e| EbookError::Io(e.to_string()))?;
        chunks.push(compressed);
    }
    let mut out = Vec::with_capacity(8 + chunks.len() * 4 + raw.len() / 2);
    out.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
    out.extend_from_slice(&(raw.len() as u32).to_le_bytes());
    for ch in &chunks {
        out.extend_from_slice(&(ch.len() as u32).to_le_bytes());
    }
    for ch in chunks {
        out.extend_from_slice(&ch);
    }
    Ok(out)
}

// ============================================================================
// 6. Typst (.typ / .typst) Input & Output
// ============================================================================

/// Detect Typst document heuristics (or explicit `.typ`/`.typst` filename hint).
pub fn is_typst_input(text: &str, title_fallback: &str) -> bool {
    let lower_name = title_fallback.to_ascii_lowercase();
    if lower_name.ends_with(".typ") || lower_name.ends_with(".typst") {
        return true;
    }
    text.contains("#set document(") || text.contains("#set page(") || text.contains("#set text(")
}

/// Parse a Typst (`.typ` / `.typst`) document into a `Book`.
pub fn parse_typst(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
    let text = decode_bytes_with_encoding(bytes, None);
    let mut title = title_fallback.to_string();
    let mut creators = Vec::new();
    let mut html = String::new();
    let mut in_code = false;
    let mut code_buf = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_code {
                html.push_str(&format!(
                    "<pre><code>{}</code></pre>\n",
                    xml_escape(&code_buf)
                ));
                code_buf.clear();
                in_code = false;
            } else {
                in_code = true;
            }
            continue;
        }
        if in_code {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }
        if trimmed.starts_with("#set document(") {
            if let Some(t) = extract_quoted_param(trimmed, "title:") {
                title = t;
            }
            if let Some(a) = extract_quoted_param(trimmed, "author:") {
                creators.push(a);
            }
            continue;
        }
        if trimmed.starts_with("#set ")
            || trimmed.starts_with("#show ")
            || trimmed.starts_with("//")
        {
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('=') {
            let lvl = trimmed.chars().take_while(|&c| c == '=').count().min(6);
            if trimmed[lvl..].starts_with(' ') {
                let heading = trimmed[lvl..].trim();
                if lvl == 1 && title == title_fallback {
                    title = heading.to_string();
                }
                html.push_str(&format!(
                    "<h{lvl}>{}</h{lvl}>\n",
                    render_typst_inline(heading)
                ));
                continue;
            }
        }
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            html.push_str(&format!(
                "<ul><li>{}</li></ul>\n",
                render_typst_inline(item)
            ));
            continue;
        }
        html.push_str(&format!("<p>{}</p>\n", render_typst_inline(trimmed)));
    }

    build_markup_book(&title, creators, &html)
}

fn render_typst_inline(s: &str) -> String {
    // Handle *bold*, _italic_, `code`
    let mut out = xml_escape(s);
    out = replace_delimited(&out, "*", "<strong>", "</strong>");
    out = replace_delimited(&out, "_", "<em>", "</em>");
    out = replace_delimited(&out, "`", "<code>", "</code>");
    out
}

/// Export any `Book` to clean Typst (`.typ`) markup.
pub fn export_typst_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
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

    let mut out = String::new();
    out.push_str(&format!(
        "#set document(title: \"{}\", author: \"{}\")\n\n",
        typst_str_escape(title),
        typst_str_escape(author)
    ));

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_typst(&dom.root_nodes, &mut out);
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                out.push_str(&typst_text_escape(para));
                out.push_str("\n\n");
            }
        }
    }
    Ok(out.into_bytes())
}

fn typst_str_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn typst_text_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if matches!(ch, '#' | '*' | '_' | '`' | '<' | '>' | '@' | '$' | '\\') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn dom_to_typst(nodes: &[DomNode], out: &mut String) {
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
                let lvl = tag[1..].parse::<usize>().unwrap_or(1);
                let mut inner = String::new();
                render_typst_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("{} {}\n\n", "=".repeat(lvl), inner.trim()));
                }
            }
            "p" | "blockquote" | "dt" | "dd" | "figcaption" => {
                let mut inner = String::new();
                render_typst_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(inner.trim());
                    out.push_str("\n\n");
                }
            }
            "li" => {
                let mut inner = String::new();
                render_typst_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("- {}\n", inner.trim()));
                }
            }
            "pre" => {
                let mut txt = String::new();
                collect_plain_text_nodes(children, &mut txt);
                if !txt.trim().is_empty() {
                    out.push_str(&format!("```\n{}\n```\n\n", txt.trim_end()));
                }
            }
            "table" => {
                let mut rows: Vec<Vec<String>> = Vec::new();
                collect_table_rows_text(children, &mut rows);
                if !rows.is_empty() {
                    let cols = rows.iter().map(Vec::len).max().unwrap_or(1);
                    out.push_str(&format!("#table(\n  columns: {cols},\n"));
                    for row in rows {
                        for c in 0..cols {
                            let cell = row.get(c).map(String::as_str).unwrap_or("");
                            out.push_str(&format!("  [{}],\n", typst_text_escape(cell)));
                        }
                    }
                    out.push_str(")\n\n");
                }
            }
            _ => {
                if has_block_descendant(children) {
                    dom_to_typst(children, out);
                } else {
                    let mut inner = String::new();
                    render_typst_runs(children, &mut inner);
                    if !inner.trim().is_empty() {
                        out.push_str(inner.trim());
                        out.push_str("\n\n");
                    }
                }
            }
        }
    }
}

fn render_typst_runs(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let norm = collapse_ws(&decode_html_entities(t));
                if !norm.is_empty() {
                    out.push_str(&typst_text_escape(&norm));
                }
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                match tag.as_str() {
                    "strong" | "b" => {
                        let mut inner = String::new();
                        render_typst_runs(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("*{}*", inner.trim()));
                        }
                    }
                    "em" | "i" => {
                        let mut inner = String::new();
                        render_typst_runs(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("_{}_", inner.trim()));
                        }
                    }
                    "code" => {
                        let mut txt = String::new();
                        collect_plain_text_nodes(children, &mut txt);
                        if !txt.trim().is_empty() {
                            out.push_str(&format!("`{}`", txt.trim()));
                        }
                    }
                    "a" => {
                        let href = attributes.get("href").map(|s| s.as_str()).unwrap_or("");
                        let mut inner = String::new();
                        render_typst_runs(children, &mut inner);
                        if !href.is_empty() && !inner.trim().is_empty() {
                            out.push_str(&format!(
                                "#link(\"{}\")[{}]",
                                typst_str_escape(href),
                                inner.trim()
                            ));
                        } else {
                            out.push_str(&inner);
                        }
                    }
                    _ => render_typst_runs(children, out),
                }
            }
        }
    }
}

// ============================================================================
// 7. AsciiDoc (.adoc) Input & Output
// ============================================================================

/// Detect AsciiDoc input heuristics (or `.adoc`/`.asciidoc` extension).
pub fn is_adoc_input(text: &str, title_fallback: &str) -> bool {
    let lower_name = title_fallback.to_ascii_lowercase();
    if lower_name.ends_with(".adoc") || lower_name.ends_with(".asciidoc") {
        return true;
    }
    let trimmed = text.trim_start();
    trimmed.starts_with("= ") && (trimmed.contains("\n== ") || trimmed.contains("\n:author:"))
}

/// Parse an AsciiDoc (`.adoc`) document into a `Book`.
pub fn parse_adoc(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
    let text = decode_bytes_with_encoding(bytes, None);
    let mut title = title_fallback.to_string();
    let mut creators = Vec::new();
    let mut html = String::new();
    let mut in_listing = false;
    let mut code_buf = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "----" {
            if in_listing {
                html.push_str(&format!(
                    "<pre><code>{}</code></pre>\n",
                    xml_escape(&code_buf)
                ));
                code_buf.clear();
                in_listing = false;
            } else {
                in_listing = true;
            }
            continue;
        }
        if in_listing {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(":author:") {
            let a = rest.trim();
            if !a.is_empty() {
                creators.push(a.to_string());
            }
            continue;
        }
        if trimmed.starts_with(':') && trimmed[1..].contains(':') {
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with('=') {
            let lvl = trimmed.chars().take_while(|&c| c == '=').count().min(6);
            if trimmed[lvl..].starts_with(' ') {
                let heading = trimmed[lvl..].trim();
                if lvl == 1 {
                    title = heading.to_string();
                }
                html.push_str(&format!(
                    "<h{lvl}>{}</h{lvl}>\n",
                    render_typst_inline(heading)
                ));
                continue;
            }
        }
        if let Some(item) = trimmed.strip_prefix("* ") {
            html.push_str(&format!(
                "<ul><li>{}</li></ul>\n",
                render_typst_inline(item)
            ));
            continue;
        }
        html.push_str(&format!("<p>{}</p>\n", render_typst_inline(trimmed)));
    }

    build_markup_book(&title, creators, &html)
}

/// Export any `Book` to clean AsciiDoc (`.adoc`) markup.
pub fn export_adoc_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let meta = book.metadata();
    let title = if meta.title.trim().is_empty() {
        "Untitled"
    } else {
        meta.title.trim()
    };
    let mut out = format!("= {title}\n");
    if let Some(author) = meta
        .creators
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        out.push_str(&format!(":author: {author}\n"));
    }
    if !meta.language().trim().is_empty() {
        out.push_str(&format!(":lang: {}\n", meta.language().trim()));
    }
    out.push('\n');

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_adoc(&dom.root_nodes, &mut out);
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                out.push_str(para);
                out.push_str("\n\n");
            }
        }
    }
    Ok(out.into_bytes())
}

fn dom_to_adoc(nodes: &[DomNode], out: &mut String) {
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
                let lvl = (tag[1..].parse::<usize>().unwrap_or(1) + 1).min(6);
                let mut inner = String::new();
                render_adoc_inline(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("{} {}\n\n", "=".repeat(lvl), inner.trim()));
                }
            }
            "p" | "dt" | "dd" | "figcaption" => {
                let mut inner = String::new();
                render_adoc_inline(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(inner.trim());
                    out.push_str("\n\n");
                }
            }
            "blockquote" => {
                let mut inner = String::new();
                render_adoc_inline(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("____\n{}\n____\n\n", inner.trim()));
                }
            }
            "li" => {
                let mut inner = String::new();
                render_adoc_inline(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("* {}\n", inner.trim()));
                }
            }
            "pre" => {
                let mut txt = String::new();
                collect_plain_text_nodes(children, &mut txt);
                if !txt.trim().is_empty() {
                    out.push_str(&format!("----\n{}\n----\n\n", txt.trim_end()));
                }
            }
            "table" => {
                let mut rows: Vec<Vec<String>> = Vec::new();
                collect_table_rows_text(children, &mut rows);
                if !rows.is_empty() {
                    out.push_str("|===\n");
                    for row in rows {
                        for cell in row {
                            out.push_str(&format!("| {} ", cell.replace('|', "\\|")));
                        }
                        out.push('\n');
                    }
                    out.push_str("|===\n\n");
                }
            }
            _ => {
                if has_block_descendant(children) {
                    dom_to_adoc(children, out);
                } else {
                    let mut inner = String::new();
                    render_adoc_inline(children, &mut inner);
                    if !inner.trim().is_empty() {
                        out.push_str(inner.trim());
                        out.push_str("\n\n");
                    }
                }
            }
        }
    }
}

fn render_adoc_inline(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let norm = collapse_ws(&decode_html_entities(t));
                if !norm.is_empty() {
                    out.push_str(&norm);
                }
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                match tag.as_str() {
                    "strong" | "b" => {
                        let mut inner = String::new();
                        render_adoc_inline(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("*{}*", inner.trim()));
                        }
                    }
                    "em" | "i" => {
                        let mut inner = String::new();
                        render_adoc_inline(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("_{}_", inner.trim()));
                        }
                    }
                    "code" => {
                        let mut txt = String::new();
                        collect_plain_text_nodes(children, &mut txt);
                        if !txt.trim().is_empty() {
                            out.push_str(&format!("`{}`", txt.trim()));
                        }
                    }
                    "a" => {
                        let href = attributes.get("href").map(|s| s.as_str()).unwrap_or("");
                        let mut inner = String::new();
                        render_adoc_inline(children, &mut inner);
                        if !href.is_empty() && !inner.trim().is_empty() {
                            out.push_str(&format!("link:{href}[{}]", inner.trim()));
                        } else {
                            out.push_str(&inner);
                        }
                    }
                    _ => render_adoc_inline(children, out),
                }
            }
        }
    }
}

// ============================================================================
// 8. Emacs Org-mode (.org) Input & Output
// ============================================================================

/// Detect Org-mode input heuristics (or `.org` extension).
pub fn is_org_input(text: &str, title_fallback: &str) -> bool {
    if title_fallback.to_ascii_lowercase().ends_with(".org") {
        return true;
    }
    let lower = text.to_ascii_lowercase();
    lower.contains("#+title:") || lower.contains("#+begin_src") || lower.contains("#+begin_quote")
}

/// Parse an Emacs Org-mode (`.org`) document into a `Book`.
pub fn parse_org(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
    let text = decode_bytes_with_encoding(bytes, None);
    let mut title = title_fallback.to_string();
    let mut creators = Vec::new();
    let mut html = String::new();
    let mut in_src = false;
    let mut code_buf = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("#+begin_src") {
            in_src = true;
            continue;
        }
        if lower.starts_with("#+end_src") {
            html.push_str(&format!(
                "<pre><code>{}</code></pre>\n",
                xml_escape(&code_buf)
            ));
            code_buf.clear();
            in_src = false;
            continue;
        }
        if in_src {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }
        if let Some(rest) = strip_prefix_ci(trimmed, "#+title:") {
            let t = rest.trim();
            if !t.is_empty() {
                title = t.to_string();
            }
            continue;
        }
        if let Some(rest) = strip_prefix_ci(trimmed, "#+author:") {
            let a = rest.trim();
            if !a.is_empty() {
                creators.push(a.to_string());
            }
            continue;
        }
        if trimmed.starts_with("#+") || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('*') {
            let lvl = trimmed.chars().take_while(|&c| c == '*').count().min(6);
            if trimmed[lvl..].starts_with(' ') {
                let heading = trimmed[lvl..].trim();
                html.push_str(&format!(
                    "<h{lvl}>{}</h{lvl}>\n",
                    render_org_inline(heading)
                ));
                continue;
            }
        }
        if let Some(item) = trimmed.strip_prefix("- ") {
            html.push_str(&format!("<ul><li>{}</li></ul>\n", render_org_inline(item)));
            continue;
        }
        html.push_str(&format!("<p>{}</p>\n", render_org_inline(trimmed)));
    }

    build_markup_book(&title, creators, &html)
}

fn render_org_inline(s: &str) -> String {
    let mut out = xml_escape(s);
    out = replace_delimited(&out, "*", "<strong>", "</strong>");
    out = replace_delimited(&out, "/", "<em>", "</em>");
    out = replace_delimited(&out, "~", "<code>", "</code>");
    out
}

/// Export any `Book` to clean Emacs Org-mode (`.org`) markup.
pub fn export_org_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let meta = book.metadata();
    let title = if meta.title.trim().is_empty() {
        "Untitled"
    } else {
        meta.title.trim()
    };
    let mut out = format!("#+TITLE: {title}\n");
    if let Some(author) = meta
        .creators
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        out.push_str(&format!("#+AUTHOR: {author}\n"));
    }
    if !meta.language().trim().is_empty() {
        out.push_str(&format!("#+LANGUAGE: {}\n", meta.language().trim()));
    }
    out.push('\n');

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_org(&dom.root_nodes, &mut out);
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                out.push_str(para);
                out.push_str("\n\n");
            }
        }
    }
    Ok(out.into_bytes())
}

fn dom_to_org(nodes: &[DomNode], out: &mut String) {
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
                let lvl = tag[1..].parse::<usize>().unwrap_or(1);
                let mut inner = String::new();
                render_org_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("{} {}\n\n", "*".repeat(lvl), inner.trim()));
                }
            }
            "p" | "dt" | "dd" | "figcaption" => {
                let mut inner = String::new();
                render_org_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(inner.trim());
                    out.push_str("\n\n");
                }
            }
            "blockquote" => {
                let mut inner = String::new();
                render_org_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("#+BEGIN_QUOTE\n{}\n#+END_QUOTE\n\n", inner.trim()));
                }
            }
            "li" => {
                let mut inner = String::new();
                render_org_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("- {}\n", inner.trim()));
                }
            }
            "pre" => {
                let mut txt = String::new();
                collect_plain_text_nodes(children, &mut txt);
                if !txt.trim().is_empty() {
                    out.push_str(&format!("#+BEGIN_SRC\n{}\n#+END_SRC\n\n", txt.trim_end()));
                }
            }
            "table" => {
                let mut rows: Vec<Vec<String>> = Vec::new();
                collect_table_rows_text(children, &mut rows);
                for row in rows {
                    out.push('|');
                    for cell in row {
                        out.push_str(&format!(" {} |", cell.replace('|', "/")));
                    }
                    out.push('\n');
                }
                out.push('\n');
            }
            _ => {
                if has_block_descendant(children) {
                    dom_to_org(children, out);
                } else {
                    let mut inner = String::new();
                    render_org_runs(children, &mut inner);
                    if !inner.trim().is_empty() {
                        out.push_str(inner.trim());
                        out.push_str("\n\n");
                    }
                }
            }
        }
    }
}

fn render_org_runs(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let norm = collapse_ws(&decode_html_entities(t));
                if !norm.is_empty() {
                    out.push_str(&norm);
                }
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                match tag.as_str() {
                    "strong" | "b" => {
                        let mut inner = String::new();
                        render_org_runs(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("*{}*", inner.trim()));
                        }
                    }
                    "em" | "i" => {
                        let mut inner = String::new();
                        render_org_runs(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("/{}/", inner.trim()));
                        }
                    }
                    "code" => {
                        let mut txt = String::new();
                        collect_plain_text_nodes(children, &mut txt);
                        if !txt.trim().is_empty() {
                            out.push_str(&format!("~{}~", txt.trim()));
                        }
                    }
                    "a" => {
                        let href = attributes.get("href").map(|s| s.as_str()).unwrap_or("");
                        let mut inner = String::new();
                        render_org_runs(children, &mut inner);
                        if !href.is_empty() && !inner.trim().is_empty() {
                            out.push_str(&format!("[[{href}][{}]]", inner.trim()));
                        } else {
                            out.push_str(&inner);
                        }
                    }
                    _ => render_org_runs(children, out),
                }
            }
        }
    }
}

// ============================================================================
// 9. reStructuredText (.rst) Input & Output
// ============================================================================

/// Detect reStructuredText input heuristics (or `.rst` extension).
pub fn is_rst_input(text: &str, title_fallback: &str) -> bool {
    if title_fallback.to_ascii_lowercase().ends_with(".rst") {
        return true;
    }
    text.contains("\n.. ") || text.contains("\n===\n") || text.contains(":Author:")
}

/// Parse a reStructuredText (`.rst`) document into a `Book`.
pub fn parse_rst(bytes: &[u8], title_fallback: &str) -> Result<Book, EbookError> {
    let text = decode_bytes_with_encoding(bytes, None);
    let lines: Vec<&str> = text.lines().collect();
    let mut title = title_fallback.to_string();
    let mut creators = Vec::new();
    let mut html = String::new();

    let mut i = 0usize;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        if let Some(rest) = strip_prefix_ci(trimmed, ":author:") {
            let a = rest.trim();
            if !a.is_empty() {
                creators.push(a.to_string());
            }
            i += 1;
            continue;
        }
        if trimmed.starts_with(".. ") {
            i += 1;
            continue;
        }
        // Check underlined heading (next line all === or --- or ~~~)
        if i + 1 < lines.len() {
            let next = lines[i + 1].trim();
            if next.len() >= 3
                && (next.chars().all(|c| c == '=')
                    || next.chars().all(|c| c == '-')
                    || next.chars().all(|c| c == '~'))
            {
                let lvl = match next.chars().next().unwrap_or('=') {
                    '=' => 1,
                    '-' => 2,
                    _ => 3,
                };
                if lvl == 1 && title == title_fallback {
                    title = trimmed.to_string();
                }
                html.push_str(&format!(
                    "<h{lvl}>{}</h{lvl}>\n",
                    render_rst_inline(trimmed)
                ));
                i += 2;
                continue;
            }
        }
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            html.push_str(&format!("<ul><li>{}</li></ul>\n", render_rst_inline(item)));
            i += 1;
            continue;
        }
        html.push_str(&format!("<p>{}</p>\n", render_rst_inline(trimmed)));
        i += 1;
    }

    build_markup_book(&title, creators, &html)
}

fn render_rst_inline(s: &str) -> String {
    let mut out = xml_escape(s);
    out = replace_delimited(&out, "**", "<strong>", "</strong>");
    out = replace_delimited(&out, "*", "<em>", "</em>");
    out = replace_delimited(&out, "``", "<code>", "</code>");
    out
}

/// Export any `Book` to clean reStructuredText (`.rst`) markup.
pub fn export_rst_bytes(book: &Book) -> Result<Vec<u8>, EbookError> {
    let meta = book.metadata();
    let title = if meta.title.trim().is_empty() {
        "Untitled"
    } else {
        meta.title.trim()
    };
    let bar = "=".repeat(title.chars().count().max(3));
    let mut out = format!("{title}\n{bar}\n\n");
    if let Some(author) = meta
        .creators
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        out.push_str(&format!(":Author: {author}\n\n"));
    }

    for idx in 0..book.spine().len() {
        let Ok(sec) = book.get_section(idx) else {
            continue;
        };
        if !sec.raw_html.trim().is_empty() {
            let dom = EbookDomTree::parse(&sec.raw_html);
            dom_to_rst(&dom.root_nodes, &mut out);
        } else if !sec.plain_text.trim().is_empty() {
            for para in sec
                .plain_text
                .split('\n')
                .map(str::trim)
                .filter(|p| !p.is_empty())
            {
                out.push_str(para);
                out.push_str("\n\n");
            }
        }
    }
    Ok(out.into_bytes())
}

fn dom_to_rst(nodes: &[DomNode], out: &mut String) {
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
                let ch = match tag.as_str() {
                    "h1" => '=',
                    "h2" => '-',
                    _ => '~',
                };
                let mut inner = String::new();
                render_rst_runs(children, &mut inner);
                let t = inner.trim();
                if !t.is_empty() {
                    let ul = ch.to_string().repeat(t.chars().count().max(3));
                    out.push_str(&format!("{t}\n{ul}\n\n"));
                }
            }
            "p" | "dt" | "dd" | "figcaption" => {
                let mut inner = String::new();
                render_rst_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(inner.trim());
                    out.push_str("\n\n");
                }
            }
            "blockquote" => {
                let mut inner = String::new();
                render_rst_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    for line in inner.trim().lines() {
                        out.push_str(&format!("   {line}\n"));
                    }
                    out.push('\n');
                }
            }
            "li" => {
                let mut inner = String::new();
                render_rst_runs(children, &mut inner);
                if !inner.trim().is_empty() {
                    out.push_str(&format!("- {}\n", inner.trim()));
                }
            }
            "pre" => {
                let mut txt = String::new();
                collect_plain_text_nodes(children, &mut txt);
                if !txt.trim().is_empty() {
                    out.push_str("::\n\n");
                    for line in txt.trim_end().lines() {
                        out.push_str(&format!("   {line}\n"));
                    }
                    out.push('\n');
                }
            }
            _ => {
                if has_block_descendant(children) {
                    dom_to_rst(children, out);
                } else {
                    let mut inner = String::new();
                    render_rst_runs(children, &mut inner);
                    if !inner.trim().is_empty() {
                        out.push_str(inner.trim());
                        out.push_str("\n\n");
                    }
                }
            }
        }
    }
}

fn render_rst_runs(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        match node {
            DomNode::Comment(_) => {}
            DomNode::Text(t) => {
                let norm = collapse_ws(&decode_html_entities(t));
                if !norm.is_empty() {
                    out.push_str(&norm);
                }
            }
            DomNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let tag = tag_name.to_ascii_lowercase();
                match tag.as_str() {
                    "strong" | "b" => {
                        let mut inner = String::new();
                        render_rst_runs(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("**{}**", inner.trim()));
                        }
                    }
                    "em" | "i" => {
                        let mut inner = String::new();
                        render_rst_runs(children, &mut inner);
                        if !inner.trim().is_empty() {
                            out.push_str(&format!("*{}*", inner.trim()));
                        }
                    }
                    "code" => {
                        let mut txt = String::new();
                        collect_plain_text_nodes(children, &mut txt);
                        if !txt.trim().is_empty() {
                            out.push_str(&format!("``{}``", txt.trim()));
                        }
                    }
                    "a" => {
                        let href = attributes.get("href").map(|s| s.as_str()).unwrap_or("");
                        let mut inner = String::new();
                        render_rst_runs(children, &mut inner);
                        if !href.is_empty() && !inner.trim().is_empty() {
                            out.push_str(&format!("`{} <{href}>`_", inner.trim()));
                        } else {
                            out.push_str(&inner);
                        }
                    }
                    _ => render_rst_runs(children, out),
                }
            }
        }
    }
}

// ============================================================================
// Shared Helpers
// ============================================================================

fn build_markup_book(
    title: &str,
    creators: Vec<String>,
    body_html: &str,
) -> Result<Book, EbookError> {
    let raw_html = format!("<div class=\"markup-content\">\n{body_html}</div>");
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
    let mut metadata = default_metadata(title);
    metadata.creators = creators;
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
    Ok(build_synthetic_book(
        EpubArchive::empty(),
        opf,
        toc,
        vec![section],
    ))
}

fn default_metadata(title: &str) -> Metadata {
    Metadata {
        title: title.to_string(),
        creators: Vec::new(),
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
    }
}

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

fn has_block_descendant(nodes: &[DomNode]) -> bool {
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

fn collect_plain_text_nodes(nodes: &[DomNode], out: &mut String) {
    for node in nodes {
        match node {
            DomNode::Text(t) => out.push_str(&decode_html_entities(t)),
            DomNode::Element { children, .. } => collect_plain_text_nodes(children, out),
            DomNode::Comment(_) => {}
        }
    }
}

fn collect_table_rows_text(nodes: &[DomNode], rows: &mut Vec<Vec<String>>) {
    for node in nodes {
        if let DomNode::Element {
            tag_name, children, ..
        } = node
        {
            if tag_name.eq_ignore_ascii_case("tr") {
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
                        collect_plain_text_nodes(cc, &mut txt);
                        cells.push(collapse_ws(&txt).trim().to_string());
                    }
                }
                if !cells.is_empty() {
                    rows.push(cells);
                }
            } else {
                collect_table_rows_text(children, rows);
            }
        }
    }
}

fn extract_body_inner_html(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = lower.find("<body")?;
    let open_end = html[open..].find('>')? + open + 1;
    let close = lower.rfind("</body>").unwrap_or(html.len());
    (close >= open_end).then(|| html[open_end..close].to_string())
}

fn rewrite_html_img_srcs<F>(html: &str, mut map_src: F) -> String
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

fn mime_for_asset(lower: &str) -> Option<&'static str> {
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
    } else {
        None
    }
}

fn parse_image_dims(data: &[u8]) -> Option<(u32, u32)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") && data.len() >= 24 && &data[12..16] == b"IHDR" {
        let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return Some((w, h));
    }
    if data.starts_with(b"GIF8") && data.len() >= 10 {
        return Some((
            u32::from(u16::from_le_bytes([data[6], data[7]])),
            u32::from(u16::from_le_bytes([data[8], data[9]])),
        ));
    }
    if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
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
            let m = data[i];
            i += 1;
            if m == 0xD9 || m == 0xDA {
                break;
            }
            if i + 2 > data.len() {
                break;
            }
            let seg_len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
            if seg_len < 2 || i + seg_len > data.len() {
                break;
            }
            if matches!(m, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF)
                && i + 7 < data.len()
            {
                let h = u32::from(u16::from_be_bytes([data[i + 3], data[i + 4]]));
                let w = u32::from(u16::from_be_bytes([data[i + 5], data[i + 6]]));
                return Some((w, h));
            }
            i += seg_len;
        }
    }
    None
}

fn extract_quoted_param(line: &str, key: &str) -> Option<String> {
    let idx = line.find(key)?;
    let rest = line[idx + key.len()..].trim_start();
    let after_q = rest.strip_prefix('"')?;
    let end = after_q.find('"')?;
    Some(after_q[..end].to_string())
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    s.get(..prefix.len())
        .filter(|p| p.eq_ignore_ascii_case(prefix))
        .map(|_| &s[prefix.len()..])
}

fn replace_delimited(s: &str, delim: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find(delim) {
        out.push_str(&rest[..start]);
        let after = &rest[start + delim.len()..];
        if let Some(end) = after.find(delim)
            && end > 0
        {
            out.push_str(open);
            out.push_str(&after[..end]);
            out.push_str(close);
            rest = &after[end + delim.len()..];
        } else {
            out.push_str(delim);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}
