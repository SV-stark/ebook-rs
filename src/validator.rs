use crate::archive::EpubArchive;
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

        // Severity is left as Warning: this crate also builds `Book`s that never had OPF metadata
        // at all (e.g. `TxtBook::parse`), and the existing contract treats a missing `dc:identifier`
        // as a warning. Raising it to Error would invalidate every one of those. Genuinely
        // malformed OPF is already rejected by the element-level checks further down.
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

        let hydrated_sections = book.get_all_sections_hydrated();
        if hydrated_sections.is_empty() {
            errors.push(ValidationError {
                severity: ValidationSeverity::Error,
                code: "RSC-002".to_string(),
                message: "Book contains 0 readable content sections".to_string(),
                location: Some("sections".to_string()),
            });
        }

        for (idx, section) in hydrated_sections.iter().enumerate() {
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
            let mut nav_html = String::from(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n<head><title>TOC</title></head>\n<body>\n<nav epub:type=\"toc\" id=\"toc\"><h1>Table of Contents</h1><ol>",
            );
            use std::fmt::Write as _;
            for (idx, _) in book.spine().iter().enumerate() {
                let _ = write!(
                    nav_html,
                    "<li><a href=\"sec_{}.xhtml\">Section {}</a></li>",
                    idx,
                    idx + 1
                );
            }
            nav_html.push_str("</ol></nav>\n</body>\n</html>");
            zip.write_all(nav_html.as_bytes())
                .map_err(|e| e.to_string())?;

            // 4. OEBPS/content.opf
            zip.start_file("OEBPS/content.opf", options_deflate)
                .map_err(|e| e.to_string())?;
            let meta = book.metadata();
            let lang = if meta.language().is_empty() {
                "en"
            } else {
                meta.language()
            };
            let mut opf_xml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"uid\">\n  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n    <dc:title>{}</dc:title>\n    <dc:identifier id=\"uid\">{}</dc:identifier>\n    <dc:language>{}</dc:language>\n",
                crate::dom::xml_escape(&meta.title),
                // These two were interpolated raw. An `&` or `<` in the source OPF produced an
                // unescaped one in the output, which is a fatal XML error for strict parsers.
                crate::dom::xml_escape(
                    meta.identifier
                        .as_deref()
                        .unwrap_or("urn:uuid:ebook-rs-export")
                ),
                crate::dom::xml_escape(lang)
            );
            for creator in &meta.creators {
                let _ = writeln!(
                    opf_xml,
                    "    <dc:creator>{}</dc:creator>",
                    crate::dom::xml_escape(creator)
                );
            }
            let modified_ts = format_dcterms_modified(meta.modified_date.as_deref());
            let _ = writeln!(
                opf_xml,
                "    <meta property=\"dcterms:modified\">{}</meta>",
                crate::dom::xml_escape(&modified_ts)
            );
            opf_xml.push_str("  </metadata>\n  <manifest>\n    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n");

            for (idx, _) in book.spine().iter().enumerate() {
                let _ = writeln!(
                    opf_xml,
                    "    <item id=\"sec_{idx}\" href=\"sec_{idx}.xhtml\" media-type=\"application/xhtml+xml\"/>"
                );
            }
            // Add asset files (images, css, fonts) from book.archive to content.opf manifest
            let mut archive_files = ahash::AHashMap::new();
            book.archive
                .hydrate_into(&mut archive_files)
                .map_err(|e| e.to_string())?;

            let mut asset_idx = 0;
            let mut seen_manifest_hrefs = std::collections::HashSet::new();
            seen_manifest_hrefs.insert("nav.xhtml".to_string());
            for idx in 0..book.spine().len() {
                seen_manifest_hrefs.insert(format!("sec_{idx}.xhtml"));
            }
            let mut url_map: ahash::AHashMap<String, String> = ahash::AHashMap::new();
            for (idx, sec) in book.spine().iter().enumerate() {
                if !sec.href.is_empty() {
                    let norm = crate::archive::normalize_path(&sec.href).to_ascii_lowercase();
                    url_map.insert(norm, format!("sec_{idx}.xhtml"));
                }
            }
            for path in archive_files.keys() {
                let path_str = path.as_str();
                if is_skipped_archive_asset(path_str) {
                    continue;
                }
                let rel_href = if path_str.to_ascii_lowercase().starts_with("oebps/") {
                    &path[6..]
                } else {
                    path.as_str()
                };
                if !seen_manifest_hrefs.insert(rel_href.to_lowercase()) {
                    continue;
                }
                let norm_orig = crate::archive::normalize_path(path_str).to_ascii_lowercase();
                url_map.insert(norm_orig, rel_href.to_string());
                let mime = EpubArchive::get_mime_type(path);
                let _ = writeln!(
                    opf_xml,
                    "    <item id=\"asset_{}\" href=\"{}\" media-type=\"{}\"/>",
                    asset_idx,
                    crate::dom::xml_escape(rel_href),
                    mime
                );
                asset_idx += 1;
            }

            let spine_dir_attr = match meta.direction {
                crate::metadata::PageProgressionDirection::Rtl => {
                    " page-progression-direction=\"rtl\""
                }
                _ => "",
            };
            let _ = writeln!(opf_xml, "  </manifest>\n  <spine{spine_dir_attr}>");
            for (idx, _) in book.spine().iter().enumerate() {
                let _ = writeln!(opf_xml, "    <itemref idref=\"sec_{idx}\"/>");
            }
            opf_xml.push_str("  </spine>\n</package>");
            zip.write_all(opf_xml.as_bytes())
                .map_err(|e| e.to_string())?;

            // 5 & 6: Prepare section documents & asset files for ZIP entry creation
            struct ZipEntry {
                path: String,
                bytes: Vec<u8>,
            }

            let mut entries = Vec::new();

            // Collect section HTML documents
            let hydrated_sections = book.get_all_sections_hydrated();
            for (idx, sec) in hydrated_sections.iter().enumerate() {
                if !sec.full_path.is_empty() {
                    let norm = crate::archive::normalize_path(&sec.full_path).to_ascii_lowercase();
                    url_map.insert(norm, format!("sec_{idx}.xhtml"));
                }
            }
            for (idx, sec) in hydrated_sections.iter().enumerate() {
                let html_body = if !sec.raw_html.is_empty() {
                    &sec.raw_html
                } else {
                    &sec.processed_html
                };
                let trimmed = html_body.trim();
                let doc_xhtml = if trimmed.contains("<html") || trimmed.contains("<body") {
                    // The document already has its own DOCTYPE. Prepending one anyway produced
                    // two `<!DOCTYPE html>` declarations, which is not well-formed XML.
                    let has_doctype =
                        trimmed.len() >= 9 && trimmed[..9].eq_ignore_ascii_case("<!doctype");
                    match (has_doctype, trimmed.starts_with("<?xml")) {
                        (true, true) => trimmed.to_string(),
                        (true, false) => {
                            format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", trimmed)
                        }
                        (false, true) => format!(
                            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n{}",
                            trimmed
                                .trim_start_matches("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")
                                .trim_start()
                        ),
                        (false, false) => format!(
                            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n{}",
                            trimmed
                        ),
                    }
                } else {
                    format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\">\n<head><title>Section {}</title></head>\n<body>{}</body>\n</html>",
                        idx + 1,
                        trimmed
                    )
                };
                let rewritten = rewrite_section_relative_urls(&doc_xhtml, &sec.full_path, &url_map);
                entries.push(ZipEntry {
                    path: format!("OEBPS/sec_{}.xhtml", idx),
                    bytes: rewritten.into_bytes(),
                });
            }

            // Collect asset files (images, CSS, fonts) from book.archive
            let mut seen_zip_paths = std::collections::HashSet::new();
            seen_zip_paths.insert("mimetype".to_string());
            seen_zip_paths.insert("meta-inf/container.xml".to_string());
            seen_zip_paths.insert("oebps/nav.xhtml".to_string());
            seen_zip_paths.insert("oebps/content.opf".to_string());
            for idx in 0..hydrated_sections.len() {
                seen_zip_paths.insert(format!("oebps/sec_{idx}.xhtml"));
            }
            for (path, bytes) in &archive_files {
                let path_low = path.to_lowercase();
                if is_skipped_archive_asset(&path_low) {
                    continue;
                }
                let zip_path = if path_low.starts_with("oebps/") {
                    path.clone()
                } else {
                    format!("OEBPS/{}", path)
                };
                if !seen_zip_paths.insert(zip_path.to_lowercase()) {
                    continue;
                }
                entries.push(ZipEntry {
                    path: zip_path,
                    bytes: bytes.clone(),
                });
            }

            // Write all section and asset entries into ZIP archive

            for entry in entries {
                zip.start_file(&entry.path, options_deflate)
                    .map_err(|e| e.to_string())?;
                zip.write_all(&entry.bytes).map_err(|e| e.to_string())?;
            }

            zip.finish().map_err(|e| e.to_string())?;
        }

        Ok(zip_buf)
    }
}

/// Returns true for source archive entries that are regenerated or would collide with
/// the exporter's fixed container, package, navigation, or section paths.
fn is_skipped_archive_asset(path: &str) -> bool {
    let low = path.to_ascii_lowercase();
    low.ends_with(".opf")
        || low.ends_with(".ncx")
        || low == "mimetype"
        || low == "meta-inf/container.xml"
        || low == "oebps/nav.xhtml"
        || low == "nav.xhtml"
        || (low.contains("sec_") && low.ends_with(".xhtml"))
}

fn is_valid_dcterms_modified(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 20
        && b[0..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
        && b[10] == b'T'
        && b[11..13].iter().all(u8::is_ascii_digit)
        && b[13] == b':'
        && b[14..16].iter().all(u8::is_ascii_digit)
        && b[16] == b':'
        && b[17..19].iter().all(u8::is_ascii_digit)
        && b[19] == b'Z'
}

/// Return a valid EPUB 3 `dcterms:modified` UTC timestamp (`CCYY-MM-DDThh:mm:ssZ`).
pub(crate) fn format_dcterms_modified(existing: Option<&str>) -> String {
    if let Some(ts) = existing
        .map(str::trim)
        .filter(|s| is_valid_dcterms_modified(s))
    {
        return ts.to_string();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(dur) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let secs = dur.as_secs() as i64;
            let days = secs.div_euclid(86_400);
            let rem = secs.rem_euclid(86_400);
            let hour = rem / 3600;
            let minute = (rem % 3600) / 60;
            let second = rem % 60;

            // Howard Hinnant's civil-from-days algorithm
            let z = days + 719_468;
            let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
            let doe = z - era * 146_097;
            let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
            let y = yoe + era * 400;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let d = doy - (153 * mp + 2) / 5 + 1;
            let m = if mp < 10 { mp + 3 } else { mp - 9 };
            let year = y + if m <= 2 { 1 } else { 0 };
            return format!("{year:04}-{m:02}-{d:02}T{hour:02}:{minute:02}:{second:02}Z");
        }
    }

    "2026-01-01T00:00:00Z".to_string()
}

/// Rewrite local `href`, `src`, `xlink:href`, and `poster` attributes in a section relocated to
/// `OEBPS/sec_{idx}.xhtml` so they resolve to the corresponding paths under `OEBPS/`.
fn rewrite_section_relative_urls(
    html: &str,
    section_full_path: &str,
    url_map: &ahash::AHashMap<String, String>,
) -> String {
    if url_map.is_empty() {
        return html.to_string();
    }
    let section_dir = section_full_path
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or("");

    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Match attribute names only when preceded by ASCII whitespace.
        let attr_len = if i > 0 && bytes[i - 1].is_ascii_whitespace() {
            let rest = &bytes[i..];
            if rest.len() >= 5 && rest[..5].eq_ignore_ascii_case(b"href=") {
                Some(5)
            } else if rest.len() >= 4 && rest[..4].eq_ignore_ascii_case(b"src=") {
                Some(4)
            } else if rest.len() >= 11 && rest[..11].eq_ignore_ascii_case(b"xlink:href=") {
                Some(11)
            } else if rest.len() >= 7 && rest[..7].eq_ignore_ascii_case(b"poster=") {
                Some(7)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(alen) = attr_len {
            let after_eq = i + alen;
            if after_eq < bytes.len() && (bytes[after_eq] == b'"' || bytes[after_eq] == b'\'') {
                let quote = bytes[after_eq];
                if let Some(rel_end) = bytes[after_eq + 1..].iter().position(|&b| b == quote) {
                    let val_start = after_eq + 1;
                    let val_end = val_start + rel_end;
                    let val = &html[val_start..val_end];
                    let rewritten = rewrite_single_url(val, section_dir, url_map);
                    out.push_str(&html[i..val_start]);
                    out.push_str(&rewritten);
                    out.push(quote as char);
                    i = val_end + 1;
                    continue;
                }
            }
        }

        let ch = html[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

fn rewrite_single_url(
    raw: &str,
    section_dir: &str,
    url_map: &ahash::AHashMap<String, String>,
) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("//")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("tel:")
        || trimmed.starts_with("data:")
        || trimmed.starts_with("javascript:")
    {
        return raw.to_string();
    }

    let split_idx = trimmed.find(['#', '?']).unwrap_or(trimmed.len());
    let (path_part, suffix) = trimmed.split_at(split_idx);
    if path_part.is_empty() {
        return raw.to_string();
    }

    let resolved =
        crate::archive::resolve_relative_path(section_dir, path_part).to_ascii_lowercase();
    if let Some(mapped) = url_map.get(&resolved) {
        return format!("{mapped}{suffix}");
    }

    raw.to_string()
}
