use crate::book::Book;

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
use std::collections::HashMap;

/// PDF Document Parser using pre-extracted text / markdown via `pdf_oxide`.
pub struct PdfBook;

impl PdfBook {
    /// Parse raw PDF byte data into a `Book` struct.
    #[cfg(feature = "pdf")]
    pub fn parse(bytes: &[u8], title_fallback: &str) -> Result<Book, String> {
        let doc = pdf_oxide::PdfDocument::from_bytes(bytes.to_vec())
            .map_err(|e| format!("Failed to open PDF document with pdf_oxide: {}", e))?;

        let page_count = doc
            .page_count()
            .map_err(|e| format!("Failed to get PDF page count: {}", e))?;

        if page_count == 0 {
            return Err("PDF document contains 0 pages".to_string());
        }

        let mut sections = Vec::with_capacity(page_count);
        let mut spine = Vec::with_capacity(page_count);
        let mut toc = Vec::with_capacity(page_count);

        for page_idx in 0..page_count {
            let markdown_content = doc
                .to_markdown(page_idx, &Default::default())
                .unwrap_or_else(|_| match doc.extract_text(page_idx) {
                    Ok(txt) => txt,
                    Err(_) => format!("<p>Page {}</p>", page_idx + 1),
                });

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
                viewport_width: None,
                viewport_height: None,
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

        let metadata = Metadata {
            title: title_fallback.to_string(),
            creators: vec!["PDF Document".to_string()],
            publishers: Vec::new(),
            languages: vec!["en".to_string()],
            rights: None,
            description: Some(format!("PDF Document ({} pages)", page_count)),
            identifier: None,
            pub_date: None,
            modified_date: None,
            subjects: vec!["PDF".to_string()],
            cover_id: None,
            cover_href: None,
            direction: PageProgressionDirection::Ltr,
            meta_properties: HashMap::new(),
            accessibility: Default::default(),
        };

        let opf = OpfPackage {
            version: "3.0".to_string(),
            opf_path: "content.opf".to_string(),
            opf_dir: "".to_string(),
            metadata,
            manifest: HashMap::new(),
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
            media_overlays: HashMap::new(),
        };

        book.generate_locations(1000);
        Ok(book)
    }

    #[cfg(not(feature = "pdf"))]
    pub fn parse(_bytes: &[u8], _title_fallback: &str) -> Result<Book, String> {
        Err("PDF support requires the 'pdf' feature flag to be enabled in Cargo.toml".to_string())
    }
}

/// Convert Markdown content to structured HTML section for PDF pages.
#[cfg(feature = "pdf")]
fn markdown_to_html(md: &str, page_num: usize) -> String {
    let mut html = format!("<div class=\"pdf-page\" data-page=\"{}\">\n", page_num);
    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(h1) = trimmed.strip_prefix("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", h1.trim()));
        } else if let Some(h2) = trimmed.strip_prefix("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", h2.trim()));
        } else if let Some(h3) = trimmed.strip_prefix("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", h3.trim()));
        } else if let Some(bullet) = trimmed.strip_prefix("- ") {
            html.push_str(&format!("<li>{}</li>\n", bullet.trim()));
        } else if trimmed.starts_with('<') && trimmed.ends_with('>') {
            html.push_str(trimmed);
            html.push('\n');
        } else {
            html.push_str(&format!("<p>{}</p>\n", trimmed));
        }
    }
    html.push_str("</div>");
    html
}
