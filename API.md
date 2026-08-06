# 📚 EBook-RS API Reference & Complete Documentation (v0.10.5)

`ebook-rs` (v0.10.5) is a multi-format pure Rust eBook engine supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT (OpenDocument Text)**, **Plain Text (.txt)**, and **Markdown (.md)** formats with **SIMD Performance Accelerators (`compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot`)**, **Universal EPUB 3 Exporter**, **Zero-Copy mmap**, **Lightweight DOM AST Tree**, **Fuzzy XML Recovery**, **Synthetic FXL Spreads**, **TOC Deep Search**, **Tree-sitter Code Parser**, **Regex Search**, **Structural EPUB Validator**, **Book Fingerprinting & Deduplication**, **Academic Citation Exporter**, and **Readium LCP/Locator** support.

---

## 📋 Table of Contents
1. [Multi-Format Book Core API (`ebook_rs::Book`)](#1-multi-format-book-core-api-ebook_rsbook)
2. [Performance Accelerators (`compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot`)](#2-performance-accelerators-compact_str-ahash-simdutf8-zlib-rs-memchr-parking_lot)
3. [Supported Formats Matrix](#3-supported-formats-matrix)
4. [Format-Specific Parsers (`MobiBook`, `Fb2Book`, `LitBook`, `CbzBook`, `OdtBook`, `TxtBook`, `PdfBook`)](#4-format-specific-parsers)
5. [EPUB 3 Accessibility Metadata (`AccessibilityMetadata`)](#5-epub-3-accessibility-metadata-accessibilitymetadata)
6. [EPUB 3 Media Overlays (SMIL Audio Sync) (`MediaOverlayPackage`)](#6-epub-3-media-overlays-smil-audio-sync-mediaoverlaypackage)
7. [Section Module (`ebook_rs::Section`)](#7-section-module-ebook_rssection)
8. [EPUB CFI Engine (`ebook_rs::Cfi`)](#8-epub-cfi-engine-ebook_rscfi)
9. [Locations Progress Engine (`ebook_rs::Locations`)](#9-locations-progress-engine-ebook_rslocations)
10. [Annotations Manager (`ebook_rs::AnnotationManager`)](#10-annotations-manager-ebook_rsannotationmanager)
11. [Full-Text & Regex Search Engine (`ebook_rs::SearchEngine`)](#11-full-text--regex-search-engine-ebook_rssearchengine)
12. [Structural EPUB Validator (`ebook_rs::EpubValidator`)](#12-structural-epub-validator-ebook_rsepubvalidator)
13. [Book Fingerprinting & Deduplication (`ebook_rs::BookFingerprint`)](#13-book-fingerprinting--deduplication-ebook_rsbookfingerprint)
14. [Academic Citation Exporter (`ebook_rs::CitationExporter`)](#14-academic-citation-exporter-ebook_rscitationexporter)
15. [Tree-sitter Concrete Syntax Tree Engine (`ebook_rs::TreeSitterEngine`)](#15-tree-sitter-concrete-syntax-tree-engine-ebook_rstreesitterengine)
16. [Synthetic FXL 2-Page Spreads (`SyntheticSpread`)](#16-synthetic-fxl-2-page-spreads-syntheticspread)
17. [Table of Contents Deep Search & Flattening (`NavPoint::search`, `NavPoint::flatten`)](#17-table-of-contents-deep-search--flattening-navpointsearch-navpointflatten)
18. [Universal EPUB 3 Exporter (`book.export_epub3_bytes()`)](#18-universal-epub-3-exporter-bookexport_epub3_bytes)
19. [Zero-Copy Memory-Mapped I/O (`Book::from_mmap`)](#19-zero-copy-memory-mapped-io-bookfrom_mmap)
20. [Lightweight DOM AST Engine (`EbookDomTree`, `DomNode`)](#20-lightweight-dom-ast-engine-ebookdomtree-domnode)

---

## 1. Multi-Format Book Core API (`ebook_rs::Book`)

```rust
use ebook_rs::Book;

// Auto-detect format and load from path
let book = Book::from_file("path/to/book.epub")?;

// Access metadata
println!("Title: {}", book.metadata().title);
println!("Format: {:?}", book.format());

// Universal EPUB 3 export
let epub3_bytes = book.export_epub3_bytes()?;
```

---

## 2. Performance Accelerators (`compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot`)

`ebook-rs` v0.10.5 integrates 6 SIMD and stack-optimization performance crates:
- **`compact_str`**: Small String Optimization (`CompactString`) storing strings <= 24 bytes directly on the stack to eliminate heap allocations.
- **`ahash`**: `AHashMap` & `AHashSet` for 3x-5x faster hash lookups in OPF manifest, DOM attributes, and annotations.
- **`simdutf8`**: SIMD-accelerated UTF-8 validation (`simdutf8::basic::from_utf8`) for 10x-20x faster HTML/XML byte-stream decoding.
- **`zlib-rs`**: `flate2` with `zlib-rs` SIMD zlib decompression for 3x faster EPUB/CBZ ZIP archive reading.
- **`memchr`**: SIMD substring searching (`memchr::memchr` / `memchr::memmem`) for ultra-fast HTML tag scanning, attribute extraction, and script stripping.
- **`parking_lot`**: Fast 1-byte non-poisoning mutex locks (`parking_lot::Mutex`) for concurrent section caches.

---

## 18. Universal EPUB 3 Exporter (`book.export_epub3_bytes()`)

```rust
let epub3_bytes = book.export_epub3_bytes()?;
std::fs::write("output.epub", &epub3_bytes)?;
```

---

## 19. Zero-Copy Memory-Mapped I/O (`Book::from_mmap`)

```rust
#[cfg(feature = "mmap")]
let book = Book::from_mmap("large_comic.cbz")?;
```

---

## 20. Lightweight DOM AST Engine (`EbookDomTree`, `DomNode`)

```rust
use ebook_rs::EbookDomTree;

let html = "<div><h1>Header</h1><script>alert(1)</script><p>Text</p></div>";
let mut tree = EbookDomTree::parse(html);
tree.strip_elements(&["script"]);
let clean_html = tree.to_html();
```
