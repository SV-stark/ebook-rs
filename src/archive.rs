use std::collections::HashMap;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Represents an EPUB archive (ZIP container) in memory or from file.
#[derive(Clone)]
pub struct EpubArchive {
    files: HashMap<String, Vec<u8>>,
}

impl EpubArchive {
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
            files.insert(normalized, content.clone());
            files.entry(lower).or_insert(content);
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
        String::from_utf8(bytes).map_err(|e| format!("UTF-8 decode error for {}: {}", path, e))
    }

    /// Check if a file exists in the archive.
    pub fn contains(&self, path: &str) -> bool {
        let clean = normalize_path(path);
        self.files.contains_key(&clean) || self.files.contains_key(&clean.to_lowercase())
    }

    /// List all unique file paths in the archive.
    pub fn list_files(&self) -> Vec<String> {
        let mut set = std::collections::HashSet::new();
        for k in self.files.keys() {
            set.insert(k.clone());
        }
        let mut list: Vec<String> = set.into_iter().collect();
        list.sort();
        list
    }

    /// Helper to resolve relative href from base directory.
    pub fn resolve_path(base_dir: &str, relative_path: &str) -> String {
        resolve_relative_path(base_dir, relative_path)
    }

    /// Detect MIME type based on file extension.
    pub fn get_mime_type(path: &str) -> &'static str {
        let lower = path.to_lowercase();
        if lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm") {
            "application/xhtml+xml"
        } else if lower.ends_with(".css") {
            "text/css"
        } else if lower.ends_with(".png") {
            "image/png"
        } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
            "image/jpeg"
        } else if lower.ends_with(".gif") {
            "image/gif"
        } else if lower.ends_with(".svg") {
            "image/svg+xml"
        } else if lower.ends_with(".webp") {
            "image/webp"
        } else if lower.ends_with(".woff") {
            "font/woff"
        } else if lower.ends_with(".woff2") {
            "font/woff2"
        } else if lower.ends_with(".ttf") || lower.ends_with(".otf") {
            "font/ttf"
        } else if lower.ends_with(".ncx") {
            "application/x-dtbncx+xml"
        } else if lower.ends_with(".opf") {
            "application/oebps-package+xml"
        } else if lower.ends_with(".js") {
            "text/javascript"
        } else {
            "application/octet-stream"
        }
    }
}

/// Helper to normalize slash paths (removes `./` and leading slashes).
pub fn normalize_path(path: &str) -> String {
    let p = path.replace('\\', "/");
    let trimmed = p.trim_start_matches('/');
    let mut parts = Vec::new();
    for part in trimmed.split('/') {
        if part == "." || part.is_empty() {
            continue;
        }
        parts.push(part);
    }
    parts.join("/")
}

/// Resolve relative path against a base directory path.
pub fn resolve_relative_path(base_dir: &str, relative_path: &str) -> String {
    // If relative_path starts with http/https or data:, return as is
    if relative_path.starts_with("http://")
        || relative_path.starts_with("https://")
        || relative_path.starts_with("data:")
    {
        return relative_path.to_string();
    }

    // Strip fragment/query for path calculation, but we will preserve fragment if needed
    let clean_rel = relative_path.split('#').next().unwrap_or(relative_path);
    let clean_rel = clean_rel.split('?').next().unwrap_or(clean_rel);

    if clean_rel.starts_with('/') {
        return normalize_path(clean_rel);
    }

    let base_clean = normalize_path(base_dir);
    let mut stack: Vec<&str> = if base_clean.is_empty() {
        Vec::new()
    } else {
        base_clean.split('/').collect()
    };

    for segment in clean_rel.split('/') {
        if segment == "." || segment.is_empty() {
            continue;
        } else if segment == ".." {
            stack.pop();
        } else {
            stack.push(segment);
        }
    }

    stack.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_and_resolve() {
        assert_eq!(normalize_path("./OEBPS//content.opf"), "OEBPS/content.opf");
        assert_eq!(
            resolve_relative_path("OEBPS", "ch1.xhtml"),
            "OEBPS/ch1.xhtml"
        );
        assert_eq!(
            resolve_relative_path("OEBPS/text", "../images/img.png"),
            "OEBPS/images/img.png"
        );
    }
}
