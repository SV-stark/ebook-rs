# 📚 EBook-RS API Reference & Complete Documentation (v0.19.4)
 
`ebook-rs` (v0.19.4) is a multi-format pure Rust eBook engine supporting **30 input formats and 28+ output formats** (**EPUB 2/3**, **Kobo KEPUB**, **Amazon KFX**, **MOBI**, **AZW3**, **PDB**, **FB2**, **LIT**, **CBZ**, **PDF**, **DOCX**, **ODT**, **RTF**, **PPTX**, **HTML**, **HTMLZ**, **TXT**, **TXTZ**, **Markdown**, **OEB**, **ZIP**, **TCR**, **PMLZ**, **RB**, **Typst**, **AsciiDoc**, **Org-mode**, **reStructuredText**, **LaTeX**) with **Mozilla UniFFI Mobile SDK (Swift/Kotlin)**, **Lossless EPUB 3 Optimizer & Minifier**, **CJK Vertical Writing & RTL Reflow Pagination**, **Web Audio API Karaoke Synchronization**, **Lazy Archive Streaming for Giant Files (>500MB)**, **Markdown Frontmatter, Wikilinks & Callouts**, **Model Context Protocol (MCP 2024-11-05) Server**, **Okapi BM25 RAG Relevance Scoring**, **Zero-Allocation Full-Text Search**, **PDF Two-Column Reading-Order Reflow**, **Multi-Threaded Parallel ZIP Exporting**, **CBZ Comic Pre-fetching & Manga Spread Mode**, **Native PyO3 Python Wheel Bindings**, **100% Clean-Room Amazon KFX Subsystem**, **Native AI/RAG Document Chunking**, **C FFI ABI Bindings**, **WASM Web Component Renderer**, **Embedded HTTP Reader Server & CLI**, **OPDS & Readium WebPub**, **Annotations & Reading Analytics**, **Font Deobfuscation**, **SpeechSynthesis TTS Word Synchronizer**, **Legacy Non-UTF-8 Charset Decoding**, **Automatic Language Detection**, **Pure-Rust Compressed State Caching (`flate2`/`zlib-rs`)**, **Universal 28-Format Exporter**, **Zero-Copy mmap**, **Lightweight DOM AST Tree**, **Fuzzy XML Recovery**, **CFI**, and **Readium LCP/Locator** support.

---

## 📋 Table of Contents
1. [Multi-Format Book Core API (`ebook_rs::Book`)](#1-multi-format-book-core-api-ebook_rsbook)
   - 1b. [Locations & Reading Progress (`locations_from_sections`)](#1b-locations--reading-progress-locations_from_sections)
   - 1c. [Universal Exporters (`export_epub3_bytes`, `export_kfx_bytes`)](#1c-universal-exporters-export_epub3_bytes-export_kfx_bytes)
2. [Supported Formats Matrix](#2-supported-formats-matrix)
3. [Format-Specific Parsers (`MobiBook`, `KfxBook`, `Fb2Book`, `LitBook`, `CbzBook`, `OdtBook`, `DocxBook`, `RtfBook`, `TxtBook`, `PdfBook`)](#3-format-specific-parsers)
4. [EPUB 3 Accessibility Metadata (`AccessibilityMetadata`)](#4-epub-3-accessibility-metadata-accessibilitymetadata)
5. [EPUB 3 Media Overlays (SMIL Audio Sync) (`MediaOverlayPackage`)](#5-epub-3-media-overlays-smil-audio-sync-mediaoverlaypackage)
6. [Readium LCP DRM (`ebook_rs::lcp`)](#6-readium-lcp-drm-ebook_rslcp)
7. [Readium Unified Locator Model (`ReadiumLocator`)](#7-readium-unified-locator-model-readiumlocator)
8. [Readium Search Web Service API](#8-readium-search-web-service-api)
9. [Full-Text & Regex Search Engine (`ebook_rs::SearchEngine`)](#9-full-text--regex-search-engine-ebook_rssearchengine)
10. [Structural EPUB Validator (`ebook_rs::EpubValidator`)](#10-structural-epub-validator-ebook_rsepubvalidator)
11. [Book Fingerprinting & Deduplication (`ebook_rs::BookFingerprint`)](#11-book-fingerprinting--deduplication-ebook_rsbookfingerprint)
12. [Academic Citation Exporter (`ebook_rs::CitationExporter`)](#12-academic-citation-exporter-ebook_rscitationexporter)
13. [Code Block Syntax Highlighter & Extractor (`ebook_rs::SyntaxHighlightEngine`)](#13-code-block-syntax-highlighter--extractor-ebook_rssyntaxhighlightengine)
14. [Synthetic FXL 2-Page Spreads (`SyntheticSpread`)](#14-synthetic-fxl-2-page-spreads-syntheticspread)
15. [Table of Contents Deep Search & Flattening (`NavPoint::search`, `NavPoint::flatten`)](#15-table-of-contents-deep-search--flattening-navpointsearch-navpointflatten)
16. [SpeechSynthesis TTS Word Synchronizer (`TtsWordToken`, `get_tts_tokens`)](#16-speechsynthesis-tts-word-synchronizer-ttswordtoken-get_tts_tokens)
17. [Universal EPUB 3 Exporter (`book.export_epub3_bytes()`)](#17-universal-epub-3-exporter-bookexport_epub3_bytes)
18. [Zero-Copy Memory-Mapped I/O (`Book::from_mmap`)](#18-zero-copy-memory-mapped-io-bookfrom_mmap)
19. [Lightweight DOM AST Tree (`EbookDomTree`, `DomNode`)](#19-lightweight-dom-ast-tree-ebookdomtree-domnode)
20. [Fuzzy XML / HTML Recovery Parser (`sanitize_and_repair_xml`)](#20-fuzzy-xml--html-recovery-parser-sanitize_and_repair_xml)
20b. [HTML Character Reference Decoding (`decode_html_entities`)](#20b-html-character-reference-decoding-decode_html_entities)
21. [Legacy Non-UTF-8 Charset Decoding (`decode_bytes_with_encoding`)](#21-legacy-non-utf-8-charset-decoding-decode_bytes_with_encoding)
22. [Automatic Language Detection (`book.detect_language()`)](#22-automatic-language-detection-bookdetect_language)
23. [Compressed State Caching (`export_cache`, `from_cache`)](#23-compressed-state-caching-export_cache-from_cache)
24. [Performance Accelerators (`compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot`)](#24-performance-accelerators-compact_str-ahash-simdutf8-zlib-rs-memchr-parking_lot)
25. [Multi-Threaded Parallel ZIP Exporter (`UniversalEpub3Exporter`)](#25-multi-threaded-parallel-zip-exporter-universalepub3exporter)
26. [CBZ Comic Pre-fetching & 2-Page Manga Spread Mode (`CbzBook`)](#26-cbz-comic-pre-fetching--2-page-manga-spread-mode-cbzbook)
27. [Native PyO3 Python Bindings (`pip install ebook-rs`)](#27-native-pyo3-python-bindings-pip-install-ebook-rs)
28. [PDF Two-Column Reading-Order Reflow (`reflow_two_column_markdown`)](#28-pdf-two-column-reading-order-reflow-reflow_two_column_markdown)
29. [Mozilla UniFFI Mobile & Native Bindings (`UniBook`, `UniSearchResult`)](#29-mozilla-uniffi-mobile--native-bindings-unibook-unisearchresult)
30. [Lossless EPUB 3 Optimizer & Minifier (`EpubOptimizer`, `EpubOptimizerOptions`)](#30-lossless-epub-3-optimizer--minifier-epuboptimizer-epuboptimizeroptions)
31. [CJK Vertical Writing & RTL Reflow Pagination (`ReflowPaginator`, `WritingMode`)](#31-cjk-vertical-writing--rtl-reflow-pagination-reflowpaginator-writingmode)
32. [Web Audio API & SMIL Karaoke Cue Sheets (`KaraokeCueSheet`, `WebAudioCue`)](#32-web-audio-api--smil-karaoke-cue-sheets-karaokecuesheet-webaudiocue)
33. [Lazy Archive Decompression for Giant Files (>500MB)](#33-lazy-archive-decompression-for-giant-files-500mb)
34. [Markdown Frontmatter, Obsidian Wikilinks & Callouts](#34-markdown-frontmatter-obsidian-wikilinks--callouts)
35. [Microsoft Word (.docx) Parser (`DocxBook`)](#35-microsoft-word-docx-parser-docxbook)
36. [Rich Text Format (.rtf) Parser (`RtfBook`)](#36-rich-text-format-rtf-parser-rtfbook)
37. [Universal 28-Format Pure-Rust Exporter (`Book::export_format_bytes`, `ebook_rs::formats`)](#37-universal-28-format-pure-rust-exporter-bookexport_format_bytes-ebook_rsformats)
38. [MCP Server (`convert_ebook`, `export_ebook_text`, & Tools/Prompts)](#38-mcp-server-convert_ebook-export_ebook_text--toolsprompts)
39. [AI / RAG Document Chunking & Okapi BM25 Ranking (`RagChunker`, `RagChunkConfig`)](#39-ai--rag-document-chunking--okapi-bm25-ranking-ragchunker-ragchunkconfig)
40. [Clean-Room Amazon KFX Codec (`KfxBook`, `UniversalKfxExporter`)](#40-clean-room-amazon-kfx-codec-kfxbook-universalkfxexporter)
41. [Annotations & Highlights (`AnnotationManager`, `Annotation`)](#41-annotations--highlights-annotationmanager-annotation)
42. [Reading Analytics & Footnotes (`ReadingAnalytics`, `Footnote`)](#42-reading-analytics--footnotes-readinganalytics-footnote)
43. [Readium WebPub & OPDS Feeds (`WebpubManifest`, `OpdsFeed`)](#43-readium-webpub--opds-feeds-webpubmanifest-opdsfeed)
44. [Font Deobfuscation & HTTP Range Streaming (`FontDeobfuscator`, `ZipHeaderReader`)](#44-font-deobfuscation--http-range-streaming-fontdeobfuscator-zipheaderreader)
45. [Rendition Layout & Before-Display Hooks (`RenditionLayout`, `register_before_display_hook`)](#45-rendition-layout--before-display-hooks-renditionlayout-register_before_display_hook)
46. [Embedded HTTP Reader Server & CLI (`ReaderServer`, `ebook-rs`)](#46-embedded-http-reader-server--cli-readerserver-ebook-rs)
47. [WebAssembly & C FFI ABI Bindings (`WasmBook`, `ebook_rs::ffi`)](#47-webassembly--c-ffi-abi-bindings-wasmbook-ebook_rsffi)
48. [Sample EPUB Generator (`sample_builder::generate_sample_epub`)](#48-sample-epub-generator-sample_buildergenerate_sample_epub)

---

## 1. Multi-Format Book Core API (`ebook_rs::Book`)

`Book` is the unified entry point struct for opening, inspecting, searching, and rendering eBooks across all formats.

### Opening eBooks (All 30 Supported Input Formats)

```rust
use ebook_rs::Book;

// Auto-detects and loads any supported format from a file path (or OEB directory / .opf)
let book = Book::from_file("sample.docx")?;

// Load from in-memory byte slice (e.g. HTTP download or WASM Uint8Array)
let bytes: Vec<u8> = std::fs::read("sample.rtf")?;
let book = Book::from_bytes(&bytes)?;
let book = Book::from_bytes_with_title(&bytes, "Custom Title")?;

// Load from any Read + Seek stream (up to 2 GiB safety limit)
let file = std::fs::File::open("sample.epub")?;
let book = Book::from_reader(file)?;
```

### Render Cache (`book.render_cache`, `RenderCacheKey`)

`get_section` memoizes its rendered output in `book.render_cache`. Entries are keyed by
`RenderCacheKey`, which packs the section index together with every input that affects the output:
`layout.allow_scripted_content` (the script-stripping gate), the number of registered
`before_display_hooks`, and the source HTML length. Keying on the index alone returned a stale
rendering whenever any of those inputs changed — flipping the public `layout` field back to
`allow_scripted_content = false` still served HTML containing `<script>`. The cache is bounded to
64 entries, evicting the oldest.

```rust
use ebook_rs::{Book, PageProgressionDirection, RenderCacheKey};

let book = Book::from_file("sample.epub")?;

// Keying the cache on all render inputs makes the entry order-insensitive:
let key = RenderCacheKey {
    index: 0,
    allow_scripted_content: book.layout().allow_scripted_content,
    asset_delivery: book.layout().asset_delivery,
    is_rtl: book.opf().metadata.direction == PageProgressionDirection::Rtl,
    hook_count: book.before_display_hooks().len(),
    source_len: book.sections()[0].raw_html.len(),
};
let _section = book.get_section(key.index)?;
```

### CFI Position Resolution & DOM Anchoring

`get_section_by_cfi` historically discarded the in-section offset. The new helpers preserve it for
exact-position restore, scroll, and highlight anchoring:

```rust
use ebook_rs::{Book, CfiDomTarget};

let book = Book::from_file("sample.epub")?;

// Section + in-section character offset (measured against section.plain_text)
let (section, offset) = book.get_section_and_offset_by_cfi("epubcfi(/6/4!/4/2/1:12)")?;

// Full resolution: section, offset, and a CSS selector for the DOM node
let (section, offset, dom) = book.resolve_cfi("epubcfi(/6/4!/4/2/1:12)", &section.processed_html)?;
println!("selector: {}", dom.css_selector);
println!("char_offset: {}", dom.char_offset);
```

### Sections, Resources, Cover & Page List

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;

// Retrieve a spine section by relative href (strips optional `#fragment`)
let section = book.get_section_by_href("chapter1.xhtml#intro")?;

// Retrieve raw resource bytes and MIME type (strips optional `"resource/"` prefix)
let (img_bytes, mime) = book.get_resource_bytes("images/cover.jpg")?;

// Retrieve cover image bytes and MIME type (falls back to manifest `cover_id` / XHTML cover)
if let Some((cover_bytes, cover_mime)) = book.cover_image() {
    println!("Cover: {} ({} bytes)", cover_mime, cover_bytes.len());
}

// Access EPUB 3 Page List and Landmarks navigation items
for page in book.page_list() {
    println!("Page {} -> {}", page.label, page.href);
}
for landmark in book.landmarks() {
    println!("Landmark {:?} -> {}", landmark.landmark_type, landmark.href);
}
```

### Lazy Hydration, Raw Access, Cache Control & Mutable Accessors

```rust
// Raw section (without rendering/resource inlining) or on-demand lazy load
let raw = book.get_section_raw(0);
let lazy = book.load_section_lazy(0)?;

// Hydrate lazy sections in-place or obtain hydrated clones without mutating `book`
book.hydrate_all_sections();
let hydrated_sections = book.get_all_sections_hydrated();

// Manually clear render_cache and inspect the font deobfuscator
book.invalidate_render_cache();
let _deobf = book.font_deobfuscator();

// Mutable accessors for in-place editing:
// `archive_mut()`, `opf_mut()`, `sections_mut()`, `sections_vec_mut()`,
// `locations_mut()`, `layout_mut()`, `toc_mut()`
```

### Async API (`ebook_rs::book::async_api`)

```rust
use std::sync::Arc;
use ebook_rs::book::async_api::{from_file_async, from_bytes_async, load_section_lazy_async};

let book = from_file_async("sample.epub").await?;
let book_from_mem = from_bytes_async(bytes).await?;
let section = load_section_lazy_async(Arc::new(book), 0).await?;
```

### Section HTML & Resource Helpers (`ebook_rs::section`)

- `extract_plain_text(html)` — strip HTML tags and extract plain text.
- `sanitize_html_scripts(html)` / `section.strip_script_content()` — strip `<script>`, `<iframe>`, `<object>`, `<embed>`, inline `on*=` handlers, and `javascript:` URIs.
- `parse_viewport_meta(html)` — extract `(Option<f64>, Option<f64>)` from `<meta name="viewport">`.
- `process_section_resources(html, section_path, archive)` — inline/rewrite referenced CSS and media resources.

### Merged Book Stylesheet & CSS Inlining

Renderers without data-URI / `<link>` support can fetch the book's CSS directly, or inline `<style>` rules onto HTML elements:

```rust
use ebook_rs::inline_stylesheet_rules;

let css: String = book.get_merged_stylesheet(); // all <link rel="stylesheet"> CSS, url() inlined

// Inline <style>...</style> rules onto matching elements (tag, .class, tag.class, #id),
// preserving existing inline `style="..."` precedence
let inlined_html = inline_stylesheet_rules(&section.raw_html);
```

### LCP Passphrase-Aware Loading

```rust
// Supply a passphrase at load time for LCP-protected books
let book = Book::from_bytes_with_passphrase(&bytes, "title", "user_passphrase")?;
// or with optional passphrase:
let book = Book::from_bytes_with_title_and_passphrase(&bytes, "title", Some("user_passphrase"))?;
// or, for an already-opened archive (eager or lazy):
let book = Book::from_archive_with_passphrase(archive, Some("user_passphrase"))?;
let book = Book::from_archive_lazy_with_passphrase(archive, "title", Some("user_passphrase"))?;
// passphrase and parsed license are retained via book.lcp_passphrase() / book.lcp_license()
```

---

## 1b. Locations & Reading Progress (`locations_from_sections`)

Build a Readium-style location map from the spine sections **without mutating the book**. Lazy
sections are hydrated on a clone, so the map reflects fully-rendered plain text while `self`
(including its `locations` field and render cache) is left untouched.

```rust
use ebook_rs::{Book, Locations};

fn build(book: &Book) -> Locations {
    // 1000 characters per location (the same default as `Locations::default()`).
    let locations = book.locations_from_sections(1000);
    println!("{} locations over {} characters",
        locations.total_locations, locations.total_characters);
    locations
}
```

To regenerate and *store* the map on the book instead, use the mutating
`Book::generate_locations(chunk_size)`.

**Related:** `book.locations()` holds the map computed at load time; `Locations::new`,
`add_spine_section`, and `finalize` are public for custom chunking strategies.

---

## 1c. Universal Exporters (`export_epub3_bytes`, `export_kfx_bytes`)

Any loaded format can be re-exported as a standards-clean EPUB 3 archive or an Amazon KFX (KF10)
container. Both are symmetric methods on `Book`:

```rust
use ebook_rs::Book;

fn export(book: &Book) -> Result<(), Box<dyn std::error::Error>> {
    let epub = book.export_epub3_bytes()?;   // EPUB 2/3, MOBI, AZW3, FB2, LIT, CBZ,
    let kfx  = book.export_kfx_bytes()?;    // PDF, ODT, DOCX, RTF, TXT, MD -> EPUB 3 / KF10

    std::fs::write("output.epub", epub)?;
    std::fs::write("output.kfx", kfx)?;
    Ok(())
}
```

`export_kfx_bytes` produces a clean-room `CONT`-magic container; it is not a byte-for-byte
reproduction of Amazon's KF10 obfuscation, which is undocumented. For a minified,
asset-deduplicated, CSS-purged EPUB use `export_optimized_epub3_bytes`. `export_cache` /
`from_cache` round-trip the parsed state as a pure-Rust gzip (`flate2`/`zlib-rs`) buffer.

---

## 2. Supported Formats Matrix

| Extension | Format Name | Auto-Detected Header Magic / Probing | Inlining & Media |
|---|---|---|---|
| `.epub` | EPUB 2 & EPUB 3 | `PK\x03\x04` (`META-INF/container.xml`) | Full Base64 Data URIs / Resource Stream |
| `.kepub` | Kobo EPUB | `PK\x03\x04` (`META-INF/container.xml`) | Parsed as EPUB (see note) |
| `.kfx` | Amazon KFX (KF10) | `CONT` Header | Clean-Room Ion v2 Codec, Fonts, Bidi/RTL, Cover |
| `.mobi` | Mobipocket / PalmDOC | `BOOKMOBI` / `TEXtRECD` at PDB offset 60 | PalmDOC LZ77 Decompressor |
| `.azw3` | Kindle Format 8 (KF8) | `BOOKMOBI` PDB Header | HTML5 / CSS3 Extracted |
| `.pdb` | PalmDOC | `TEXtRECD` / PDB Header | PalmDOC LZ77 Text Extraction |
| `.fb2` | FictionBook 2 XML | `<FictionBook>` XML | Embedded Base64 Images |
| `.lit` | Microsoft Reader | `ITOLITLS` Header | Multi-section HTML Conversion |
| `.cbz` | Comic Book ZIP | `PK\x03\x04` Archive (Fallback) | Sequential Image Page HTML Reader |
| `.pdf` | PDF Document | `%PDF-` Signature | Plain Text / Markdown Page Extraction (`pdf` feature) |
| `.odt` | OpenDocument Text | `PK\x03\x04` (`content.xml` / `meta.xml`) | Heading & Paragraph Section Extraction |
| `.docx` | Microsoft Word (Office Open XML) | `PK\x03\x04` (`word/document.xml`) | Heading, Table, Style & Media Image Ingestion |
| `.pptx` | PowerPoint (Office Open XML) | `PK\x03\x04` (`ppt/presentation.xml`) | Slide & Text Extraction |
| `.rtf` | Rich Text Format | `{\rtf` Header | Control Words, Nested Groups & Embedded Pictures |
| `.html` / `.htm` | Standalone HTML | HTML Tag Probing | Sectioned HTML Ingestion |
| `.htmlz` | Calibre HTMLZ | `PK\x03\x04` ZIP (`index.html` / OPF) | HTML + CSS + Images |
| `.zip` | Calibre HTML/Images ZIP | `PK\x03\x04` ZIP | HTML + CSS + Images |
| `.oeb` | Open eBook Directory | Directory or `.opf` Path | `content.opf` + Referenced Assets |
| `.tcr` | Psion TCR | `!!8-Bit!!` Header | 256-Entry Dictionary Decompression |
| `.pmlz` | Palm Markup Language ZIP | `PK\x03\x04` ZIP (`.pml`) | PML Markup + Images |
| `.rb` | Rocket eBook | `NUVO` Header | Chunked Zlib HTML |
| `.txt` | Plain Text | UTF-8 Text | Paragraph HTML Formatting |
| `.txtz` | Calibre TXTZ | `PK\x03\x04` ZIP | Compressed Text + Metadata |
| `.md` | Markdown Document | UTF-8 (`# Heading`, `---`, `+++`) | Frontmatter, Wikilinks, Callouts & Heading TOC |
| `.typ` / `.typst` | Typst | `.typ`/`.typst` or Typst Syntax | Markup Sectioning |
| `.adoc` | AsciiDoc | `.adoc`/`.asciidoc` or AsciiDoc Syntax | Markup Sectioning |
| `.org` | Emacs Org-mode | `.org` or Org Syntax | Markup Sectioning |
| `.rst` | reStructuredText | `.rst` or RST Syntax | Markup Sectioning |
| `.tex` | LaTeX | `.tex`/`.latex` or LaTeX Syntax | Markup Sectioning |

Detection is by **magic bytes / container probing**, not by file extension — the extension column
is informational only. ZIP-based formats are disambiguated by probing for `META-INF/container.xml`
(EPUB), `word/document.xml` (DOCX), then `content.xml` / `meta.xml` (ODT), falling back to CBZ.

**KEPUB note:** `.kepub` files are ordinary ZIP containers and are handled by the EPUB path above.
There is **no KoboSpan-specific DOM handling** — the `kobospan:` custom elements are not
transformed. If your KEPUB depends on KoboSpan markup for layout, convert it to EPUB first.

---

## 3. Format-Specific Parsers

In addition to `Book::from_file()`, format-specific parsers are available:
- `ebook_rs::CbzBook::parse(bytes, title_fallback)` — Parses CBZ comic archives.
- `ebook_rs::MobiBook::parse(bytes)` — Parses MOBI / AZW3 PalmDOC PDB containers.
- `ebook_rs::KfxBook::parse(bytes)` — Parses Amazon KFX (`CONT`) containers into a `Book`.
- `ebook_rs::Fb2Book::parse(bytes)` — Parses FictionBook 2 XML documents.
- `ebook_rs::LitBook::parse(bytes)` — Parses Microsoft Reader LIT files.
- `ebook_rs::OdtBook::parse(bytes, title_fallback)` — Parses OpenDocument Text (.odt) archives.
- `ebook_rs::DocxBook::parse(bytes, title_fallback)` — Parses Microsoft Word (.docx) documents.
- `ebook_rs::RtfBook::parse(bytes, title_fallback)` — Parses Rich Text Format (.rtf) documents.
- `ebook_rs::TxtBook::parse(bytes, title_fallback, is_markdown)` — Parses Plain Text (.txt) or Markdown (.md) documents.
- `ebook_rs::PdfBook::parse(bytes, title_fallback)` — Parses pre-OCR PDF documents (`pdf` feature).
- `ebook_rs::formats::*` — Direct parsers for `parse_pdb`, `parse_pptx`, `parse_html`, `parse_oeb_path`, `parse_tcr`, `parse_zip_ebook`, `parse_pmlz_archive`, `parse_rb`, `parse_typst`, `parse_adoc`, `parse_org`, `parse_rst`, and `parse_tex`.

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
for (smil_path, pkg) in book.media_overlays() {
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

## 6. Readium LCP DRM (`ebook_rs::lcp`)

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

Passphrase-aware loading is also available on `Book` (see §1): `Book::from_bytes_with_passphrase`,
`Book::from_archive_with_passphrase`, `Book::from_archive_lazy_with_passphrase`, and
`Book::from_bytes_with_title_and_passphrase`. The passphrase and parsed license are retained via
`book.lcp_passphrase()` / `book.lcp_license()` for per-resource decryption. The
`META-INF/license.lcpl` path is resolved automatically.

---

## 7. Readium Unified Locator Model (`ReadiumLocator`)

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

## 8. Readium Search Web Service API

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

## 9. Full-Text & Regex Search Engine (`ebook_rs::SearchEngine`)

`ebook-rs` provides literal string search and regular expression pattern search with `<mark>` context highlighting:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;

// 1. Literal Search (default cap: 500 results)
let literal_results = book.search("quantum");

// 1b. Bounded Literal Search (enforces cap during scanning; None => 500, clamped at 5000)
let limited_results = book.search_limited("quantum", Some(25));
// Also available directly on slices of sections:
let engine_results = ebook_rs::SearchEngine::search_limited(book.sections(), "quantum", false, Some(25));

// 2. Regular Expression Search (case-insensitive regex)
let regex_results = book.search_regex("(?i)quantum|physics|particle")?;

// 2b. Bounded Regular Expression Search
let limited_regex = book.search_regex_limited("(?i)quantum|physics", Some(25))?;
let engine_regex = ebook_rs::SearchEngine::search_regex_limited(book.sections(), "(?i)quantum", Some(25))?;

for match_item in regex_results {
    println!("Spine Index: {}", match_item.spine_index);
    println!("CFI: {}", match_item.cfi);
    println!("Context Snippet: {}", match_item.snippet); // "...<mark>quantum</mark> physics..."
}
```

---

## 10. Structural EPUB Validator (`ebook_rs::EpubValidator`)

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

## 11. Book Fingerprinting & Deduplication (`ebook_rs::BookFingerprint`)

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

## 12. Academic Citation Exporter (`ebook_rs::CitationExporter`)

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

## 13. Code Block Syntax Highlighter & Extractor (`ebook_rs::SyntaxHighlightEngine`)

Extract code blocks (`<pre><code>`) and tokenize line-level syntax nodes (`SyntaxNodeInfo`) across technical eBooks and documentation:

```rust
use ebook_rs::{Book, SyntaxHighlightEngine};

let book = Book::from_file("rust_guide.md")?;

// Extract all code blocks with tokenized syntax nodes
let blocks = book.extract_code_blocks();
for block in blocks {
    println!("Language: {}", block.language);
    println!("Code: {}", block.code);
    for node in block.ast_nodes {
        println!("Syntax Node: {} [byte {}-{}]", node.kind, node.start_byte, node.end_byte);
    }
}

// Tokenize standalone code snippet
let nodes = SyntaxHighlightEngine::parse_code("fn main() {}", "rust");
```

---

## 14. Synthetic FXL 2-Page Spreads (`SyntheticSpread`)

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

## 15. Table of Contents Deep Search & Flattening (`NavPoint::search`, `NavPoint::flatten`)

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

## 16. SpeechSynthesis TTS Word Synchronizer (`TtsWordToken`, `get_tts_tokens`)

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

// Or call directly on a `Section`:
let sec_tokens = section.tokenize_tts_words();
let sec_tts_html = section.to_tts_annotated_html();
```

---

## 17. Universal EPUB 3 Exporter (`book.export_epub3_bytes()`)

Convert ANY parsed eBook format (MOBI, AZW3, FB2, LIT, CBZ, PDF, ODT, DOCX, RTF, TXT, MD) into a clean, compliant EPUB 3 ZIP archive buffer:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.mobi")?;

// Convert MOBI to EPUB 3 bytes
let epub_bytes: Vec<u8> = book.export_epub3_bytes()?;
std::fs::write("converted_sample.epub", epub_bytes)?;
```

---

## 18. Zero-Copy Memory-Mapped I/O (`Book::from_mmap`)

Load multi-gigabyte eBook files with instantaneous startup using kernel memory paging:

```rust
use ebook_rs::Book;

let book = Book::from_mmap("huge_volume.epub")?;
println!("Sections loaded: {}", book.sections().len());
```

---

## 19. Lightweight DOM AST Tree (`EbookDomTree`, `DomNode`)

Zero-dependency DOM tree generator for structural manipulation and text extraction:

```rust
use ebook_rs::dom::EbookDomTree;

let html = "<div class='chapter'><h1>Title</h1><p>Hello world</p></div>";
let tree = EbookDomTree::parse(html);
let clean_html = tree.to_html();

// CSS selector query (tag, #id, .class, [attr], [attr=val], combinators, comma groups)
let headings = tree.query_selector_all("article.post > h1, h2.title");

// Resolve applicable styles for a node (specificity-ordered, inline style wins)
if let Some(node) = headings.first() {
    let styles = tree.style_of(node);
    println!("{:?}", styles.get("color"));
}
```

---

## 20. Fuzzy XML / HTML Recovery Parser (`sanitize_and_repair_xml`)

Lenient recovery sanitizer repairing unescaped ampersands (`&`), unclosed tags, and malformed entities for 100% parse success rates:

```rust
use ebook_rs::sanitize_and_repair_xml;

let broken_xml = "<package><title>AT&T & R&D Guide</title></package>";
let repaired = sanitize_and_repair_xml(broken_xml);
```

---

## 20b. HTML Character Reference Decoding (`decode_html_entities`)

Inverse of `xml_escape`. Several format parsers (FB2, PDF, ODT, DOCX, RTF, CBZ) build their section
HTML by escaping text nodes, so plain text derived from that HTML would otherwise contain literal
`&apos;` / `&amp;` entities. Decoding is a **single** left-to-right pass, so `&amp;amp;` correctly
decodes to `&amp;` rather than looping:

```rust
use ebook_rs::decode_html_entities;

assert_eq!(decode_html_entities("it&apos;s &quot;quoted&quot;"), "it's \"quoted\"");
assert_eq!(decode_html_entities("a &amp; b"), "a & b");
assert_eq!(decode_html_entities("&#65;&#x42;"), "AB");
assert_eq!(decode_html_entities("&amp;amp;"), "&amp;");

// Malformed, unknown, and out-of-range references are left verbatim rather than dropped.
assert_eq!(decode_html_entities("5 &lt 6"), "5 &lt 6");
assert_eq!(decode_html_entities("&#xD800;"), "&#xD800;");
```

Attribute-value-only references (`&colon;`, `&Tab;`, …) are deliberately **not** decoded here; URI
scheme probing uses its own narrower decoder.

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

## 23. Compressed State Caching (`export_cache`, `from_cache`)

Compress and restore parsed book states in pure Rust (`flate2` + `zlib-rs`) for high-throughput server caching and WebAssembly state persistence:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;

// Export pure-Rust compressed state cache
let cache_bytes: Vec<u8> = book.export_cache()?;

// Instantly restore parsed Book from cache
let restored_book = Book::from_cache(&cache_bytes)?;
```

> **Deprecated (`zstd` feature):** `export_zstd_cache` and `from_zstd_cache` are deprecated and no longer enabled by default (since `zstd` links C `libzstd`). Opt in via `--features zstd` only if you need legacy `.zst` cache compatibility.

---

## 24. Performance Accelerators (`compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot`)

`ebook-rs` integrates 6 SIMD and stack-optimization performance crates:
- **`compact_str`**: Small String Optimization (`CompactString`) storing strings <= 24 bytes directly on the stack to eliminate heap allocations.
- **`ahash`**: `AHashMap` & `AHashSet` for 3x-5x faster hash lookups in OPF manifest, DOM attributes, and annotations.
- **`simdutf8`**: SIMD-accelerated UTF-8 validation (`simdutf8::basic::from_utf8`) for 10x-20x faster HTML/XML byte-stream decoding.
- **`zlib-rs`**: `flate2` with `zlib-rs` SIMD zlib decompression for 3x faster EPUB/CBZ ZIP archive reading.
- **`memchr`**: SIMD substring searching (`memchr::memchr` / `memchr::memmem`) for ultra-fast HTML tag scanning, attribute extraction, and script stripping.
- **`parking_lot`**: Fast 1-byte non-poisoning mutex locks (`parking_lot::Mutex`) for concurrent section caches.

---

## 25. Multi-Threaded Parallel ZIP Exporter (`UniversalEpub3Exporter`)

Parallelizes HTML section document and image asset compression across Rayon worker threads when `parallel` feature is enabled (`entries.par_iter()`), significantly accelerating EPUB 3 export performance for 100MB+ image-heavy books:

```rust
use ebook_rs::{Book, UniversalEpub3Exporter};

let book = Book::from_file("large_comic.cbz")?;
// Exports EPUB 3 zip buffer using multi-threaded compression
let epub_bytes = UniversalEpub3Exporter::export(&book)?;
```

---

## 26. CBZ Comic Pre-fetching & 2-Page Manga Spread Mode (`CbzBook`)

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

## 27. Native PyO3 Python Bindings (`pip install ebook-rs`)

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

## 28. PDF Two-Column Reading-Order Reflow (`reflow_two_column_markdown`)

Detects multi-column line patterns and spatial column dividers in IEEE, ArXiv, and ACM academic paper PDFs, sorting Left-Column paragraphs top-to-bottom followed by Right-Column paragraphs top-to-bottom into continuous single-column EPUB sections:

```rust
use ebook_rs::pdf::reflow_two_column_markdown;

let raw_pdf_md = "Title\n\nLeft Col 1   |   Right Col 1\nLeft Col 2   |   Right Col 2";
let single_column_md = reflow_two_column_markdown(raw_pdf_md);
```

---

## 29. Mozilla UniFFI Mobile & Native Bindings (`UniBook`, `UniSearchResult`)

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

## 30. Lossless EPUB 3 Optimizer & Minifier (`EpubOptimizer`, `EpubOptimizerOptions`)

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

## 31. CJK Vertical Writing & RTL Reflow Pagination (`ReflowPaginator`, `WritingMode`)

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

## 32. Web Audio API & SMIL Karaoke Cue Sheets (`KaraokeCueSheet`, `WebAudioCue`)

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

## 33. Lazy Archive Decompression for Giant Files (>500MB)

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

## 34. Markdown Frontmatter, Obsidian Wikilinks & Callouts

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

## 35. Microsoft Word (.docx) Parser (`DocxBook`)

`DocxBook` parses Microsoft Word (.docx) Office Open XML ZIP packages into structured `Book` instances, extracting WordprocessingML headings, paragraphs, formatted text runs (`<strong>`, `<em>`, `<u>`, `<s>`), tables, embedded media images, and Dublin Core metadata:

```rust
use ebook_rs::{Book, DocxBook};

// 1. Direct DOCX parsing
let docx_bytes = std::fs::read("document.docx")?;
let book = DocxBook::parse(&docx_bytes, "Fallback Title")?;

println!("Title: {}", book.metadata().title);
println!("Authors: {:?}", book.metadata().creators);
println!("Sections: {}", book.sections().len());

// 2. Auto-detection via Book::from_bytes
let book = Book::from_bytes(&docx_bytes)?;

// 3. Export to EPUB 3
let epub_bytes = book.export_epub3_bytes()?;
```

---

## 36. Rich Text Format (.rtf) Parser (`RtfBook`)

`RtfBook` parses Microsoft Rich Text Format (.rtf) documents using a streaming control-word tokenizer, managing nested group formatting state stacks, code page charset conversions, Unicode character escapes (`\uN`), tables (`\trowd`/`\cell`/`\row`), and embedded pictures:

```rust
use ebook_rs::{Book, RtfBook};

// 1. Direct RTF parsing
let rtf_bytes = std::fs::read("manuscript.rtf")?;
let book = RtfBook::parse(&rtf_bytes, "Default Title")?;

println!("Title: {}", book.metadata().title);
println!("Sections: {}", book.sections().len());

// 2. Auto-detection via Book::from_bytes
let book = Book::from_bytes(&rtf_bytes)?;

// 3. Perform zero-allocation SIMD search across RTF chapters
let results = book.search("protagonist");
```

---

## 37. Universal 28-Format Pure-Rust Exporter (`Book::export_format_bytes`, `ebook_rs::formats`)

Any loaded `Book` can be exported in pure Rust (zero external binaries) to **28+ formats** (29 format identifiers in `Book::supported_output_formats()`) via typed methods or dynamic format dispatch:

```rust
use ebook_rs::Book;

let book = Book::from_file("sample.epub")?;

// Dynamic dispatch by extension/name:
for fmt in Book::supported_output_formats() {
    let bytes = book.export_format_bytes(fmt)?;
    println!("{fmt}: {} bytes", bytes.len());
}

// Or typed format exporters:
let epub  = book.export_epub3_bytes()?;    // EPUB 3 ZIP archive
let kepub = book.export_kepub_bytes()?;    // Kobo EPUB 3 with koboSpan instrumentation
let kfx   = book.export_kfx_bytes()?;      // Clean-room Amazon KFX (KF10)
let mobi  = book.export_mobi_bytes()?;     // Mobipocket (.mobi) + PalmDOC LZ77 + EXTH + images
let azw3  = book.export_azw3_bytes()?;     // Amazon Kindle (.azw3) + EXTH + images
let pdb   = book.export_pdb_bytes()?;      // PalmDOC (.pdb) LZ77
let lit   = book.export_lit_bytes()?;      // Microsoft Reader (.lit)
let pdf   = book.export_pdf_bytes()?;      // Pure-Rust paginated PDF 1.4
let fb2   = book.export_fb2_bytes()?;      // FictionBook 2.0 XML + base64 <binary> images
let html  = book.export_html_bytes()?;     // Standalone HTML5 with inlined data: images
let htmlz = book.export_htmlz_bytes()?;    // Calibre HTMLZ archive
let txt   = book.export_txt_bytes()?;      // UTF-8 Plain Text
let txtz  = book.export_txtz_bytes()?;     // Calibre TXTZ archive
let md    = book.export_markdown_bytes()?; // Markdown + YAML frontmatter + GFM tables
let oeb   = book.export_oeb_bytes()?;      // Zipped Open eBook package
let zip   = book.export_zip_bytes()?;      // Calibre-compatible HTML/CSS/Images ZIP
let tcr   = book.export_tcr_bytes()?;      // Psion TCR (!!8-Bit!!) 256-entry dictionary
let docx  = book.export_docx_bytes()?;     // OOXML WordprocessingML + <w:drawing> images
let odt   = book.export_odt_bytes()?;      // OpenDocument Text 1.2 + Pictures/*
let rtf   = book.export_rtf_bytes()?;      // RTF 1.x + \uN? Unicode + \pict images
let pptx  = book.export_pptx_bytes()?;     // PowerPoint OOXML (.pptx) slides
let pmlz  = book.export_pmlz_bytes()?;     // Palm Markup Language ZIP
let rb    = book.export_rb_bytes()?;       // Rocket eBook (NUVO + chunked zlib HTML)
let typ   = book.export_typst_bytes()?;    // Typst (.typ / .typst)
let adoc  = book.export_adoc_bytes()?;     // AsciiDoc (.adoc)
let org   = book.export_org_bytes()?;      // Emacs Org-mode (.org)
let rst   = book.export_rst_bytes()?;      // reStructuredText (.rst)
let tex   = book.export_tex_bytes()?;      // LaTeX (.tex)
book.export_oeb_dir("out.oeb")?;           // Unzipped Open eBook directory (content.opf + assets)
```

Python (`PyO3`) and `UniBook` (UniFFI) bindings also expose `export_format_bytes("docx")` (and any other supported format).

---

## 38. MCP Server (`convert_ebook`, `export_ebook_text`, & Tools/Prompts)

The built-in Model Context Protocol server (`src/mcp.rs`, protocol `2024-11-05`, feature `mcp`) exposes tools, resources, and prompts for AI agents (with filesystem tools strictly confined to `--sandbox-root`):

- **Tools**:
  - `get_metadata`, `get_toc`, `read_section`, `search_book`, `chunk_book_for_rag` (with optional Okapi `query_rank` BM25 scoring), `validate_epub`.
  - `convert_ebook`: Converts any supported input eBook or OEB directory to `.epub`, `.kepub`, `.kepub.epub`, `.kfx`, `.mobi`, `.azw3`, `.pdb`, `.pdf`, `.fb2`, `.html`, `.htmlz`, `.txt`, `.txtz`, `.md`, `.oeb`, `.zip`, `.tcr`, `.docx`, `.odt`, `.rtf`, `.pptx`, `.pmlz`, `.rb`, `.typ`, `.typst`, `.adoc`, `.org`, `.rst`, `.tex`, or RAG `.json`.
  - `export_ebook_text`: Renders any supported eBook directly to inline UTF-8 `md`, `txt`, `html`, `fb2`, `typst`, `adoc`, `org`, `rst`, or `tex` without writing a file (supports optional `max_chars`, default `100_000`).
- **Resources**: `ebook://info` (reports `version`, `supported_input_formats`, `supported_output_formats`, and `features`), `ebook://{path}/metadata`, `ebook://{path}/toc`.
- **Prompts**: `summarize_book`, `extract_entities`, `generate_study_guide`.

---

## 39. AI / RAG Document Chunking & Okapi BM25 Ranking (`RagChunker`, `RagChunkConfig`)

Split books into semantic chunks with heading-hierarchy trails, EPUB CFI citation anchors, token estimates, and Okapi BM25 relevance ranking:

```rust
use ebook_rs::{Book, RagChunkConfig, RagChunker};

let book = Book::from_file("sample.epub")?;
let config = RagChunkConfig {
    max_tokens: 512,
    overlap_tokens: 64,
    preserve_headings: true,
    include_cfi: true,
    min_chunk_size: 50,
}
.sanitized();

let chunks = book.to_rag_chunks(&config);
let top_hits = RagChunker::rank_chunks_bm25(&chunks, "quantum mechanics", 5);
for hit in top_hits {
    println!("[score={:.3}] {} ({})", hit.score, hit.chunk.chapter_title, hit.chunk.cfi);
}
```

---

## 40. Clean-Room Amazon KFX Codec (`KfxBook`, `UniversalKfxExporter`)

Pure-Rust Amazon KFX (`CONT` container + Ion v2 binary codec) reader and exporter supporting fonts, `font-family`, bidi/RTL, tables, links, TOC, and cover images:

```rust
use ebook_rs::{Book, KfxBook, UniversalKfxExporter, KFX_MAGIC_CONT};

let kfx_bytes = std::fs::read("sample.kfx")?;
if KfxBook::is_kfx(&kfx_bytes) {
    let book = KfxBook::parse(&kfx_bytes)?;
    println!("Loaded KFX: {}", book.metadata().title);
}

let epub = Book::from_file("sample.epub")?;
let exported_kfx: Vec<u8> = UniversalKfxExporter::export(&epub)?;
```

---

## 41. Annotations & Highlights (`AnnotationManager`, `Annotation`)

Manage highlights, bookmarks, and notes keyed by EPUB CFI with W3C Web Annotation JSON export:

```rust
use ebook_rs::Book;

let mut book = Book::from_file("sample.epub")?;
let hl = book.annotations_mut().create_highlight(
    "epubcfi(/6/4!/4/2/1:10)",
    "highlighted passage",
    Some("#ffeb3b"),
    Some("Key quote"),
);
let bm = book.annotations_mut().create_bookmark("epubcfi(/6/6!/4/2/1:0)", Some("Resume here"));

let w3c_json = book.annotations().to_w3c_json()?;
```

---

## 42. Reading Analytics & Footnotes (`ReadingAnalytics`, `Footnote`)

Compute word counts, estimated reading time, difficulty score, top keywords, and extract structured HTML footnotes:

```rust
use ebook_rs::ReadingAnalytics;
use ebook_rs::footnote::parse_footnotes_from_html;

let stats = ReadingAnalytics::analyze_text(&section.plain_text);
println!("Words: {}, Est. minutes: {:.1}, Difficulty: {:.1}", stats.word_count, stats.reading_time_minutes, stats.difficulty_score);

let footnotes = section.extract_footnotes();
// Equivalent low-level helper on raw HTML:
let footnotes_raw = parse_footnotes_from_html(&section.raw_html);
```

---

## 43. Readium WebPub & OPDS Feeds (`WebpubManifest`, `OpdsFeed`)

Export books as Readium WebPub Manifests and parse OPDS 1.x Atom XML or OPDS 2.0 JSON catalogs (`opds` feature):

```rust
use ebook_rs::{Book, OpdsFeed};

let book = Book::from_file("sample.epub")?;
let manifest = book.to_webpub_manifest();
let manifest_json = book.to_webpub_json()?;

let feed = OpdsFeed::parse_atom_xml(atom_xml)?;
let json_feed = OpdsFeed::parse_json(opds2_json)?;
for entry in &feed.entries {
    if let Some(link) = entry.download_link(Some("application/epub+zip")) {
        println!("{} -> {}", entry.title, link.href);
    }
}
```

---

## 44. Font Deobfuscation & HTTP Range Streaming (`FontDeobfuscator`, `ZipHeaderReader`)

Deobfuscate IDPF and Adobe obfuscated fonts via `META-INF/encryption.xml`, and inspect remote ZIP central directories over HTTP Range requests:

```rust
use ebook_rs::{FontDeobfuscator, ZipHeaderReader};

let deobf = FontDeobfuscator::parse_encryption_xml(encryption_xml);
if deobf.is_encrypted("OEBPS/fonts/custom.woff2") {
    deobf.deobfuscate("OEBPS/fonts/custom.woff2", &mut font_bytes, "urn:uuid:...");
}

let entries = ZipHeaderReader::parse_central_directory(&tail_bytes)?;
```

---

## 45. Rendition Layout & Before-Display Hooks (`RenditionLayout`, `register_before_display_hook`)

Configure viewport, theme, custom fonts, script stripping, asset delivery strategy, and HTML transformation hooks (automatically invalidating `render_cache`):

```rust
use ebook_rs::{Book, RenditionLayout};

let mut book = Book::from_file("sample.epub")?;
let mut layout = RenditionLayout::default();
layout.set_custom_font("Literata", "https://example.com/literata.woff2");
book.set_layout(layout);

book.register_before_display_hook(|html, _href| {
    html.push_str("<!-- injected -->");
});
```

---

## 46. Embedded HTTP Reader Server & CLI (`ReaderServer`, `ebook-rs`)

With the `server` feature enabled, `ReaderServer` serves a built-in web reader, REST/Readium endpoints, and HTTP `/api/mcp` JSON-RPC with optional bearer-token auth and MCP filesystem sandboxing:

```rust
use ebook_rs::{Book, ReaderServer};

let book = Book::from_file("sample.epub")?;
ReaderServer::new(book, 8080)
    .try_with_auth_token("secret-token")?
    .with_sandbox_root("./books")
    .listen()?;
```

The `ebook-rs` CLI binary exposes: `serve`, `convert`, `parse`, `search`, `locations`, `rag`, `sample`, and `mcp`.

---

## 47. WebAssembly & C FFI ABI Bindings (`WasmBook`, `ebook_rs::ffi`)

- **WASM (`wasm` feature)**: `ebook_rs::WasmBook` exposes `new`, `get_metadata_json`, `get_toc_json`, `get_spine_json`, `get_section_html`, `get_tts_words_json`, `get_tts_annotated_html`, `search_json`, `cfi_to_spine_index`, `get_resource_bytes`, `get_section_analytics_json`, `paginate_section_json`, `resolve_cfi_dom_json`, `set_custom_font`, `to_rag_chunks_json`, `get_webpub_manifest_json`, `export_epub_bytes`, `enable_manga_mode`, and `WasmBook::get_custom_element_js()`.
- **C FFI (`ebook_rs::ffi`)**: Exposes `ebook_rs_book_from_bytes`, `ebook_rs_book_free`, `ebook_rs_get_metadata_json`, `ebook_rs_get_toc_json`, `ebook_rs_get_section_html`, `ebook_rs_search_json`, `ebook_rs_to_rag_chunks_json`, and `ebook_rs_string_free`.

---

## 48. Sample EPUB Generator (`sample_builder::generate_sample_epub`)

Generate a complete, valid in-memory EPUB 3 archive (with TOC, OPF, NCX, NAV, stylesheet, and chapters) for tests, demos, or CLI `sample` output:

```rust
use ebook_rs::{Book, sample_builder::generate_sample_epub};

let epub_bytes: Vec<u8> = generate_sample_epub()?;
let book = Book::from_bytes(&epub_bytes)?;
println!("Generated sample book: {}", book.title());
```
