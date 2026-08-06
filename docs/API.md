# 📚 EBook-RS API Reference & Complete Documentation (v0.10.0)

`ebook-rs` (v0.10.0) is a multi-format pure Rust eBook engine supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT (OpenDocument Text)**, **Plain Text (.txt)**, and **Markdown (.md)** formats with **Universal EPUB 3 Exporter**, **Zero-Copy mmap**, **Lightweight DOM AST Tree**, **Fuzzy XML Recovery**, **Synthetic FXL Spreads**, **TOC Deep Search**, **Tree-sitter Code Parser**, **Regex Search**, **Structural EPUB Validator**, **Book Fingerprinting & Deduplication**, **Academic Citation Exporter**, and **Readium LCP/Locator** support.

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
21. [Readium Webpub Manifest Export (`ebook_rs::webpub`)](#21-readium-webpub-manifest-export-ebook_rswebpub)
22. [Readium LCP DRM (`ebook_rs::lcp`)](#22-readium-lcp-drm-ebook_rslcp)
23. [Readium Unified Locator Model (`ReadiumLocator`)](#23-readium-unified-locator-model-readiumlocator)
24. [OPDS Catalog Client & Feed Generator (`ebook_rs::opds`)](#24-opds-catalog-client--feed-generator-ebook_rsopds)
25. [HTTP Reader Server & Web UI (`ebook_rs::server`)](#25-http-reader-server--web-ui-ebook_rsserver)

---

## 17. Universal EPUB 3 Exporter (`book.export_epub3_bytes()`)

Convert ANY parsed eBook format (MOBI, AZW3, FB2, KEPUB, LIT, CBZ, PDF, ODT, TXT, MD) into a clean, compliant EPUB 3 ZIP archive buffer:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.mobi")?;

// Convert MOBI to EPUB 3 bytes
let epub_bytes: Vec<u8> = book.export_epub3_bytes()?;
std::fs::write("converted_sample.epub", epub_bytes)?;
```

---

## 18. Zero-Copy Memory-Mapped I/O (`Book::from_mmap`)

Open 500MB+ omnibus books or comic archives (`.cbz`) using zero-copy OS memory-mapped I/O (`memmap2` feature):

```rust
use ebook_rs::Book;

// Zero-copy mmap loading
let book = Book::from_mmap("huge_comic_omnibus.cbz")?;
println!("Loaded {} sections with mmap", book.sections.len());
```

---

## 19. Lightweight DOM AST Tree (`EbookDomTree`, `DomNode`)

Zero-allocation HTML/XML DOM AST tree parser supporting fast node querying and element stripping:

```rust
use ebook_rs::EbookDomTree;

let html = "<div><h1>Title</h1><script>alert(1)</script><p>Text</p></div>";
let mut tree = EbookDomTree::parse(html);

// Find elements by tag
let h1_nodes = tree.find_elements_by_tag("h1");

// Strip forbidden script/style tags
tree.strip_elements(&["script", "style"]);
let clean_html = tree.to_html();
```

---

## 20. Fuzzy XML / HTML Recovery Parser (`sanitize_and_repair_xml`)

Lenient recovery sanitizer repairing unescaped ampersands (`&`), unclosed tags, and malformed entities for 100% parse success rates:

```rust
use ebook_rs::sanitize_and_repair_xml;

let broken_xml = "<package><title>AT&T & R&D Guide</title></package>";
let repaired = sanitize_and_repair_xml(broken_xml);
println!("Repaired XML: {}", repaired); // <package><title>AT&amp;T &amp; R&amp;D Guide</title></package>
```
