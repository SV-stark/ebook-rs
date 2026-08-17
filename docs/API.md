# 📚 EBook-RS API Reference & Complete Documentation (v0.16.0)

`ebook-rs` (v0.16.0) is a multi-format pure Rust eBook engine supporting **EPUB 2**, **EPUB 3**, **Amazon KFX (Kindle Format 10)**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT (OpenDocument Text)**, **DOCX (Microsoft Word)**, **RTF (Rich Text Format)**, **Plain Text (.txt)**, and **Markdown (.md)** formats with **Mozilla UniFFI Mobile SDK (Swift/Kotlin)**, **Lossless EPUB 3 Optimizer & Minifier**, **CJK Vertical Writing & RTL Reflow Pagination**, **Web Audio API Karaoke Synchronization**, **Lazy Archive Streaming for Giant Files (>500MB)**, **Markdown Frontmatter, Wikilinks & Callouts**, **Model Context Protocol (MCP 2024-11-05) Server**, **Okapi BM25 RAG Relevance Scoring**, **Zero-Allocation Full-Text Search**, **Academic PDF Two-Column Spatial Reflow**, **Multi-Threaded Parallel ZIP Exporting**, **CBZ Comic Pre-fetching & Manga Spread Mode**, **Native PyO3 Python Wheel Bindings**, **100% Clean-Room Amazon KFX Subsystem**, **Native AI/RAG Document Chunking**, **C FFI ABI Bindings**, **WASM Web Component Renderer**, **RTL & Vertical CJK Modes**, **Hardened Sanitizer & Boundary Parsers**, **SpeechSynthesis TTS Word Synchronizer**, **Legacy Non-UTF-8 Charset Decoding**, **Automatic Language Detection**, **Zstd Compressed State Caching**, **Universal EPUB 3 Exporter**, **Zero-Copy mmap**, **Lightweight DOM AST Tree**, **Fuzzy XML Recovery**, **CFI**, and **Readium LCP/Locator** support.

---

## 📋 Table of Contents
1. [Multi-Format Book Core API (`ebook_rs::Book`)](#1-multi-format-book-core-api-ebook_rsbook)
2. [Supported Formats Matrix](#2-supported-formats-matrix)
3. [Format-Specific Parsers (`MobiBook`, `Fb2Book`, `LitBook`, `CbzBook`, `OdtBook`, `DocxBook`, `RtfBook`, `TxtBook`, `PdfBook`)](#3-format-specific-parsers)
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
23: [Zstd Compressed State Caching (`export_zstd_cache`, `from_zstd_cache`)](#23-zstd-compressed-state-caching-export_zstd_cache-from_zstd_cache)
24. [Readium Webpub Manifest Export (`ebook_rs::webpub`)](#24-readium-webpub-manifest-export-ebook_rswebpub)
25. [Readium LCP DRM (`ebook_rs::lcp`)](#25-readium-lcp-drm-ebook_rslcp)
26. [Readium Unified Locator Model (`ReadiumLocator`)](#23-readium-unified-locator-model-readiumlocator)
27. [OPDS Catalog Client & Feed Generator (`ebook_rs::opds`)](#24-opds-catalog-client--feed-generator-ebook_rsopds)
28. [HTTP Reader Server & Web UI (`ebook_rs::server`)](#25-http-reader-server--web-ui-ebook_rsserver)
29. [Performance Accelerators (`compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot`)](#26-performance-accelerators-compact_str-ahash-simdutf8-zlib-rs-memchr-parking_lot)
30. [Manga / Comic Reader Engine (`CbzBook::parse_manga`, `enable_manga_mode`)](#30-manga--comic-reader-engine-cbzbookparse_manga-enable_manga_mode)
31. [Native PyO3 Python Bindings (`pip install ebook-rs`)](#31-native-pyo3-python-bindings-pip-install-ebook-rs)
32. [Academic PDF Two-Column Spatial Reflowing Engine (`reflow_two_column_markdown`)](#32-academic-pdf-two-column-spatial-reflowing-engine-reflow_two_column_markdown)
33. [Mozilla UniFFI Mobile & Native Bindings (`UniBook`, `UniSearchResult`)](#33-mozilla-uniffi-mobile--native-bindings-unibook-unisearchresult)
34. [Lossless EPUB 3 Optimizer & Minifier (`EpubOptimizer`, `EpubOptimizerOptions`)](#34-lossless-epub-3-optimizer--minifier-epuboptimizer-epuboptimizeroptions)
35. [CJK Vertical Writing & RTL Reflow Pagination (`ReflowPaginator`, `WritingMode`)](#35-cjk-vertical-writing--rtl-reflow-pagination-reflowpaginator-writingmode)
36. [Web Audio API & SMIL Karaoke Cue Sheets (`KaraokeCueSheet`, `WebAudioCue`)](#36-web-audio-api--smil-karaoke-cue-sheets-karaokecuesheet-webaudiocue)
37. [Lazy Archive Decompression for Giant Files (>500MB)](#37-lazy-archive-decompression-for-giant-files-500mb)
38. [Markdown Frontmatter, Obsidian Wikilinks & Callouts](#38-markdown-frontmatter-obsidian-wikilinks--callouts)
39. [Microsoft Word (.docx) Parser (`DocxBook`)](#39-microsoft-word-docx-parser-docxbook)
40. [Rich Text Format (.rtf) Parser (`RtfBook`)](#40-rich-text-format-rtf-parser-rtfbook)

---

## 1. Multi-Format Book Core API (`ebook_rs::Book`)

`Book` is the unified entry point struct for opening, inspecting, searching, and rendering eBooks across all formats.

### Opening eBooks (EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ, DOCX, RTF)

```rust
use ebook_rs::Book;

// Auto-detects and loads EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ, DOCX, or RTF from file path
let mut book = Book::from_file("sample.docx")?;

// Load from in-memory byte slice (e.g. HTTP download or WASM Uint8Array)
let bytes: Vec<u8> = std::fs::read("sample.rtf")?;
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
| `.docx` | Microsoft Word (Office Open XML) | `PK\x03\x04` (`word/document.xml`) | Heading, Table, Style & Media Image Ingestion |
| `.rtf` | Rich Text Format | `{\rtf` Header | Control Words, Nested Groups & Embedded Pictures |
| `.txt` | Plain Text | UTF-8 Text | Paragraph HTML Formatting |
| `.md` | Markdown Document | UTF-8 (`# Heading`) | Heading-based Sectioning & TOC |

---

## 3. Format-Specific Parsers

In addition to `Book::from_file()`, format-specific parsers are available:
- `ebook_rs::CbzBook::parse(bytes, title_fallback)` — Parses CBZ comic archives.
- `ebook_rs::MobiBook::parse(bytes)` — Parses MOBI / AZW3 PalmDOC PDB containers.
- `ebook_rs::Fb2Book::parse(bytes)` — Parses FictionBook 2 XML documents.
- `ebook_rs::LitBook::parse(bytes)` — Parses Microsoft Reader LIT files.
- `ebook_rs::OdtBook::parse(bytes, title_fallback)` — Parses OpenDocument Text (.odt) archives.
- `ebook_rs::DocxBook::parse(bytes, title_fallback)` — Parses Microsoft Word (.docx) documents.
- `ebook_rs::RtfBook::parse(bytes, title_fallback)` — Parses Rich Text Format (.rtf) documents.
- `ebook_rs::TxtBook::parse(bytes, title_fallback, is_markdown)` — Parses Plain Text (.txt) or Markdown (.md) documents.
- `ebook_rs::PdfBook::parse(bytes, title_fallback)` — Parses pre-OCR PDF documents (`pdf` feature).

---

## 4. EPUB 3 Accessibility Metadata (`AccessibilityMetadata`)

`ebook-rs` parses W3C EPUB Accessibility 1.1 and Schema.org metadata into `book.metadata().accessibility`:

```rust
let a11y = &book.metadata().accessibility;

if a11y.is_accessible {
    println!("Access modes: {:?}", a11y.access_modes);
    println!("Features: {:?}", a11y.accessibility_features);
    println!("Summary: {:?}", a11y.accessibility_summary);
    println!("Certified by: {:?}", a11y.certified_by);

    assert!(a11y.has_alternative_text());
    assert!(a11y.has_structural_navigation());
    assert!(a11y.is_screen_reader_friendly());
}
```

---

## 5. EPUB 3 Media Overlays (SMIL Audio Sync) (`MediaOverlayPackage`)

`ebook-rs` provides full parsing and query support for EPUB 3 synchronized audio-text read-aloud overlays (`.smil`):

```rust
// Access parsed SMIL packages
for (smil_path, pkg) in &book.media_overlays {
    // Reverse lookup: Find text node active for a given audio timestamp (in seconds)
    if let Some(text_ref) = pkg.find_text_ref_by_timestamp("audio/ch1.mp3", 8.5) {
        println!("Active element ID: {:?}", text_ref.element_id);
    }

    // Forward lookup: Find audio clip start/end times for a clicked paragraph
    if let Some(audio_clip) = pkg.find_audio_clip_by_text_href("chapter1.xhtml#p2") {
        println!("Audio clip: {} -> {}s", audio_clip.clip_begin, audio_clip.clip_end);
    }

    // Export SMIL package as JSON
    let json_str = pkg.to_smil_json()?;
}
```

---

## 12. Readium LCP DRM (`ebook_rs::lcp`)

Parse and validate Readium **Lightweight Content Protection (LCP)** license files (`META-INF/license.lcpl`) and decrypt LCP-protected content:

```rust
use ebook_rs::lcp::{LcpLicense, LcpDecryptor};

// Parse license.lcpl JSON
let lcpl_json = std::fs::read_to_string("META-INF/license.lcpl")?;
let license = LcpLicense::parse(&lcpl_json)?;

println!("Provider: {}", license.provider);
println!("User: {:?}", license.user.as_ref().map(|u| &u.name));

// Check expiry against current ISO date
if license.is_expired("2026-12-01T00:00:00Z") {
    eprintln!("License expired!");
}

// Decrypt LCP AES-256-CBC encrypted content
let encrypted_bytes: Vec<u8> = std::fs::read("content/chapter1.xhtml")?;
let decrypted = LcpDecryptor::decrypt_bytes(&encrypted_bytes, "user_passphrase", &license)?;
```

---

## 13. Readium Unified Locator Model (`ReadiumLocator`)

Generate a W3C/Readium-standard `ReadiumLocator` JSON from any spine index and character offset for cross-platform reading position sync:

```rust
use ebook_rs::ReadiumLocator;

let locator: ReadiumLocator = book.to_readium_locator(3, 250)?;

// Locator contains:
// - href: section file path
// - type: "application/xhtml+xml"
// - locations.cfi: "epubcfi(/6/8!/4/2/1:250)"
// - locations.position: 14
// - locations.progression: 0.42   (section progress)
// - locations.totalProgression: 0.08  (whole-book progress)
// - text.highlight: "...next 100 chars of text at offset..."

let json = serde_json::to_string_pretty(&locator)?;
println!("{}", json);
```

---

## 14. Readium Search Web Service API

Format `SearchResult` from `book.search()` into the Readium standard `application/vnd.readium.search+json` HTTP response schema:

```rust
use ebook_rs::SearchEngine;

let results = book.search("Alice");
let json = SearchEngine::to_readium_search_json(&results, "Alice")?;

// Returns:
// {
//   "@context": "http://readium.org/webpub-manifest/context.jsonld",
//   "metadata": { "numberOfResults": 399, "query": "Alice" },
//   "locators": [
//     { "href": "section_0.html", "type": "application/xhtml+xml",
//       "locations": { "cfi": "epubcfi(/6/2!/4/2/1:42)", "position": 1 },
//       "text": { "snippet": "...Alice was beginning to get very tired..." } },
//     ...
//   ]
// }

println!("{}", json);
```

---

## 10. Full-Text & Regex Search Engine (`ebook_rs::SearchEngine`)

`ebook-rs` provides literal string search and regular expression pattern search with `<mark>` context highlighting:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;

// 1. Literal Search
let literal_results = book.search("quantum");

// 2. Regular Expression Search (case-insensitive regex)
let regex_results = book.search_regex("(?i)quantum|physics|particle")?;

for match_item in regex_results {
    println!("Spine Index: {}", match_item.spine_index);
    println!("CFI: {}", match_item.cfi);
    println!("Context Snippet: {}", match_item.snippet); // "...<mark>quantum</mark> physics..."
}
```

---

## 11. Structural EPUB Validator (`ebook_rs::EpubValidator`)

Validate eBook package structure, OPF metadata, spine items, and navigation hierarchy:

```rust
use ebook_rs::{Book, EpubValidator, ValidationSeverity};

let book = Book::from_file("sample.epub")?;

let report = book.validate(); // or EpubValidator::validate(&book)

if report.is_valid {
    println!("✅ Book passed validation with 0 errors!");
} else {
    println!("❌ Book contains {} validation errors:", report.errors_count);
    for err in report.errors {
        match err.severity {
            ValidationSeverity::Error => eprintln!("[ERROR] {}: {}", err.code, err.message),
            ValidationSeverity::Warning => println!("[WARN] {}: {}", err.code, err.message),
            ValidationSeverity::Info => println!("[INFO] {}: {}", err.code, err.message),
        }
    }
}
```

---

## 12. Book Fingerprinting & Deduplication (`ebook_rs::BookFingerprint`)

Generate metadata-independent SHA-256 content hashes to calculate similarity scores and detect duplicate books across formats:

```rust
use ebook_rs::Book;

let book1 = Book::from_file("book_v1.epub")?;
let book2 = Book::from_file("book_v2.mobi")?;

let fp1 = book1.fingerprint();
let fp2 = book2.fingerprint();

println!("Book 1 Content Hash: {}", fp1.content_hash);
println!("Book 2 Content Hash: {}", fp2.content_hash);

let match_score = fp1.match_score(&fp2); // Returns 0.0 to 1.0
if fp1.is_duplicate_of(&fp2) {
    println!("Duplicate book detected! Match score: {:.2}%", match_score * 100.0);
}
```

---

## 13. Academic Citation Exporter (`ebook_rs::CitationExporter`)

Export academic citations in standard scholarly formats (**BibTeX**, **APA**, **MLA**, **Chicago**):

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;

// 1. BibTeX Format
println!("BibTeX:\n{}", book.to_bibtex());

// 2. APA (7th ed.) Format
println!("APA: {}", book.to_apa());

// 3. MLA (9th ed.) Format
println!("MLA: {}", book.to_mla());

// 4. Chicago (17th ed.) Format
println!("Chicago: {}", book.to_chicago());
```

---

## 14. Tree-sitter Concrete Syntax Tree Engine (`ebook_rs::TreeSitterEngine`)

Tokenize, parse AST syntax nodes (`SyntaxNodeInfo`), and highlight embedded code blocks (`<pre><code>`) across technical eBooks and documentation:

```rust
use ebook_rs::{Book, TreeSitterEngine};

let book = Book::from_file("rust_guide.md")?;

// Extract all code blocks with AST syntax node trees
let blocks = book.extract_code_blocks();
for block in blocks {
    println!("Language: {}", block.language);
    println!("Code: {}", block.code);
    for node in block.ast_nodes {
        println!("AST Node: {} [byte {}-{}]", node.kind, node.start_byte, node.end_byte);
    }
}

// Tokenize standalone code snippet
let ast_nodes = TreeSitterEngine::parse_code("fn main() {}", "rust");
```

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

---

## 17. Universal EPUB 3 Exporter (`book.export_epub3_bytes()`)

Convert ANY parsed eBook format (MOBI, AZW3, FB2, KEPUB, LIT, CBZ, PDF, ODT, DOCX, RTF, TXT, MD) into a clean, compliant EPUB 3 ZIP archive buffer:

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

---

## 26. Performance Accelerators (`compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot`)

`ebook-rs` v0.10.5 integrates 6 SIMD and stack-optimization performance crates:
- **`compact_str`**: Small String Optimization (`CompactString`) storing strings <= 24 bytes directly on the stack to eliminate heap allocations.
- **`ahash`**: `AHashMap` & `AHashSet` for 3x-5x faster hash lookups in OPF manifest, DOM attributes, and annotations.
- **`simdutf8`**: SIMD-accelerated UTF-8 validation (`simdutf8::basic::from_utf8`) for 10x-20x faster HTML/XML byte-stream decoding.
- **`zlib-rs`**: `flate2` with `zlib-rs` SIMD zlib decompression for 3x faster EPUB/CBZ ZIP archive reading.
- **`memchr`**: SIMD substring searching (`memchr::memchr` / `memchr::memmem`) for ultra-fast HTML tag scanning, attribute extraction, and script stripping.
- **`parking_lot`**: Fast 1-byte non-poisoning mutex locks (`parking_lot::Mutex`) for concurrent section caches.

---

## 30. Multi-Threaded Parallel ZIP Exporter (`UniversalEpub3Exporter`)

Parallelizes HTML section document and image asset compression across Rayon worker threads when `parallel` feature is enabled (`entries.par_iter()`), significantly accelerating EPUB 3 export performance for 100MB+ image-heavy books:

```rust
use ebook_rs::{Book, UniversalEpub3Exporter};

let book = Book::from_file("large_comic.cbz")?;
// Exports EPUB 3 zip buffer using multi-threaded compression
let epub_bytes = UniversalEpub3Exporter::export(&book)?;
```

---

## 31. CBZ Comic Pre-fetching & 2-Page Manga Spread Mode (`CbzBook`)

Provides zero-latency image pre-fetching and 2-page Right-to-Left Manga spread view layout:

```rust
use ebook_rs::cbz::CbzBook;
use ebook_rs::Book;

let mut book = CbzBook::parse_manga(&bytes, "My Manga")?;
// Enable RTL Manga Spread Mode
CbzBook::enable_manga_mode(&mut book);

// Pre-fetch adjacent comic page image byte payloads into memory
let prefetched_pages = CbzBook::prefetch_page_images(&book, 0, 3);
```

---

## 32. Native PyO3 Python Bindings (`pip install ebook-rs`)

Native Python extension bindings exposing `Book` and `Section` APIs directly to Python:

```python
import ebook_rs

# Open eBook or comic archive
book = ebook_rs.Book.open("sample.mobi")
print("Title:", book.title)

# Enable Manga Mode & Pre-fetch adjacent pages
book.enable_manga_mode()
pages = book.prefetch_comic_pages(0, 3)

# Export AI RAG Document Chunks as JSON
chunks_json = book.to_rag_chunks_json()
```

---

## 32. Academic PDF Two-Column Spatial Reflowing Engine (`reflow_two_column_markdown`)

Detects multi-column line patterns and spatial column dividers in IEEE, ArXiv, and ACM academic paper PDFs, sorting Left-Column paragraphs top-to-bottom followed by Right-Column paragraphs top-to-bottom into continuous single-column EPUB sections:

```rust
use ebook_rs::pdf::reflow_two_column_markdown;

let raw_pdf_md = "Title\n\nLeft Col 1   |   Right Col 1\nLeft Col 2   |   Right Col 2";
let single_column_md = reflow_two_column_markdown(raw_pdf_md);
```

---

## 33. Mozilla UniFFI Mobile & Native Bindings (`UniBook`, `UniSearchResult`)

`ebook-rs` provides safe, high-level Mozilla UniFFI bindings allowing native iOS (Swift) and Android (Kotlin) reader applications to consume the engine directly:

```rust
use ebook_rs::UniBook;

// Load eBook in Swift / Kotlin native layers
let unibook = UniBook::open("sample.epub".to_string())?;
println!("Title: {}", unibook.get_title());

// Perform Readium CFI searches across all chapters
let results = unibook.search("philosophy".to_string(), false);

// Paginate section with CJK vertical writing mode
let page_map_json = unibook.paginate_section(0, 16, 400, 600, true)?;
```

---

## 34. Lossless EPUB 3 Optimizer & Minifier (`EpubOptimizer`, `EpubOptimizerOptions`)

Automated, non-destructive eBook minification engine that strips redundant HTML whitespace, purges unused CSS selectors across all section documents, and deduplicates identical image and font assets using SHA-1 fingerprints:

```rust
use ebook_rs::Book;
use ebook_rs::optimizer::{EpubOptimizer, EpubOptimizerOptions};

let mut book = Book::from_file("book.epub")?;
let options = EpubOptimizerOptions {
    minify_html: true,
    minify_css: true,
    purge_unused_css: true,
    deduplicate_assets: true,
};

let report = EpubOptimizer::optimize(&mut book, &options);
println!("Deduplicated Assets: {}", report.deduplicated_assets_count);
println!("Purged CSS Rules: {}", report.purged_css_rules_count);

// Export optimized EPUB3 ZIP archive
let optimized_bytes = book.export_optimized_epub3_bytes(&options)?;
```

---

## 35. CJK Vertical Writing & RTL Reflow Pagination (`ReflowPaginator`, `WritingMode`)

Deterministic, DOM-free Reflow Paginator with first-class support for Japanese/Chinese vertical text (`WritingMode::VerticalRl`, `WritingMode::VerticalLr`) and Arabic/Hebrew right-to-left layout (`WritingMode::HorizontalRtl`):

```rust
use ebook_rs::paginator::{ReflowPaginator, WritingMode, is_cjk_char};

let text = "吾輩は猫である。名前はまだ無い。";

// Configure vertical right-to-left paginator
let paginator = ReflowPaginator::new(16, 1.6, 400, 600, 24)
    .with_writing_mode(WritingMode::VerticalRl);

let page_map = paginator.paginate_text(text);
println!("Total Pages: {}", page_map.total_pages);
println!("CSS: {}", paginator.css_properties());
```

---

## 36. Web Audio API & SMIL Karaoke Cue Sheets (`KaraokeCueSheet`, `WebAudioCue`)

SMIL 3.0 Media Overlay engine generating synchronized Web Audio API karaoke cue sheets and HTML annotation tags (`data-audio-src`, `data-clip-begin`, `data-clip-end`, `class="media-overlay-active-target"`):

```rust
use ebook_rs::media_overlay::MediaOverlayPackage;

let package = MediaOverlayPackage::parse_smil(smil_xml, "OEBPS/audio/ch1.smil")?;
let cue_sheet = package.to_karaoke_cue_sheet();

// Annotate section HTML with audio synchronization attributes
let annotated_html = package.annotate_html_with_media_overlays(raw_html);

// Export JSON manifest for Web Audio API players
let manifest_json = package.generate_web_audio_manifest()?;
```

---

## 37. Lazy Archive Decompression for Giant Files (>500MB)

When total uncompressed size across ZIP entries exceeds 500MB, `EpubArchive` seamlessly activates **Lazy Mode**: only structural XML metadata is held in memory, while heavy chapter documents and high-resolution comic images are decompressed on-demand during reading:

```rust
use ebook_rs::archive::EpubArchive;

let archive = EpubArchive::open("huge_manga_omnibus.cbz")?;
if archive.is_lazy() {
    println!("Archive operates in zero-overhead Lazy Streaming Mode");
}

// On-demand entry decompression
let page_bytes = archive.read_bytes("images/page_001.jpg")?;
```

---

## 38. Markdown Frontmatter, Obsidian Wikilinks & Callouts

Markdown parser supporting YAML (`---`) and TOML (`+++`) metadata frontmatter, Obsidian wikilinks (`[[Link]]`, `[[Link|Label]]`), and GFM callout blocks (`> [!NOTE]`, `> [!WARNING]`, `> [!TIP]`):

```rust
use ebook_rs::TxtBook;

let md = r#"---
title: "The Rust Odyssey"
author: "Stark Developer"
---

# Chapter 1

Welcome to [[Chapter 2|Next Chapter]]!

> [!NOTE] Implementation Detail
> Memory mapped I/O ensures instant startup.
"#;

let book = TxtBook::parse(md.as_bytes(), "Default", true)?;
assert_eq!(book.metadata().title, "The Rust Odyssey");
```

---

## 39. Microsoft Word (.docx) Parser (`DocxBook`)

`DocxBook` parses Microsoft Word (.docx) Office Open XML ZIP packages into structured `Book` instances, extracting WordprocessingML headings, paragraphs, formatted text runs (`<strong>`, `<em>`, `<u>`, `<s>`), tables, embedded media images, and Dublin Core metadata:

```rust
use ebook_rs::{Book, DocxBook};

// 1. Direct DOCX parsing
let docx_bytes = std::fs::read("document.docx")?;
let book = DocxBook::parse(&docx_bytes, "Fallback Title")?;

println!("Title: {}", book.metadata().title);
println!("Authors: {:?}", book.metadata().creators);
println!("Sections: {}", book.sections.len());

// 2. Auto-detection via Book::from_bytes
let book = Book::from_bytes(&docx_bytes)?;

// 3. Export to EPUB 3
let epub_bytes = book.export_epub3_bytes()?;
```

---

## 40. Rich Text Format (.rtf) Parser (`RtfBook`)

`RtfBook` parses Microsoft Rich Text Format (.rtf) documents using a streaming control-word tokenizer, managing nested group formatting state stacks, code page charset conversions, Unicode character escapes (`\uN`), tables (`\trowd`/`\cell`/`\row`), and embedded pictures:

```rust
use ebook_rs::{Book, RtfBook};

// 1. Direct RTF parsing
let rtf_bytes = std::fs::read("manuscript.rtf")?;
let book = RtfBook::parse(&rtf_bytes, "Default Title")?;

println!("Title: {}", book.metadata().title);
println!("Sections: {}", book.sections.len());

// 2. Auto-detection via Book::from_bytes
let book = Book::from_bytes(&rtf_bytes)?;

// 3. Perform zero-allocation SIMD search across RTF chapters
let results = book.search("protagonist");
```
