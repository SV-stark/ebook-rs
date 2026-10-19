use crate::error::EbookError;
use ahash::AHashMap;
use parking_lot::Mutex;
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use std::sync::Arc;
use zip::ZipArchive;

/// Maximum aggregate bytes that may be decompressed on demand while an archive is in lazy mode.
const MAX_LAZY_READ_TOTAL: u64 = 1024 * 1024 * 1024;
/// Maximum bytes decompressed for a single on-demand (lazy) archive entry.
const MAX_LAZY_ENTRY_SIZE: u64 = 256 * 1024 * 1024;

type LazyZipSource = Option<Arc<Mutex<ZipArchive<Cursor<Vec<u8>>>>>>;

/// Cumulative decompression budget shared by every format extractor.
///
/// The sizes declared in a ZIP central directory are fully attacker-controlled and may be set
/// far below the real payload, so any limit enforced against a *declared* size can be bypassed
/// (an under-declared total even flips `from_reader` into lazy mode, skipping the eager budget
/// entirely). Limits must therefore be charged against bytes **actually** decompressed.
#[derive(Debug, Clone)]
pub struct DecompressionBudget {
    max_total: u64,
    max_single: u64,
    used_total: u64,
}

impl DecompressionBudget {
    /// Create a budget allowing `max_total` bytes in aggregate and `max_single` per entry.
    #[must_use]
    pub fn new(max_total: u64, max_single: u64) -> Self {
        Self {
            max_total,
            max_single,
            used_total: 0,
        }
    }

    /// Bytes still available in the cumulative budget.
    #[must_use]
    pub fn remaining(&self) -> u64 {
        self.max_total.saturating_sub(self.used_total)
    }

    /// Cap for the next entry: the lesser of the per-entry limit and the remaining budget.
    #[must_use]
    pub fn allowance(&self) -> u64 {
        self.max_single.min(self.remaining())
    }

    /// Charge `n` actually-read bytes against the cumulative budget.
    ///
    /// # Errors
    /// Returns [`EbookError::InvalidFormat`] if this charge would exceed the total budget.
    pub fn charge(&mut self, n: u64) -> Result<(), EbookError> {
        self.used_total = self.used_total.saturating_add(n);
        if self.used_total > self.max_total {
            return Err(EbookError::InvalidFormat(format!(
                "Cumulative decompressed size exceeds the {} byte safety budget (possible decompression bomb)",
                self.max_total
            )));
        }
        Ok(())
    }

    /// Read one entry into `out`, stopping at [`Self::allowance`] and charging what was read.
    ///
    /// Returns the number of bytes read. Truncation at the cap is silent by design (a partial
    /// image is better than an OOM), but exhausting the cumulative budget is an error so the
    /// caller aborts the loop instead of retrying.
    ///
    /// # Errors
    /// Propagates I/O errors, and returns [`EbookError::InvalidFormat`] when the budget is spent.
    pub fn read_capped<R: Read>(
        &mut self,
        r: &mut R,
        out: &mut Vec<u8>,
    ) -> Result<u64, EbookError> {
        if self.remaining() == 0 {
            return Err(EbookError::InvalidFormat(
                "Decompression budget exhausted".to_string(),
            ));
        }
        let cap = self.allowance();
        let before = out.len() as u64;
        r.by_ref()
            .take(cap)
            .read_to_end(out)
            .map_err(|e| EbookError::Io(format!("Failed to decompress archive entry: {}", e)))?;
        let read = (out.len() as u64).saturating_sub(before);
        self.charge(read)?;
        Ok(read)
    }
}

/// Represents an EPUB archive (ZIP container) in memory or from file with lazy decompression for giant archives (>500MB).
#[derive(Clone)]
pub struct EpubArchive {
    files: AHashMap<String, Vec<u8>>,
    lazy_source: LazyZipSource,
    lazy_index: AHashMap<String, usize>,
    /// Running budget for on-demand decompression in lazy mode. Shared across clones so that
    /// `read_bytes(&self)` can charge it; a single 256 MB per-entry cap is not an aggregate cap,
    /// and one section referencing thousands of distinct entries would otherwise decompress
    /// unbounded amounts.
    lazy_budget: Arc<Mutex<DecompressionBudget>>,
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
    #[must_use]
    pub fn empty() -> Self {
        Self {
            files: AHashMap::new(),
            lazy_source: None,
            lazy_index: AHashMap::new(),
            lazy_budget: Arc::new(Mutex::new(DecompressionBudget::new(
                MAX_LAZY_READ_TOTAL,
                MAX_LAZY_ENTRY_SIZE,
            ))),
        }
    }

    /// Insert or update a file entry in the archive.
    pub fn insert(&mut self, path: impl AsRef<str>, data: Vec<u8>) {
        let key = normalize_path(path.as_ref());
        self.files.insert(key, data);
    }

    /// Remove a file entry from the archive.
    pub fn remove(&mut self, path: &str) -> Option<Vec<u8>> {
        let key = normalize_path(path);
        self.files.remove(&key)
    }

    /// Access reference to underlying files map in the archive.
    #[must_use]
    pub fn files(&self) -> &AHashMap<String, Vec<u8>> {
        &self.files
    }

    /// Retrieve `.opf` package document path from `META-INF/container.xml`.
    pub fn get_opf_path(&self) -> Result<String, EbookError> {
        let container_xml = self.read_string("META-INF/container.xml")?;
        crate::opf::parse_container_xml(&container_xml).map_err(EbookError::Xml)
    }

    /// Helper to detect MIME type from entry file extension.
    #[must_use]
    pub fn get_mime_type(path: &str) -> &'static str {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext.eq_ignore_ascii_case("xhtml")
            || ext.eq_ignore_ascii_case("html")
            || ext.eq_ignore_ascii_case("htm")
        {
            "application/xhtml+xml"
        } else if ext.eq_ignore_ascii_case("css") {
            "text/css"
        } else if ext.eq_ignore_ascii_case("png") {
            "image/png"
        } else if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
            "image/jpeg"
        } else if ext.eq_ignore_ascii_case("gif") {
            "image/gif"
        } else if ext.eq_ignore_ascii_case("svg") {
            "image/svg+xml"
        } else if ext.eq_ignore_ascii_case("webp") {
            "image/webp"
        } else if ext.eq_ignore_ascii_case("ttf") {
            "font/ttf"
        } else if ext.eq_ignore_ascii_case("otf") {
            "font/otf"
        } else if ext.eq_ignore_ascii_case("woff") {
            "font/woff"
        } else if ext.eq_ignore_ascii_case("woff2") {
            "font/woff2"
        } else if ext.eq_ignore_ascii_case("js") {
            "application/javascript"
        } else if ext.eq_ignore_ascii_case("json") {
            "application/json"
        } else if ext.eq_ignore_ascii_case("smil") {
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
                let name = zip_entry_name(&file);
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
                lazy_budget: Arc::new(Mutex::new(DecompressionBudget::new(
                    MAX_LAZY_READ_TOTAL,
                    MAX_LAZY_ENTRY_SIZE,
                ))),
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
                let name = zip_entry_name(&file);
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
                lazy_budget: Arc::new(Mutex::new(DecompressionBudget::new(
                    MAX_LAZY_READ_TOTAL,
                    MAX_LAZY_ENTRY_SIZE,
                ))),
            })
        }
    }

    /// Whether archive operates in lazy on-demand decompression mode.
    pub fn is_lazy(&self) -> bool {
        self.lazy_source.is_some()
    }

    /// Read raw bytes of a file in the archive.
    ///
    /// A `#fragment` suffix is stripped for URL-style lookups, but the *full* normalized key is
    /// tried first. `resolve_relative_path` percent-decodes `%23` into a real `#`, so paths that
    /// legitimately contain one (`OEBPS/cov#1.png`) resolve to exactly that and used to have their
    /// tail thrown away by a second strip, making a file that `list_files()` reports unreachable.
    pub fn read_bytes(&self, path: &str) -> Result<Vec<u8>, EbookError> {
        let clean = normalize_path(path);
        let clean_no_frag = clean.split('#').next().unwrap_or(&clean);

        if let Some(data) = self
            .eager_lookup(&clean)
            .or_else(|| self.eager_lookup(clean_no_frag))
        {
            return Ok(data.clone());
        }

        // Check lazy source if in lazy mode
        if let Some(ref lazy_arc) = self.lazy_source {
            let entry_idx = self
                .lazy_index
                .get(&clean)
                .or_else(|| self.lazy_index.get(clean_no_frag))
                .or_else(|| {
                    self.lazy_index
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(&clean))
                        .map(|(_, idx)| idx)
                })
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
                let mut budget = self.lazy_budget.lock();
                let mut data = Vec::new();
                budget.read_capped(&mut file, &mut data)?;
                return Ok(data);
            }
        }

        Err(EbookError::NotFound(format!(
            "File not found in archive: {}",
            path
        )))
    }

    /// Find an eagerly-stored entry by exact key, then case-insensitively.
    ///
    /// `files` is keyed by the *normalized* entry name with any `#` intact, so the full key must
    /// be tried before the fragment-stripped fallback.
    fn eager_lookup(&self, key: &str) -> Option<&Vec<u8>> {
        if let Some(data) = self.files.get(key) {
            return Some(data);
        }
        self.files
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    /// Decompress every lazily-indexed entry into `out`, alongside the eagerly-stored files.
    ///
    /// Used when a lazy archive must be materialized as a whole (e.g. for a full-book export).
    /// Returns the first per-entry error encountered, so a caller can report that the cache would
    /// be incomplete rather than silently losing content.
    pub fn hydrate_into(&self, out: &mut AHashMap<String, Vec<u8>>) -> Result<(), EbookError> {
        out.extend(self.files.iter().map(|(k, v)| (k.clone(), v.clone())));
        let Some(ref lazy_arc) = self.lazy_source else {
            return Ok(());
        };
        let mut zip = lazy_arc.lock();
        for (name, idx) in &self.lazy_index {
            if out.contains_key(name) {
                continue;
            }
            let mut file = zip.by_index(*idx).map_err(|e| {
                EbookError::Zip(format!("Failed to decompress lazy entry {name}: {e}"))
            })?;
            let mut data = Vec::new();
            let mut budget = self.lazy_budget.lock();
            budget.read_capped(&mut file, &mut data)?;
            out.insert(name.clone(), data);
        }
        Ok(())
    }

    /// Zero-copy reference getter for eagerly loaded raw file bytes in the archive.
    ///
    /// Note: this does *not* consult the lazy index, unlike [`Self::read_bytes`] and
    /// [`Self::contains`]. Branch on [`Self::contains`] only when you are prepared for a `NotFound`
    /// here, or use [`Self::read_bytes`] which handles both modes.
    pub fn read_bytes_ref(&self, path: &str) -> Result<&[u8], EbookError> {
        let clean = normalize_path(path);
        if let Some(data) = self.eager_lookup(&clean) {
            return Ok(data.as_slice());
        }
        // Full key first: a `#` in a real path is not a fragment.
        let clean_no_frag = clean.split('#').next().unwrap_or(&clean);
        if let Some(data) = self.eager_lookup(clean_no_frag) {
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
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_be_bytes(*c))
                .collect();
            Ok(String::from_utf16_lossy(&u16_data))
        } else if bytes.starts_with(&[0xFF, 0xFE]) {
            let u16_data: Vec<u16> = bytes[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            Ok(String::from_utf16_lossy(&u16_data))
        } else if bytes.windows(2).any(|w| w == b"<\0" || w == b"\0<") {
            let is_le = bytes.windows(2).any(|w| w == b"<\0");
            let u16_data: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| {
                    if is_le {
                        u16::from_le_bytes(*c)
                    } else {
                        u16::from_be_bytes(*c)
                    }
                })
                .collect();
            Ok(String::from_utf16_lossy(&u16_data))
        } else {
            Ok(crate::dom::decode_bytes_with_encoding(&bytes, None))
        }
    }

    /// Check if a file exists in the archive.
    ///
    /// Mirrors [`Self::read_bytes`]: the full normalized key (which may legitimately contain a
    /// `#` after `resolve_relative_path` decodes `%23`) is checked before the fragment-stripped
    /// form, so a listed entry is never reported missing.
    pub fn contains(&self, path: &str) -> bool {
        let clean = normalize_path(path);
        if self.contains_exact(&clean) {
            return true;
        }
        let clean_no_frag = clean.split('#').next().unwrap_or(&clean);
        clean_no_frag != clean && self.contains_exact(clean_no_frag)
    }

    /// Existence check against a single already-normalized key.
    fn contains_exact(&self, key: &str) -> bool {
        self.files.contains_key(key)
            || self.files.keys().any(|k| k.eq_ignore_ascii_case(key))
            || self.lazy_index.contains_key(key)
            || self.lazy_index.keys().any(|k| k.eq_ignore_ascii_case(key))
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
///
/// `..` segments are resolved lexically and are clamped at the root: an entry named
/// `../../../etc/passwd` normalizes to `etc/passwd` rather than escaping. Note that this is
/// normalization, not validation — a path that escapes still produces an in-memory key, so
/// callers that derive keys from untrusted input (FB2 `<binary id>`, ODT/DOCT entry names) must
/// reject non-plain filenames before calling `insert`.
pub fn normalize_path(path: &str) -> String {
    let clean = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
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

/// Helper function to decode a ZIP entry name without ever failing.
///
/// `zip` 9 made [`zip::read::ZipFile::name`] fallible: it yields the raw bytes when they are
/// valid UTF-8 and otherwise attempts a CP437 decode, which itself fails for bytes that are
/// neither. `zip` 7 always produced a name (lossy UTF-8 when the UTF-8 flag was set, a CP437
/// mapping otherwise), so callers were able to rely on getting a usable string back and an
/// entry could never vanish just because of its name. Decoding the raw bytes lossily on the
/// error path preserves that guarantee instead of dropping the entry silently.
pub fn zip_entry_name<R: std::io::Read + std::io::Seek + ?Sized>(
    file: &zip::read::ZipFile<'_, R>,
) -> String {
    match file.name() {
        Ok(name) => name.into_owned(),
        Err(_) => String::from_utf8_lossy(file.name_raw()).into_owned(),
    }
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
