use std::fmt;

/// Strongly-typed error enum for `ebook-rs` operations and multi-format parsers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EbookError {
    /// File or stream input/output error.
    Io(String),
    /// XML or HTML structural parsing error.
    Xml(String),
    /// Zip or container archive extraction error.
    Zip(String),
    /// Digital Rights Management (DRM / ADEPT / LCP / Mobipocket) restriction.
    DrmProtected(String),
    /// Unsupported or invalid eBook format specifications.
    InvalidFormat(String),
    /// Data integrity or corrupted file record.
    CorruptedData(String),
    /// Missing file, entry, or resource within an eBook archive.
    NotFound(String),
    /// General or custom operation error message.
    Custom(String),
}

impl fmt::Display for EbookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EbookError::Io(msg) => write!(f, "I/O Error: {}", msg),
            EbookError::Xml(msg) => write!(f, "XML Parse Error: {}", msg),
            EbookError::Zip(msg) => write!(f, "Zip Archive Error: {}", msg),
            EbookError::DrmProtected(msg) => write!(f, "DRM Protected: {}", msg),
            EbookError::InvalidFormat(msg) => write!(f, "Invalid Format: {}", msg),
            EbookError::CorruptedData(msg) => write!(f, "Corrupted Data: {}", msg),
            EbookError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            EbookError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for EbookError {}

impl From<std::io::Error> for EbookError {
    fn from(err: std::io::Error) -> Self {
        EbookError::Io(err.to_string())
    }
}

impl From<String> for EbookError {
    fn from(msg: String) -> Self {
        EbookError::Custom(msg)
    }
}

impl From<&str> for EbookError {
    fn from(msg: &str) -> Self {
        EbookError::Custom(msg.to_string())
    }
}

impl From<EbookError> for String {
    fn from(err: EbookError) -> Self {
        err.to_string()
    }
}
