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
