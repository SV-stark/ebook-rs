#![cfg(feature = "python")]

use crate::UniversalEpub3Exporter;
use crate::book::Book;
use crate::kfx::writer::UniversalKfxExporter;
use crate::rag::RagChunkConfig;
use crate::section::Section;
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;

/// Python representation of an eBook Chapter Section.
#[pyclass(name = "Section")]
#[derive(Clone)]
pub struct PySection {
    inner: Section,
}

#[pymethods]
impl PySection {
    /// Spine index of the section.
    #[getter]
    pub fn index(&self) -> usize {
        self.inner.index
    }

    /// Relative manifest href path.
    #[getter]
    pub fn href(&self) -> String {
        self.inner.href.clone()
    }

    /// Full path within the eBook archive.
    #[getter]
    pub fn full_path(&self) -> String {
        self.inner.full_path.clone()
    }

    /// Raw section HTML content before rendering asset inlining.
    #[getter]
    pub fn raw_html(&self) -> String {
        self.inner.raw_html.clone()
    }

    /// Rendered section HTML content with inlined images and styles.
    #[getter]
    pub fn processed_html(&self) -> String {
        self.inner.processed_html.clone()
    }

    /// Clean plain text content without HTML tags.
    #[getter]
    pub fn plain_text(&self) -> String {
        self.inner.plain_text.clone()
    }
}

/// Python representation of an eBook document.
#[pyclass(name = "Book")]
pub struct PyBook {
    inner: Book,
}

#[pymethods]
impl PyBook {
    /// Open an eBook from a local file path.
    ///
    /// Supports EPUB 2/3, KFX, MOBI, AZW3, FB2, LIT, CBZ, PDF, ODT, TXT, MD.
    #[staticmethod]
    pub fn open(path: &str) -> PyResult<Self> {
        Book::from_file(path)
            .map(|book| PyBook { inner: book })
            .map_err(|e| PyValueError::new_err(e))
    }

    /// Load an eBook from an in-memory byte slice.
    #[staticmethod]
    pub fn from_bytes(bytes: &[u8]) -> PyResult<Self> {
        Book::from_bytes(bytes)
            .map(|book| PyBook { inner: book })
            .map_err(|e| PyValueError::new_err(e))
    }

    /// Publication title.
    #[getter]
    pub fn title(&self) -> String {
        self.inner.metadata().title.clone()
    }

    /// Authors / creators list.
    #[getter]
    pub fn authors(&self) -> Vec<String> {
        self.inner.metadata().creators.clone()
    }

    /// Languages list (BCP-47 tags).
    #[getter]
    pub fn languages(&self) -> Vec<String> {
        self.inner.metadata().languages.clone()
    }

    /// Description / summary of the book.
    #[getter]
    pub fn description(&self) -> Option<String> {
        self.inner.metadata().description.clone()
    }

    /// Total number of readable chapter sections.
    #[getter]
    pub fn section_count(&self) -> usize {
        self.inner.sections.len()
    }

    /// Retrieve processed Section object by 0-based spine index.
    pub fn get_section(&mut self, index: usize) -> PyResult<PySection> {
        self.inner
            .get_section(index)
            .map(|section| PySection { inner: section })
            .map_err(|e| PyValueError::new_err(e))
    }

    /// Retrieve rendered HTML section content by 0-based spine index.
    pub fn get_section_html(&mut self, index: usize) -> PyResult<String> {
        self.inner
            .get_section(index)
            .map(|section| section.processed_html)
            .map_err(|e| PyValueError::new_err(e))
    }

    /// Retrieve Section by manifest href path.
    pub fn get_section_by_href(&mut self, href: &str) -> PyResult<PySection> {
        let index = self
            .inner
            .sections
            .iter()
            .position(|s| s.href == href || s.full_path == href)
            .ok_or_else(|| PyKeyError::new_err(format!("Section href not found: {}", href)))?;
        self.get_section(index)
    }

    /// Generate AI / RAG document chunks with CFI citation anchors as JSON.
    #[pyo3(signature = (max_tokens=None, overlap_tokens=None))]
    pub fn to_rag_chunks_json(
        &self,
        max_tokens: Option<usize>,
        overlap_tokens: Option<usize>,
    ) -> PyResult<String> {
        let config = RagChunkConfig {
            max_tokens: max_tokens.unwrap_or(512),
            overlap_tokens: overlap_tokens.unwrap_or(64),
            preserve_headings: true,
            ..Default::default()
        };
        let chunks = self.inner.to_rag_chunks(&config);
        serde_json::to_string(&chunks).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Export publication as Readium WebPub Manifest JSON.
    pub fn to_webpub_manifest_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.to_webpub_manifest())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Export publication metadata as JSON.
    pub fn metadata_json(&self) -> PyResult<String> {
        serde_json::to_string(self.inner.metadata())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Export publication Table of Contents as JSON.
    pub fn toc_json(&self) -> PyResult<String> {
        serde_json::to_string(self.inner.toc()).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Export book as a W3C-valid EPUB 3 ZIP archive buffer.
    pub fn export_epub3_bytes(&self) -> PyResult<Vec<u8>> {
        UniversalEpub3Exporter::export(&self.inner).map_err(|e| PyRuntimeError::new_err(e))
    }

    /// Export book as an Amazon KFX binary container buffer.
    pub fn export_kfx_bytes(&self) -> PyResult<Vec<u8>> {
        UniversalKfxExporter::export(&self.inner).map_err(|e| PyRuntimeError::new_err(e))
    }

    /// Enable 2-Page Manga Spread mode (Right-to-Left reading progression).
    pub fn enable_manga_mode(&mut self) {
        crate::cbz::CbzBook::enable_manga_mode(&mut self.inner);
    }

    /// Pre-fetch adjacent comic page images into memory for zero-latency rendering.
    #[pyo3(signature = (current_index=0, window=3))]
    pub fn prefetch_comic_pages(
        &self,
        current_index: usize,
        window: usize,
    ) -> Vec<(usize, String, Vec<u8>)> {
        crate::cbz::CbzBook::prefetch_page_images(&self.inner, current_index, window)
    }

    /// Search full-text content of the book for a query string.
    pub fn search(&self, query: &str) -> PyResult<String> {
        let results = self.inner.search(query);
        serde_json::to_string(&results).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }
}

/// PyO3 Module entry point for Python `import ebook_rs`.
#[pymodule]
fn ebook_rs(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySection>()?;
    m.add_class::<PyBook>()?;
    Ok(())
}
