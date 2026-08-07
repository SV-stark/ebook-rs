# 📚 EBook-RS API Reference & Complete Documentation (v0.11.1)

`ebook-rs` (v0.11.1) is a multi-format pure Rust eBook engine supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT (OpenDocument Text)**, **Plain Text (.txt)**, and **Markdown (.md)** formats with **SpeechSynthesis TTS Word Synchronizer**, **Legacy Non-UTF-8 Charset Decoding**, **Automatic Language Detection**, **Zstd Compressed State Caching**, **Universal EPUB 3 Exporter**, **Zero-Copy mmap**, **Lightweight DOM AST Tree**, **Fuzzy XML Recovery**, **CFI**, and **Readium LCP/Locator** support.

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
17. [SpeechSynthesis TTS Word Synchronizer (`TtsWordToken`, `get_tts_tokens`)](#17-speechsynthesis-tts-word-synchronizer-ttswordtoken-get_tts_tokens)
18. [Universal EPUB 3 Exporter (`book.export_epub3_bytes()`)](#18-universal-epub-3-exporter-bookexport_epub3_bytes)
19. [Zero-Copy Memory-Mapped I/O (`Book::from_mmap`)](#19-zero-copy-memory-mapped-io-bookfrom_mmap)
20. [Lightweight DOM AST Tree (`EbookDomTree`, `DomNode`)](#20-lightweight-dom-ast-tree-ebookdomtree-domnode)
21. [Legacy Non-UTF-8 Charset Decoding (`decode_bytes_with_encoding`)](#21-legacy-non-utf-8-charset-decoding-decode_bytes_with_encoding)
22. [Automatic Language Detection (`book.detect_language()`)](#22-automatic-language-detection-bookdetect_language)
23. [Zstd Compressed State Caching (`export_zstd_cache`, `from_zstd_cache`)](#23-zstd-compressed-state-caching-export_zstd_cache-from_zstd_cache)

---

## 17. SpeechSynthesis TTS Word Synchronizer (`TtsWordToken`, `get_tts_tokens`)

Tokenize text into word tokens with character offsets and generate HTML annotated with `<span id="tts-w-{index}">` for live Web Speech API SpeechSynthesis word-by-word visual highlighting:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;

// 1. Get tokenized word tokens with exact character range offsets
let tokens = book.get_tts_tokens(0)?;
for t in tokens {
    println!("Word #{}: '{}' (range {}-{})", t.index, t.word, t.char_start, t.char_end);
}

// 2. Get section HTML with <span id="tts-w-{index}"> annotations
let tts_html = book.get_tts_section_html(0)?;
```
