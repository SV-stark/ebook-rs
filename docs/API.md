# 📚 EBook-RS API Reference & Complete Documentation (v0.11.0)

`ebook-rs` (v0.11.0) is a multi-format pure Rust eBook engine supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT (OpenDocument Text)**, **Plain Text (.txt)**, and **Markdown (.md)** formats with **Legacy Non-UTF-8 Charset Decoding**, **Automatic Language Detection**, **Zstd Compressed State Caching**, **Universal EPUB 3 Exporter**, **Zero-Copy mmap**, **Lightweight DOM AST Tree**, **Fuzzy XML Recovery**, **CFI**, and **Readium LCP/Locator** support.

---

## 📋 Table of Contents
1. [Multi-Format Book Core API (`ebook_rs::Book`)](#1-multi-format-book-core-api-ebook_rsbook)
2. [Supported Formats Matrix](#2-supported-formats-matrix)
3. [Format-Specific Parsers (`MobiBook`, `Fb2Book`, `LitBook`, `CbzBook`, `OdtBook`, `TxtBook`, `PdfBook`)](#3-format-specific-parsers)
4. [EPUB 3 Accessibility Metadata (`AccessibilityMetadata`)](#4-epub-3-accessibility-metadata-accessibilitymetadata)
5. [EPUB 3 Media Overlays (SMIL Audio Sync) (`MediaOverlayPackage`)](#5-epub-3-media-overlays-smil-audio-sync-mediaoverlaypackage)
6. [Section Module (`ebook_rs::Section`)](#6-section-module-ebook_rssection)
7. [EPUB CFI Engine (`ebook_rs::Cfi`)](#7-epub-cfi-engine-ebook_rscfi)
8. [Locations Progress Engine (`ebook_rs::Locations`)](#8-locations-progress-engine-ebook_rslocations)
9. [Annotations Manager (`ebook_rs::AnnotationManager`)](#9-annotations-manager-ebook_rsannotationmanager)
10. [Full-Text & Regex Search Engine (`ebook_rs::SearchEngine`)](#10-full-text--regex-search-engine-ebook_rssearchengine)
11. [Structural EPUB Validator (`ebook_rs::EpubValidator`)](#11-structural-epub-validator-ebook_rsepubvalidator)
12. [Book Fingerprinting & Deduplication (`ebook_rs::BookFingerprint`)](#12-book-fingerprinting--deduplication-ebook_rsbookfingerprint)
13. [Academic Citation Exporter (`ebook_rs::CitationExporter`)](#13-academic-citation-exporter-ebook_rscitationexporter)
14. [Tree-sitter Concrete Syntax Tree Engine (`ebook_rs::TreeSitterEngine`)](#14-tree-sitter-concrete-syntax-tree-engine-ebook_rstreesitterengine)
15. [Synthetic FXL 2-Page Spreads (`SyntheticSpread`)](#15-synthetic-fxl-2-page-spreads-syntheticspread)
16. [Table of Contents Deep Search & Flattening (`NavPoint::search`, `NavPoint::flatten`)](#16-table-of-contents-deep-search--flattening-navpointsearch-navpointflatten)
17. [Universal EPUB 3 Exporter (`book.export_epub3_bytes()`)](#17-universal-epub-3-exporter-bookexport_epub3_bytes)
18. [Zero-Copy Memory-Mapped I/O (`Book::from_mmap`)](#18-zero-copy-memory-mapped-io-bookfrom_mmap)
19. [Lightweight DOM AST Tree (`EbookDomTree`, `DomNode`)](#19-lightweight-dom-ast-tree-ebookdomtree-domnode)
20. [Fuzzy XML / HTML Recovery Parser (`sanitize_and_repair_xml`)](#20-fuzzy-xml--html-recovery-parser-sanitize_and_repair_xml)
21. [Legacy Non-UTF-8 Charset Decoding (`decode_bytes_with_encoding`)](#21-legacy-non-utf-8-charset-decoding-decode_bytes_with_encoding)
22. [Automatic Language Detection (`book.detect_language()`)](#22-automatic-language-detection-bookdetect_language)
23. [Zstd Compressed State Caching (`export_zstd_cache`, `from_zstd_cache`)](#23-zstd-compressed-state-caching-export_zstd_cache-from_zstd_cache)
24. [Readium Webpub Manifest Export (`ebook_rs::webpub`)](#24-readium-webpub-manifest-export-ebook_rswebpub)
25. [Readium LCP DRM (`ebook_rs::lcp`)](#25-readium-lcp-drm-ebook_rslcp)

---

## 21. Legacy Non-UTF-8 Charset Decoding (`decode_bytes_with_encoding`)

Decode raw byte slices from legacy encodings (`Windows-1252`, `Shift-JIS`, `GBK`, `ISO-8859-1`) into clean Rust UTF-8 strings:

```rust
use ebook_rs::decode_bytes_with_encoding;

let win1252_bytes = b"Hello \x93World\x94";
let decoded = decode_bytes_with_encoding(win1252_bytes, Some("windows-1252"));
println!("Decoded: {}", decoded);
```

---

## 22. Automatic Language Detection (`book.detect_language()`)

Fast statistical language identification on text content when OPF metadata `dc:language` is missing:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;
if let Some(lang) = book.detect_language() {
    println!("Detected Language: {}", lang); // "eng", "fra", "deu", etc.
}
```

---

## 23. Zstd Compressed State Caching (`export_zstd_cache`, `from_zstd_cache`)

Compress and restore parsed book states in sub-milliseconds for high-throughput server caching and WebAssembly state persistence:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;

// Export sub-millisecond compressed state cache
let zstd_cache: Vec<u8> = book.export_zstd_cache()?;

// Instantly restore parsed Book from cache
let restored_book = Book::from_zstd_cache(&zstd_cache)?;
```
