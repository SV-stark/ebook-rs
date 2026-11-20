---
title: Rust API Reference
description: Essential types, structs, and function signatures in ebook-rs.
---

### `Book` (src/book.rs)
The primary entry point for loading and querying eBooks.

```rust
pub struct Book {
    pub metadata: Metadata,
    pub spine: Vec<SpineItem>,
    pub sections: Vec<Section>,
    pub toc: Vec<NavPoint>,
    pub locations: BookLocations,
    pub archive: EpubArchive,
}

impl Book {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, EbookError>;
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EbookError>;
    pub fn from_bytes_with_title(bytes: &[u8], title: &str) -> Result<Self, EbookError>;
    pub fn from_bytes_with_passphrase(bytes: &[u8], title: &str, passphrase: &str) -> Result<Self, EbookError>;
    pub fn get_section(&self, index: usize) -> Result<&Section, EbookError>;
    pub fn get_section_by_cfi(&self, cfi: &str) -> Result<Section, String>;
    pub fn get_section_and_offset_by_cfi(&self, cfi: &str) -> Result<(Section, usize), String>;
    pub fn resolve_cfi(&self, cfi: &str, html: &str) -> Result<(Section, usize, CfiDomTarget), String>;
    pub fn get_all_sections_hydrated(&self) -> Vec<Section>;
    pub fn get_merged_stylesheet(&self) -> String;
    pub fn search(&self, query: &str) -> Vec<SearchResult>;
    pub fn to_rag_chunks(&self, config: &RagChunkConfig) -> Vec<RagChunk>;
    pub fn to_webpub_manifest(&self) -> Result<WebPubManifest, EbookError>;
    pub fn export_zstd_cache(&self) -> Result<Vec<u8>, EbookError>;
    pub fn from_zstd_cache(cache: &[u8]) -> Result<Self, EbookError>;

    // Unified 27-Format Pure-Rust Exporter API (v0.19.0)
    pub fn supported_output_formats() -> &'static [&'static str];
    pub fn export_format_bytes(&self, format: &str) -> Result<Vec<u8>, EbookError>;
    // Plus typed helpers: export_kepub_bytes, export_mobi_bytes, export_azw3_bytes,
    // export_fb2_bytes, export_pdf_bytes, export_docx_bytes, export_odt_bytes,
    // export_rtf_bytes, export_pptx_bytes, export_html_string, export_htmlz_bytes,
    // export_txt_string, export_txtz_bytes, export_markdown_string, export_oeb_zip_bytes,
    // export_oeb_dir, export_zip_bytes, export_pdb_bytes, export_pmlz_bytes,
    // export_rb_bytes, export_tcr_bytes, export_typst_string, export_adoc_string,
    // export_org_string, export_rst_string, export_tex_string.
}
```

### `UniversalEpub3Exporter` (src/validator.rs)
Export any parsed `Book` to standard EPUB 3.

```rust
pub struct UniversalEpub3Exporter;

impl UniversalEpub3Exporter {
    pub fn export(book: &Book) -> Result<Vec<u8>, String>;
}
```

### `SearchEngine` (src/search.rs)
Zero-allocation full-text and regex search.

```rust
pub struct SearchEngine;

impl SearchEngine {
    pub fn search(book: &Book, query: &str) -> Vec<SearchResult>;
    pub fn regex_search(book: &Book, pattern: &str) -> Result<Vec<SearchResult>, String>;
}
```
