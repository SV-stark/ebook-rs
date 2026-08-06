# 📚 EBook-RS API Reference & Complete Documentation (v0.9.0)

`ebook-rs` (v0.9.0) is a multi-format pure Rust eBook engine supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT (OpenDocument Text)**, **Plain Text (.txt)**, and **Markdown (.md)** formats with **Synthetic FXL Spreads**, **TOC Deep Search**, **Tree-sitter Code Parser**, **Regex Search**, **Structural EPUB Validator**, **Book Fingerprinting & Deduplication**, **Academic Citation Exporter**, and **Readium LCP/Locator** support.

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
17. [Readium Webpub Manifest Export (`ebook_rs::webpub`)](#17-readium-webpub-manifest-export-ebook_rswebpub)
18. [Readium LCP DRM (`ebook_rs::lcp`)](#18-readium-lcp-drm-ebook_rslcp)
19. [Readium Unified Locator Model (`ReadiumLocator`)](#19-readium-unified-locator-model-readiumlocator)
20. [OPDS Catalog Client & Feed Generator (`ebook_rs::opds`)](#20-opds-catalog-client--feed-generator-ebook_rsopds)
21. [HTTP Reader Server & Web UI (`ebook_rs::server`)](#21-http-reader-server--web-ui-ebook_rsserver)

---

## 1. Multi-Format Book Core API (`ebook_rs::Book`)

`Book` is the unified entry point struct for opening, inspecting, searching, and rendering eBooks across all formats.

### Opening eBooks (EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ, PDF, ODT, TXT, MD)

```rust
use ebook_rs::Book;

// Auto-detects and loads EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ, PDF, ODT, TXT, or MD from file path
let mut book = Book::from_file("sample.epub")?;

// Load from in-memory byte slice (e.g. HTTP download or WASM Uint8Array)
let bytes: Vec<u8> = std::fs::read("sample.epub")?;
let mut book = Book::from_bytes(&bytes)?;
```

---

## 2. Supported Formats Matrix

| Extension | Format Name | Auto-Detected Header Magic | Inlining & Media |
|---|---|---|---|
| `.epub` | EPUB 2 & EPUB 3 | `PK\x03\x04` | Full Base64 Data URIs / Resource Stream |
| `.kepub` | Kobo EPUB | `PK\x03\x04` | KoboSpan DOM Aware |
| `.mobi` | Mobipocket / PalmDOC | `BOOKMOBI` / `TEXtREDR` | PalmDOC LZ77 Decompressor |
| `.azw3` | Kindle Format 8 (KF8) | `BOOKMOBI` PDB Header | HTML5 / CSS3 Extracted |
| `.fb2` | FictionBook 2 XML | `<FictionBook>` XML | Embedded Base64 Images |
| `.lit` | Microsoft Reader | `ITOLITLS` Header | Multi-section HTML Conversion |
| `.cbz` | Comic Book ZIP | `PK\x03\x04` Archive | Sequential Image Page HTML Reader |
| `.pdf` | PDF Document | `%PDF-` Signature | Plain Text / Markdown Page Extraction |
| `.odt` | OpenDocument Text | `PK\x03\x04` (`content.xml`) | Heading & Paragraph Section Extraction |
| `.txt` | Plain Text | UTF-8 Text | Paragraph HTML Formatting |
| `.md` | Markdown Document | UTF-8 (`# Heading`) | Heading-based Sectioning & TOC |

---

## 15. Synthetic FXL 2-Page Spreads (`SyntheticSpread`)

Auto-synthesize responsive side-by-side two-page spread containers for EPUB 3 Fixed-Layout and comic books:

```rust
use ebook_rs::Book;

let book = Book::from_file("comic.cbz")?;

// Generate side-by-side synthetic spread for page 0 and page 1
let spread = book.get_synthetic_spread(0, Some(1))?;

println!("Left: {}, Right: {:?}", spread.left_index, spread.right_index);
println!("Page dimensions: {:.0}x{:.0}", spread.width, spread.height);
println!("Combined Spread HTML:\n{}", spread.combined_html);
```

---

## 16. Table of Contents Deep Search & Flattening (`NavPoint::search`, `NavPoint::flatten`)

Search TOC nodes down to any depth level with parent breadcrumbs and flatten TOC trees into linear depth lists:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;

// 1. Deep TOC Search
let matches = book.search_toc("quantum");
for m in matches {
    println!("Found: {} (Breadcrumb: '{}', Depth: {})", m.label, m.breadcrumb, m.depth);
}

// 2. Flatten TOC Tree
let flat_toc = book.flatten_toc();
for node in flat_toc {
    println!("[Depth {}] {} -> {}", node.depth, node.breadcrumb, node.href);
}
```
