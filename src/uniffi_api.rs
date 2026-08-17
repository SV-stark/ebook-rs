use crate::book::Book;
use crate::optimizer::{EpubOptimizer, EpubOptimizerOptions};
use crate::paginator::{ReflowPaginator, WritingMode};
use parking_lot::RwLock;
use std::sync::Arc;


/// Search match record for UniFFI Swift/Kotlin clients.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct UniSearchResult {
    pub spine_index: u32,
    pub section_href: String,
    pub cfi: String,
    pub snippet: String,
    pub char_offset: u32,
}

/// Section information record for mobile native views.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct UniSectionSummary {
    pub index: u32,
    pub href: String,
    pub char_count: u32,
    pub has_images: bool,
}

/// High-level, thread-safe Mozilla UniFFI export object for iOS (Swift) and Android (Kotlin).
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct UniBook {
    inner: Arc<RwLock<Book>>,
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
impl UniBook {
    /// Open an eBook from a filesystem path (EPUB, MOBI, AZW3, FB2, KFX, PDF, CBZ, ODT, TXT, MD).
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    pub fn open(path: String) -> Result<Arc<Self>, String> {
        let book = Book::from_file(&path).map_err(|e| format!("Failed to open eBook: {}", e))?;
        Ok(Arc::new(Self {
            inner: Arc::new(RwLock::new(book)),
        }))
    }

    /// Load an eBook from raw memory bytes.
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Arc<Self>, String> {
        let book =
            Book::from_bytes(&bytes).map_err(|e| format!("Failed to parse eBook bytes: {}", e))?;
        Ok(Arc::new(Self {
            inner: Arc::new(RwLock::new(book)),
        }))
    }

    /// Get publication title.
    pub fn get_title(&self) -> String {
        let book = self.inner.read();
        book.metadata().title.clone()
    }

    /// Get publication authors / creators.
    pub fn get_authors(&self) -> Vec<String> {
        let book = self.inner.read();
        book.metadata().creators.clone()
    }

    /// Get primary publication language code.
    pub fn get_language(&self) -> String {
        let book = self.inner.read();
        book.metadata().language().to_string()
    }

    /// Get total number of readable content sections.
    pub fn get_sections_count(&self) -> u32 {
        let book = self.inner.read();
        book.sections.len() as u32
    }

    /// Get processed HTML content for a specific section by zero-based index.
    pub fn get_section_html(&self, index: u32) -> Result<String, String> {
        let book = self.inner.read();
        book.get_section(index as usize)
            .map(|s| s.processed_html.clone())
            .map_err(|e| e.to_string())
    }

    /// Get plain text content for a specific section by zero-based index.
    pub fn get_section_plain_text(&self, index: u32) -> Result<String, String> {
        let book = self.inner.read();
        book.sections
            .get(index as usize)
            .map(|s| s.plain_text.clone())
            .ok_or_else(|| format!("Section index {} out of bounds", index))
    }

    /// Search across all book sections for query matches with Readium CFIs.
    pub fn search(&self, query: String, case_sensitive: bool) -> Vec<UniSearchResult> {
        let book = self.inner.read();
        let results = crate::search::SearchEngine::search(&book.sections, &query, case_sensitive);
        results
            .into_iter()
            .map(|r| {
                let href = book
                    .sections
                    .get(r.spine_index)
                    .map(|s| s.href.clone())
                    .unwrap_or_default();
                UniSearchResult {
                    spine_index: r.spine_index as u32,
                    section_href: href,
                    cfi: r.cfi,
                    snippet: r.snippet,
                    char_offset: r.char_offset as u32,
                }
            })
            .collect()
    }

    /// Get Table of Contents formatted as a JSON string.
    pub fn get_toc_json(&self) -> Result<String, String> {
        let book = self.inner.read();
        serde_json::to_string_pretty(&book.toc)
            .map_err(|e| format!("Failed to serialize TOC: {}", e))
    }

    /// Get complete publication metadata formatted as a JSON string.
    pub fn get_metadata_json(&self) -> Result<String, String> {
        let book = self.inner.read();
        serde_json::to_string_pretty(book.metadata())
            .map_err(|e| format!("Failed to serialize metadata: {}", e))
    }

    /// Calculate virtual reflow page breaks for a specific section.
    pub fn paginate_section(
        &self,
        section_index: u32,
        font_size_px: u32,
        viewport_width_px: u32,
        viewport_height_px: u32,
        is_vertical: bool,
    ) -> Result<String, String> {
        let book = self.inner.read();
        let section = book
            .sections
            .get(section_index as usize)
            .ok_or_else(|| format!("Section index {} out of bounds", section_index))?;

        let mut paginator =
            ReflowPaginator::new(font_size_px, 1.6, viewport_width_px, viewport_height_px, 24);
        if is_vertical {
            paginator = paginator.with_writing_mode(WritingMode::VerticalRl);
        }

        let map = paginator.paginate_section(section);
        serde_json::to_string(&map).map_err(|e| format!("Failed to serialize page map: {}", e))
    }

    /// Export loaded eBook as a universal standard EPUB 3 ZIP archive buffer.
    pub fn export_epub3(&self) -> Result<Vec<u8>, String> {
        let book = self.inner.read();
        book.export_epub3_bytes()
    }

    /// Export loaded eBook as a minified, asset-deduplicated, CSS-purged EPUB 3 archive.
    pub fn export_optimized_epub3(&self) -> Result<Vec<u8>, String> {
        let mut cloned = self.inner.read().clone();
        let options = EpubOptimizerOptions::default();
        EpubOptimizer::optimize(&mut cloned, &options);
        cloned.export_epub3_bytes()
    }
}
