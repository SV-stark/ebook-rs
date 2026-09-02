use crate::error::EbookError;
use ahash::AHashMap;
use parking_lot::Mutex;
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use std::sync::Arc;
use zip::ZipArchive;

type LazyZipSource = Option<Arc<Mutex<ZipArchive<Cursor<Vec<u8>>>>>>;

/// Represents an EPUB archive (ZIP container) in memory or from file with lazy decompression for giant archives (>500MB).
#[derive(Clone)]
pub struct EpubArchive {
    files: AHashMap<String, Vec<u8>>,
    lazy_source: LazyZipSource,
    lazy_index: AHashMap<String, usize>,
}

impl EpubArchive {
    /// Open an `EpubArchive` from a filesystem path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, EbookError> {
        let path_ref = path.as_ref();
        let file = std::fs::File::open(path_ref).map_err(|e| {
            EbookError::Io(format!(
                "Failed to open EPUB file {}: {}",
                path_ref.display(),
                e
            ))
        })?;
        Self::from_reader(file)
    }

    /// Create an empty `EpubArchive` instance.
    pub fn empty() -> Self {
        Self {
            files: AHashMap::new(),
            lazy_source: None,
            lazy_index: AHashMap::new(),
        }
    }

    /// Insert or update a file entry in the archive.
    pub fn insert(&mut self, path: impl Into<String>, data: Vec<u8>) {
        let key = normalize_path(&path.into());
        self.files.insert(key, data);
    }

    /// Remove a file entry from the archive.
    pub fn remove(&mut self, path: &str) -> Option<Vec<u8>> {
        let key = normalize_path(path);
        self.files.remove(&key)
    }

    /// Access reference to underlying files map in the archive.
    pub fn files(&self) -> &AHashMap<String, Vec<u8>> {
        &self.files
    }

    /// Retrieve `.opf` package document path from `META-INF/container.xml`.
    pub fn get_opf_path(&self) -> Result<String, EbookError> {
        let container_xml = self.read_string("META-INF/container.xml")?;
        crate::opf::parse_container_xml(&container_xml).map_err(EbookError::Xml)
    }

    /// Helper to detect MIME type from entry file extension.
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
        } else if lower.ends_with(".ttf") {
            "font/ttf"
        } else if lower.ends_with(".otf") {
            "font/otf"
        } else if lower.ends_with(".woff") {
            "font/woff"
        } else if lower.ends_with(".woff2") {
            "font/woff2"
        } else if lower.ends_with(".js") {
            "application/javascript"
        } else if lower.ends_with(".json") {
            "application/json"
        } else if lower.ends_with(".smil") {
            "application/smil+xml"
        } else {
            "application/octet-stream"
        }
    }

    /// Create an `EpubArchive` from raw ZIP byte data with Zip Bomb protection and lazy decompression.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EbookError> {
        Self::from_reader(Cursor::new(bytes))
    }

    /// Construct `EpubArchive` from any `Read + Seek` source.
    /// If total uncompressed size exceeds 500MB, seamlessly switches to lazy on-demand decompression mode.
    pub fn from_reader<R: Read + Seek>(mut reader: R) -> Result<Self, EbookError> {
        let mut raw_bytes = Vec::new();
        reader
            .seek(std::io::SeekFrom::Start(0))
            .map_err(|e| EbookError::Io(format!("Failed to seek reader: {}", e)))?;
        reader
            .read_to_end(&mut raw_bytes)
            .map_err(|e| EbookError::Io(format!("Failed to read archive bytes: {}", e)))?;

        let compressed_len = raw_bytes.len() as u64;
        let mut zip = ZipArchive::new(Cursor::new(raw_bytes))
            .map_err(|e| EbookError::Zip(format!("Failed to parse ZIP archive: {}", e)))?;

        let entry_count = zip.len();
        const MAX_ZIP_ENTRIES: usize = 50_000;
        if entry_count > MAX_ZIP_ENTRIES {
            return Err(EbookError::InvalidFormat(format!(
                "ZIP archive exceeds maximum entry limit ({} > {})",
                entry_count, MAX_ZIP_ENTRIES
            )));
        }

        let mut total_uncompressed_estimate: u64 = 0;
        for i in 0..entry_count {
            if let Ok(file) = zip.by_index_raw(i) {
                total_uncompressed_estimate =
                    total_uncompressed_estimate.saturating_add(file.size());
            }
        }

        // Decompression ratio protection (e.g. 100:1 ratio check against zip-bombs)
        const MAX_DECOMPRESSION_RATIO: u64 = 100;
        if compressed_len > 0 && total_uncompressed_estimate > 20 * 1024 * 1024 {
            if total_uncompressed_estimate / compressed_len > MAX_DECOMPRESSION_RATIO {
                return Err(EbookError::InvalidFormat(
                    "Zip bomb detected: uncompressed ratio exceeds 100:1 safety limit".to_string(),
                ));
            }
        }

        const MAX_EAGER_TOTAL_SIZE: u64 = 256 * 1024 * 1024; // 256 MB threshold for eager loading
        let is_giant_archive = total_uncompressed_estimate > MAX_EAGER_TOTAL_SIZE;

        let mut files = AHashMap::new();
        let mut lazy_index = AHashMap::new();

        if is_giant_archive {
            // Lazy Mode: Index all entries and eagerly load only structural XML documents with strict cumulative safety cap
            let mut cumulative_metadata_size: usize = 0;
            const MAX_LAZY_METADATA_BUDGET: usize = 64 * 1024 * 1024; // 64 MB total across all metadata XMLs
            const MAX_SINGLE_XML_ENTRY: u64 = 16 * 1024 * 1024; // 16 MB max per metadata file

            for i in 0..entry_count {
                let mut file = zip.by_index(i).map_err(|e| {
                    EbookError::Zip(format!("Failed to read entry index {}: {}", i, e))
                })?;
                let name = file.name().to_string();
                if name.ends_with('/') {
                    continue;
                }
                let norm = normalize_path(&name);
                lazy_index.insert(norm.clone(), i);

                // Eagerly parse critical metadata files only
                let lower = norm.to_lowercase();
                if lower.ends_with(".xml")
                    || lower.ends_with(".opf")
                    || lower.ends_with(".ncx")
                    || lower.contains("container.xml")
                {
                    let mut content = Vec::new();
                    if file
                        .by_ref()
                        .take(MAX_SINGLE_XML_ENTRY)
                        .read_to_end(&mut content)
                        .is_ok()
                    {
                        cumulative_metadata_size =
                            cumulative_metadata_size.saturating_add(content.len());
                        if cumulative_metadata_size > MAX_LAZY_METADATA_BUDGET {
                            return Err(EbookError::InvalidFormat(
                                "Archive metadata exceeds aggregate safety budget (possible decompression bomb)".to_string(),
                            ));
                        }
                        files.insert(norm, content);
                    }
                }
            }

            Ok(Self {
                files,
                lazy_source: Some(Arc::new(Mutex::new(zip))),
                lazy_index,
            })
        } else {
            // Eager Mode: Load and decompress everything into memory with safe budget
            let mut total_decompressed: usize = 0;
            const MAX_EAGER_BUDGET: usize = 300 * 1024 * 1024;
            const MAX_SINGLE_FILE_SIZE: u64 = 64 * 1024 * 1024;

            for i in 0..entry_count {
                let mut file = zip.by_index(i).map_err(|e| {
                    EbookError::Zip(format!("Failed to read file index {}: {}", i, e))
                })?;
                let name = file.name().to_string();
                if name.ends_with('/') {
                    continue;
                }

                let mut content = Vec::new();
                file.by_ref()
                    .take(MAX_SINGLE_FILE_SIZE)
                    .read_to_end(&mut content)
                    .map_err(|e| {
                        EbookError::Io(format!("Failed to read entry content {}: {}", name, e))
                    })?;

                total_decompressed = total_decompressed.saturating_add(content.len());
                if total_decompressed > MAX_EAGER_BUDGET {
                    return Err(EbookError::InvalidFormat(
                        "Cumulative uncompressed archive size exceeds memory safety limit"
                            .to_string(),
                    ));
                }

                let normalized = normalize_path(&name);
                files.insert(normalized, content);
            }

            Ok(Self {
                files,
                lazy_source: None,
                lazy_index,
            })
        }
    }

    /// Whether archive operates in lazy on-demand decompression mode.
    pub fn is_lazy(&self) -> bool {
        self.lazy_source.is_some()
    }

    /// Read raw bytes of a file in the archive.
    pub fn read_bytes(&self, path: &str) -> Result<Vec<u8>, EbookError> {
        let clean = normalize_path(path);
        let clean_no_frag = clean.split('#').next().unwrap_or(&clean);

        if let Some(data) = self.files.get(clean_no_frag) {
            return Ok(data.clone());
        }

        if let Some((_, data)) = self
            .files
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(clean_no_frag))
        {
            return Ok(data.clone());
        }

        // Check lazy source if in lazy mode
        if let Some(ref lazy_arc) = self.lazy_source {
            let entry_idx = self
                .lazy_index
                .get(clean_no_frag)
                .or_else(|| {
                    self.lazy_index
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(clean_no_frag))
                        .map(|(_, idx)| idx)
                })
                .copied();

            if let Some(idx) = entry_idx {
                let mut zip = lazy_arc.lock();
                let mut file = zip.by_index(idx).map_err(|e| {
                    EbookError::Zip(format!("Failed to decompress lazy entry {}: {}", path, e))
                })?;
                const MAX_LAZY_ENTRY_SIZE: u64 = 256 * 1024 * 1024; // 256 MB per entry limit
                let alloc_capacity = (file.size() as usize).min(32 * 1024 * 1024);
                let mut data = Vec::with_capacity(alloc_capacity);
                file.by_ref()
                    .take(MAX_LAZY_ENTRY_SIZE)
                    .read_to_end(&mut data)
                    .map_err(|e| {
                        EbookError::Io(format!("Failed to read lazy entry content: {}", e))
                    })?;
                return Ok(data);
            }
        }

        Err(EbookError::NotFound(format!(
            "File not found in archive: {}",
            path
        )))
    }

    /// Zero-copy reference getter for eagerly loaded raw file bytes in the archive.
    pub fn read_bytes_ref(&self, path: &str) -> Result<&[u8], EbookError> {
        let clean = normalize_path(path);
        let clean_no_frag = clean.split('#').next().unwrap_or(&clean);
        if let Some(data) = self.files.get(clean_no_frag) {
            return Ok(data.as_slice());
        }
        if let Some((_, data)) = self
            .files
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(clean_no_frag))
        {
            return Ok(data.as_slice());
        }
        Err(EbookError::NotFound(format!(
            "File not found in eager archive buffer: {}",
            path
        )))
    }

    /// Read text content of a file in the archive with SIMD UTF-8 / UTF-16 / legacy decoding.
    pub fn read_string(&self, path: &str) -> Result<String, EbookError> {
        let bytes = self.read_bytes(path)?;
        if let Ok(s) = simdutf8::basic::from_utf8(&bytes) {
            Ok(s.to_string())
        } else if bytes.starts_with(&[0xFE, 0xFF]) {
            let u16_data: Vec<u16> = bytes[2..]
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&u16_data))
        } else if bytes.starts_with(&[0xFF, 0xFE]) {
            let u16_data: Vec<u16> = bytes[2..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&u16_data))
        } else if bytes.windows(2).any(|w| w == b"<\0" || w == b"\0<") {
            let is_le = bytes.windows(2).any(|w| w == b"<\0");
            let u16_data: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| {
                    if is_le {
                        u16::from_le_bytes([c[0], c[1]])
                    } else {
                        u16::from_be_bytes([c[0], c[1]])
                    }
                })
                .collect();
            Ok(String::from_utf16_lossy(&u16_data))
        } else {
            Ok(crate::dom::decode_bytes_with_encoding(&bytes, None))
        }
    }

    /// Check if a file exists in the archive.
    pub fn contains(&self, path: &str) -> bool {
        let clean = normalize_path(path);
        let clean_no_frag = clean.split('#').next().unwrap_or(&clean);
        self.files.contains_key(clean_no_frag)
            || self
                .files
                .keys()
                .any(|k| k.eq_ignore_ascii_case(clean_no_frag))
            || self.lazy_index.contains_key(clean_no_frag)
            || self
                .lazy_index
                .keys()
                .any(|k| k.eq_ignore_ascii_case(clean_no_frag))
    }

    /// List all unique file entry paths inside the archive.
    pub fn list_files(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.files.keys().cloned().collect();
        paths.extend(self.lazy_index.keys().cloned());
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
    // 1. Strip URL fragment identifier (#fragment)
    let rel_no_frag = relative.split('#').next().unwrap_or(relative);
    // 1b. Strip URL query string (?v=2, ?cache=...) — CSS versioned hrefs
    let rel_no_query = rel_no_frag.split('?').next().unwrap_or(rel_no_frag);
    // 2. Percent-decode URI path (%20, non-ASCII)
    let decoded = percent_encoding::percent_decode_str(rel_no_query)
        .decode_utf8_lossy()
        .to_string();

    let rel_clean = decoded.replace('\\', "/");
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
