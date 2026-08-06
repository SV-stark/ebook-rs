use std::collections::HashMap;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Represents an EPUB archive (ZIP container) in memory or from file.
#[derive(Clone)]
pub struct EpubArchive {
    files: HashMap<String, Vec<u8>>,
}

impl EpubArchive {
    /// Open an `EpubArchive` from a filesystem path.
    pub fn open(path: &str) -> Result<Self, String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("Failed to read EPUB file {}: {}", path, e))?;
        Self::from_bytes(&bytes)
    }

    /// Create an empty `EpubArchive` instance.
    pub fn empty() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Retrieve `.opf` package document path from `META-INF/container.xml`.
    pub fn get_opf_path(&self) -> Result<String, String> {
        let container_xml = self.read_string("META-INF/container.xml")?;
        crate::opf::parse_container_xml(&container_xml)
    }

    /// Helper to detect MIME type from entry file extension.
    pub fn get_mime_type(path: &str) -> &'static str {
        let lower = path.to_lowercase();
        if lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm") {
            "application/xhtml+xml"
        } else if lower.ends_with(".css") {
            "text/css"
        } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
            "image/jpeg"
        } else if lower.ends_with(".png") {
            "image/png"
        } else if lower.ends_with(".gif") {
            "image/gif"
        } else if lower.ends_with(".svg") {
            "image/svg+xml"
        } else if lower.ends_with(".webp") {
            "image/webp"
        } else if lower.ends_with(".ttf")
            || lower.ends_with(".otf")
            || lower.ends_with(".woff")
            || lower.ends_with(".woff2")
        {
            "font/otf"
        } else if lower.ends_with(".ncx") {
            "application/x-dtbncx+xml"
        } else {
            "application/octet-stream"
        }
    }

    /// Create an `EpubArchive` from raw ZIP byte data.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let cursor = Cursor::new(bytes);
        let mut zip =
            ZipArchive::new(cursor).map_err(|e| format!("Failed to parse ZIP archive: {}", e))?;
        let mut files = HashMap::new();

        for i in 0..zip.len() {
            let mut file = zip
                .by_index(i)
                .map_err(|e| format!("Failed to read file index {}: {}", i, e))?;
            let name = file.name().to_string();
            // Ignore directories
            if name.ends_with('/') {
                continue;
            }
            let mut content = Vec::new();
            file.read_to_end(&mut content)
                .map_err(|e| format!("Failed to read entry content {}: {}", name, e))?;

            let normalized = normalize_path(&name);
            let lower = normalized.to_lowercase();

            // B1 Fix: Only clone if keys differ to prevent double memory allocation
            if normalized != lower {
                files.insert(lower, content.clone());
            }
            files.insert(normalized, content);
        }

        Ok(Self { files })
    }

    /// Read raw bytes of a file in the archive.
    pub fn read_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        let clean = normalize_path(path);
        if let Some(data) = self.files.get(&clean) {
            return Ok(data.clone());
        }
        // Fallback: try lowercased
        let lower = clean.to_lowercase();
        if let Some(data) = self.files.get(&lower) {
            return Ok(data.clone());
        }
        Err(format!("File not found in archive: {}", path))
    }

    /// Read text content of a file in the archive.
    pub fn read_string(&self, path: &str) -> Result<String, String> {
        let bytes = self.read_bytes(path)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    /// Check if a file exists in the archive.
    pub fn contains(&self, path: &str) -> bool {
        let clean = normalize_path(path);
        self.files.contains_key(&clean) || self.files.contains_key(&clean.to_lowercase())
    }

    /// List all unique file entry paths inside the archive.
    pub fn list_files(&self) -> Vec<String> {
        // P2 Fix: Iterate keys directly without double allocation into a HashSet
        let mut paths: Vec<String> = self.files.keys().cloned().collect();
        paths.sort();
        paths.dedup();
        paths
    }
}

/// Helper function to normalize ZIP entry paths.
pub fn normalize_path(path: &str) -> String {
    let clean = path.replace('\\', "/");
    let mut parts = Vec::new();
    for part in clean.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

/// Helper function to resolve relative paths against a base directory.
pub fn resolve_relative_path(base_dir: &str, relative: &str) -> String {
    let rel_clean = relative.replace('\\', "/");
    if rel_clean.starts_with('/') {
        return normalize_path(&rel_clean);
    }
    let combined = if base_dir.is_empty() {
        rel_clean
    } else {
        format!("{}/{}", base_dir, rel_clean)
    };
    normalize_path(&combined)
}

/// Helper for HTTP Range request generation and byte-slice streaming (`bytes=start-end`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HttpRangeRequest {
    pub url: String,
    pub start: u64,
    pub end: Option<u64>,
}

impl HttpRangeRequest {
    /// Create a new HTTP Range byte slice request.
    pub fn new(url: &str, start: u64, end: Option<u64>) -> Self {
        Self {
            url: url.to_string(),
            start,
            end,
        }
    }

    /// Generate standard HTTP Range header tuple ("Range", "bytes=start-end").
    pub fn to_range_header(&self) -> (String, String) {
        let val = match self.end {
            Some(end_byte) => format!("bytes={}-{}", self.start, end_byte),
            None => format!("bytes={}-", self.start),
        };
        ("Range".to_string(), val)
    }

    /// Parse an incoming HTTP Range header string (e.g. "bytes=0-1024").
    pub fn parse_range_header(header_val: &str) -> Option<(u64, Option<u64>)> {
        let clean = header_val.trim();
        let spec = clean.strip_prefix("bytes=")?;
        let mut parts = spec.split('-');
        let start = parts.next()?.parse::<u64>().ok()?;
        let end = parts.next().and_then(|s| s.parse::<u64>().ok());
        Some((start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_and_resolve() {
        assert_eq!(
            normalize_path("OEBPS/../OEBPS/ch1.xhtml"),
            "OEBPS/ch1.xhtml"
        );
        assert_eq!(
            resolve_relative_path("OEBPS/Text", "../Images/cover.jpg"),
            "OEBPS/Images/cover.jpg"
        );
    }
}
