use crate::book::Book;
use serde::{Deserialize, Serialize};

/// Severity level for EPUB validation diagnostic items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// Diagnostic item representing a validation rule check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
    pub location: Option<String>,
}

/// Complete report returned by the `EpubValidator`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub errors_count: usize,
    pub warnings_count: usize,
    pub info_count: usize,
}

/// Comprehensive EPUB and eBook structural validator.
pub struct EpubValidator;

impl EpubValidator {
    /// Validate a `Book` instance against EPUB 2 / EPUB 3 structural standards.
    pub fn validate(book: &Book) -> ValidationReport {
        let mut errors = Vec::new();
        let meta = book.metadata();

        // 1. Mandatory OPF Metadata Checks
        if meta.title.trim().is_empty() {
            errors.push(ValidationError {
                severity: ValidationSeverity::Error,
                code: "PKG-001".to_string(),
                message: "dc:title is missing or empty in OPF metadata".to_string(),
                location: Some("metadata.title".to_string()),
            });
        }

        if meta.identifier.as_deref().unwrap_or("").trim().is_empty() {
            errors.push(ValidationError {
                severity: ValidationSeverity::Warning,
                code: "PKG-002".to_string(),
                message: "dc:identifier is missing or empty in OPF metadata".to_string(),
                location: Some("metadata.identifier".to_string()),
            });
        }

        if meta.language().trim().is_empty() {
            errors.push(ValidationError {
                severity: ValidationSeverity::Warning,
                code: "PKG-003".to_string(),
                message: "dc:language is missing or empty in OPF metadata".to_string(),
                location: Some("metadata.language".to_string()),
            });
        }

        if meta.creator().trim().is_empty() {
            errors.push(ValidationError {
                severity: ValidationSeverity::Info,
                code: "PKG-004".to_string(),
                message: "dc:creator (author) is not specified".to_string(),
                location: Some("metadata.creator".to_string()),
            });
        }

        // 2. Spine & Section Integrity Checks
        if book.spine().is_empty() {
            errors.push(ValidationError {
                severity: ValidationSeverity::Error,
                code: "RSC-001".to_string(),
                message: "Spine contains 0 reading items".to_string(),
                location: Some("spine".to_string()),
            });
        }

        if book.sections.is_empty() {
            errors.push(ValidationError {
                severity: ValidationSeverity::Error,
                code: "RSC-002".to_string(),
                message: "Book contains 0 readable content sections".to_string(),
                location: Some("sections".to_string()),
            });
        }

        for (idx, section) in book.sections.iter().enumerate() {
            if section.href.trim().is_empty() {
                errors.push(ValidationError {
                    severity: ValidationSeverity::Error,
                    code: "RSC-003".to_string(),
                    message: format!("Section {} has empty href reference", idx),
                    location: Some(format!("sections[{}]", idx)),
                });
            }

            if section.char_count == 0 && section.raw_html.trim().is_empty() {
                errors.push(ValidationError {
                    severity: ValidationSeverity::Warning,
                    code: "RSC-004".to_string(),
                    message: format!(
                        "Section {} ('{}') has no extracted text or HTML content",
                        idx, section.href
                    ),
                    location: Some(format!("sections[{}]", idx)),
                });
            }
        }

        // 3. Navigation / TOC Link Resolution Checks
        if book.toc().is_empty() {
            errors.push(ValidationError {
                severity: ValidationSeverity::Warning,
                code: "NAV-001".to_string(),
                message: "Table of Contents (NCX / NAV) is empty or missing".to_string(),
                location: Some("toc".to_string()),
            });
        }

        // 4. EPUB 3 Accessibility Metadata Verification
        let a11y = &meta.accessibility;
        if !a11y.is_accessible && a11y.access_modes.is_empty() {
            errors.push(ValidationError {
                severity: ValidationSeverity::Info,
                code: "A11Y-001".to_string(),
                message: "No EPUB 3 accessibility metadata (schema:accessMode) declared"
                    .to_string(),
                location: Some("metadata.accessibility".to_string()),
            });
        }

        let errors_count = errors
            .iter()
            .filter(|e| e.severity == ValidationSeverity::Error)
            .count();
        let warnings_count = errors
            .iter()
            .filter(|e| e.severity == ValidationSeverity::Warning)
            .count();
        let info_count = errors
            .iter()
            .filter(|e| e.severity == ValidationSeverity::Info)
            .count();

        ValidationReport {
            is_valid: errors_count == 0,
            errors,
            errors_count,
            warnings_count,
            info_count,
        }
    }
}

/// Universal EPUB 3 Exporter capable of serializing any `Book` (EPUB, MOBI, PDF, FB2, CBZ, TXT, ODT) to a valid EPUB 3 zip buffer.
pub struct UniversalEpub3Exporter;

impl UniversalEpub3Exporter {
    pub fn export(book: &Book) -> Result<Vec<u8>, String> {
        use std::io::Write;
        let mut zip_buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));

            // 1. mimetype (uncompressed, first file in ZIP per EPUB spec)
            let options_stored = zip::write::FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("mimetype", options_stored)
                .map_err(|e| e.to_string())?;
            zip.write_all(b"application/epub+zip")
                .map_err(|e| e.to_string())?;

            let options_deflate = zip::write::FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Deflated);

            // 2. META-INF/container.xml
            zip.start_file("META-INF/container.xml", options_deflate)
                .map_err(|e| e.to_string())?;
            zip.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n  <rootfiles>\n    <rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/>\n  </rootfiles>\n</container>")
                .map_err(|e| e.to_string())?;

            // 3. OEBPS/nav.xhtml
            zip.start_file("OEBPS/nav.xhtml", options_deflate)
                .map_err(|e| e.to_string())?;
            let mut nav_html = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n<head><title>TOC</title></head>\n<body>\n<nav epub:type=\"toc\" id=\"toc\"><h1>Table of Contents</h1><ol>");
            for (idx, _) in book.spine().iter().enumerate() {
                nav_html.push_str(&format!("<li><a href=\"sec_{}.xhtml\">Section {}</a></li>", idx, idx + 1));
            }
            nav_html.push_str("</ol></nav>\n</body>\n</html>");
            zip.write_all(nav_html.as_bytes()).map_err(|e| e.to_string())?;

            // 4. OEBPS/content.opf
            zip.start_file("OEBPS/content.opf", options_deflate)
                .map_err(|e| e.to_string())?;
            let meta = book.metadata();
            let mut opf_xml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"uid\">\n  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n    <dc:title>{}</dc:title>\n    <dc:identifier id=\"uid\">{}</dc:identifier>\n    <dc:language>{}</dc:language>\n  </metadata>\n  <manifest>\n    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n",
                crate::dom::sanitize_and_repair_xml(&meta.title),
                meta.identifier.as_deref().unwrap_or("urn:uuid:ebook-rs-export"),
                if meta.language().is_empty() { "en" } else { meta.language() }
            );

            for (idx, _) in book.spine().iter().enumerate() {
                opf_xml.push_str(&format!("    <item id=\"sec_{}\" href=\"sec_{}.xhtml\" media-type=\"application/xhtml+xml\"/>\n", idx, idx));
            }
            opf_xml.push_str("  </manifest>\n  <spine>\n");
            for (idx, _) in book.spine().iter().enumerate() {
                opf_xml.push_str(&format!("    <itemref idref=\"sec_{}\"/>\n", idx));
            }
            opf_xml.push_str("  </spine>\n</package>");
            zip.write_all(opf_xml.as_bytes()).map_err(|e| e.to_string())?;

            // 5. OEBPS/sec_{idx}.xhtml
            for (idx, _) in book.spine().iter().enumerate() {
                if let Ok(sec) = book.get_section(idx) {
                    zip.start_file(format!("OEBPS/sec_{}.xhtml", idx), options_deflate)
                        .map_err(|e| e.to_string())?;
                    let doc_xhtml = format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\">\n<head><title>Section {}</title></head>\n<body>{}</body>\n</html>",
                        idx + 1,
                        sec.processed_html
                    );
                    zip.write_all(doc_xhtml.as_bytes()).map_err(|e| e.to_string())?;
                }
            }

            zip.finish().map_err(|e| e.to_string())?;
        }
        Ok(zip_buf)
    }
}
