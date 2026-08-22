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
    pub fn get_section(&self, index: usize) -> Result<&Section, EbookError>;
    pub fn get_all_sections_hydrated(&self) -> Vec<Section>;
    pub fn search(&self, query: &str) -> Vec<SearchResult>;
    pub fn to_rag_chunks(&self, config: &RagChunkConfig) -> Vec<RagChunk>;
    pub fn to_webpub_manifest(&self) -> Result<WebPubManifest, EbookError>;
    pub fn export_zstd_cache(&self) -> Result<Vec<u8>, EbookError>;
    pub fn from_zstd_cache(cache: &[u8]) -> Result<Self, EbookError>;
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
```\n