# 📚 EBook-RS Complete API Reference Guide (v0.5.0)

Welcome to the comprehensive, exhaustive API Reference Guide for **`ebook-rs`** (v0.5.0).

---

## 📑 Table of Contents

1. [Core Reader Engine (`Book`)](#1-core-reader-engine-book)
2. [Section & NLP Reading Analytics (`Section`, `ReadingAnalytics`)](#2-section--nlp-reading-analytics-section-readinganalytics)
3. [Deterministic Reflow Paginator (`ReflowPaginator`, `SectionPageMap`)](#3-deterministic-reflow-paginator-reflowpaginator-sectionpagemap)
4. [Remote ZIP Central Directory Streamer (`ZipHeaderReader`, `HttpRangeRequest`)](#4-remote-zip-central-directory-streamer-zipheaderreader-httprangerequest)
5. [Footnote & Endnote Previewer (`Footnote`)](#5-footnote--endnote-previewer-footnote)
6. [OPDS Catalog Feed Client (`OpdsFeed`, `OpdsEntry`, `OpdsLink`)](#6-opds-catalog-feed-client-opdsfeed-opdsentry-opdslink)
7. [Rendition & Layout Configuration (`RenditionLayout`, `AssetDeliveryStrategy`)](#7-rendition--layout-configuration-renditionlayout-assetdeliverystrategy)
8. [CFI Engine (`Cfi`, `CfiPath`, `CfiStep`, `CfiOffset`)](#8-cfi-engine-cfi-cfipath-cfistep-cfioffset)
9. [Annotations & Locations (`AnnotationManager`, `Locations`)](#9-annotations--locations-annotationmanager-locations)
10. [Font De-Obfuscation (`FontDeobfuscator`)](#10-font-de-obfuscation-fontdeobfuscator)
11. [Multi-Format Engine Support (`CbzBook`, `MobiBook`, `Fb2Book`, `LitBook`)](#11-multi-format-engine-support-cbzbook-mobibook-fb2book-litbook)
12. [WebAssembly Bindings (`WasmBook`)](#12-webassembly-bindings-wasmbook)
13. [Embedded HTTP Reader Server (`ReaderServer`)](#13-embedded-http-reader-server-readerserver)

---

## 1. Core Reader Engine (`Book`)

`Book` is the central engine for auto-detecting, parsing, and searching eBook files.

### Constructors

```rust
use ebook_rs::Book;

// Auto-detects and loads EPUB, MOBI, AZW3, FB2, KEPUB, LIT, or CBZ from file path
pub fn from_file(path: &str) -> Result<Self, String>;

// Opens an eBook from an in-memory byte slice
pub fn from_bytes(bytes: &[u8]) -> Result<Self, String>;

// Opens an eBook with a title fallback
pub fn from_bytes_with_title(bytes: &[u8], title_fallback: &str) -> Result<Self, String>;
```

### Retrieval & Inspection Methods

```rust
// Access OPF Metadata (title, creators, language, publisher, rights, cover_href)
pub fn metadata(&self) -> &Metadata;

// Access Spine item manifest list
pub fn spine(&self) -> &[SpineItem];

// Access Table of Contents navigation tree
pub fn toc(&self) -> &[NavPoint];

// Access EPUB 3 Landmarks navigation list
pub fn landmarks(&self) -> &[Landmark];

// Access EPUB 3 Page List navigation items
pub fn page_list(&self) -> &[PageListItem];

// Retrieve raw cover image bytes and MIME type
pub fn cover_image(&self) -> Option<(Vec<u8>, &'static str)>;

// Get section by 0-based spine index
pub fn get_section(&self, index: usize) -> Result<Section, String>;

// Get section by relative href path string
pub fn get_section_by_href(&self, href: &str) -> Result<Section, String>;

// Get raw resource bytes (images, styles, fonts) by relative archive path
pub fn get_resource_bytes(&self, path: &str) -> Result<(Vec<u8>, &'static str), String>;

// Perform full-text search across all chapters (returns 0-alloc SearchResult matches)
pub fn search(&self, query: &str) -> Vec<SearchResult>;

// Register a pre-display transformation hook for HTML modifications
pub fn register_before_display_hook<F>(&mut self, hook: F)
where
    F: Fn(&mut String, &str) + Send + Sync + 'static;

// Export Readium Webpub JSON manifest (application/webpub+json)
pub fn to_webpub_manifest(&self) -> WebpubManifest;
```

---

## 2. Section & NLP Reading Analytics (`Section`, `ReadingAnalytics`)

### `Section` Struct

```rust
pub struct Section {
    pub index: usize,
    pub idref: String,
    pub href: String,
    pub full_path: String,
    pub raw_html: String,
    pub processed_html: String,
    pub plain_text: String,
    pub plain_text_lower: String,
    pub char_count: usize,
    pub viewport_width: Option<f64>,
    pub viewport_height: Option<f64>,
}
```

### `Section` Methods

```rust
// Strip <script> blocks, inline on*="..." event attributes, and javascript: links for XSS protection
pub fn strip_script_content(&mut self);

// Extract footnotes and endnotes into Footnote models for popup previews
pub fn extract_footnotes(&self) -> Vec<Footnote>;

// Calculate NLP Reading Analytics for this section
pub fn analytics(&self) -> ReadingAnalytics;

// Calculate virtual reflow page break map for this section
pub fn paginate(&self, paginator: Option<&ReflowPaginator>) -> SectionPageMap;
```

### `ReadingAnalytics` Struct

```rust
pub struct ReadingAnalytics {
    pub word_count: usize,
    pub reading_time_minutes: f32,
    pub difficulty_score: f32,          // 0.0 (easy) to 10.0 (complex)
    pub top_keywords: Vec<(String, usize)>, // Frequency-sorted keywords (stopwords excluded)
}
```

---

## 3. Deterministic Reflow Paginator (`ReflowPaginator`, `SectionPageMap`)

Calculates line wraps and page boundaries without requiring a web browser or DOM reflow.

```rust
pub struct ReflowPaginator {
    pub font_size_px: u32,
    pub line_height: f32,
    pub viewport_width_px: u32,
    pub viewport_height_px: u32,
    pub margin_px: u32,
}

impl ReflowPaginator {
    pub fn new(font_size: u32, line_height: f32, width: u32, height: u32, margin: u32) -> Self;
    pub fn paginate_text(&self, text: &str) -> SectionPageMap;
    pub fn paginate_section(&self, section: &Section) -> SectionPageMap;
}

pub struct SectionPageMap {
    pub total_pages: usize,
    pub page_ranges: Vec<PageRange>,
}

pub struct PageRange {
    pub page_number: usize,
    pub start_char: usize,
    pub end_char: usize,
}
```

---

## 4. Remote ZIP Central Directory Streamer (`ZipHeaderReader`, `HttpRangeRequest`)

### `ZipHeaderReader`

```rust
pub struct ZipHeaderReader;

impl ZipHeaderReader {
    // Locate End of Central Directory (EOCD) signature PK\x05\x06 from tail byte slice
    pub fn find_eocd(bytes: &[u8]) -> Option<usize>;

    // Parse Central Directory index entries into ZipEntryLocation list
    pub fn parse_central_directory(tail_bytes: &[u8]) -> Result<Vec<ZipEntryLocation>, String>;
}
```

### `HttpRangeRequest`

```rust
pub struct HttpRangeRequest {
    pub url: String,
    pub start: u64,
    pub end: Option<u64>,
}

impl HttpRangeRequest {
    pub fn new(url: &str, start: u64, end: Option<u64>) -> Self;
    pub fn to_range_header(&self) -> (String, String); // ("Range", "bytes=start-end")
    pub fn parse_range_header(header_val: &str) -> Option<(u64, Option<u64>)>;
}
```

---

## 5. Footnote & Endnote Previewer (`Footnote`)

```rust
pub struct Footnote {
    pub id: String,
    pub href: String,
    pub target_id: String,
    pub label: String,
    pub html_content: String,
    pub plain_text: String,
}

// Global parser helper
pub fn parse_footnotes_from_html(html: &str) -> Vec<Footnote>;
```

---

## 6. OPDS Catalog Feed Client (`OpdsFeed`, `OpdsEntry`, `OpdsLink`)

Gated under `features = ["opds"]`.

```rust
pub struct OpdsFeed {
    pub id: String,
    pub title: String,
    pub updated: Option<String>,
    pub icon: Option<String>,
    pub links: Vec<OpdsLink>,
    pub entries: Vec<OpdsEntry>,
}

impl OpdsFeed {
    // Parse OPDS 1.2 Atom XML Catalog Feed
    pub fn parse_atom_xml(xml: &str) -> Result<Self, String>;

    // Parse OPDS 2.0 JSON Catalog Feed
    pub fn parse_json(json_str: &str) -> Result<Self, String>;
}

pub struct OpdsEntry {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
    pub links: Vec<OpdsLink>,
}

impl OpdsEntry {
    // Find EPUB or format acquisition download link
    pub fn download_link(&self, target_mime: Option<&str>) -> Option<&OpdsLink>;
}
```

---

## 7. Rendition & Layout Configuration (`RenditionLayout`, `AssetDeliveryStrategy`)

```rust
pub enum AssetDeliveryStrategy {
    InlinedBase64,   // Default: Inlines all resources as Base64 Data URIs
    ResourceStream,  // Resource URLs kept as clean relative paths
}

pub struct RenditionLayout {
    pub layout_mode: LayoutMode,
    pub flow_mode: FlowMode,
    pub spread_mode: SpreadMode,
    pub theme: Theme,
    pub font_family: String,
    pub font_size_px: u32,
    pub line_height: f32,
    pub margin_px: u32,
    pub allow_scripted_content: bool,
    pub viewport_config: ViewportManagerConfig,
    pub asset_delivery: AssetDeliveryStrategy,
}

impl RenditionLayout {
    // Compute dynamic CSS override block
    pub fn to_css_override(&self) -> String;

    // Compute FXL Aspect Ratio Scale Factor and CSS matrix string
    pub fn compute_fxl_scale(
        &self,
        vp_width: f64,
        vp_height: f64,
        container_width: f64,
        container_height: f64,
    ) -> Option<(f64, String)>;
}
```

---

## 8. CFI Engine (`Cfi`, `CfiPath`, `CfiStep`, `CfiOffset`)

```rust
pub struct Cfi {
    pub path: CfiPath,
    pub start_offset: Option<CfiOffset>,
    pub range_end: Option<Box<Cfi>>,
}

impl Cfi {
    pub fn parse(cfi_str: &str) -> Result<Self, String>;
    pub fn spine_index(&self) -> usize;
    pub fn create_range(start: &Cfi, end: &Cfi) -> Result<Self, String>;
}
```

---

## 9. Annotations & Locations (`AnnotationManager`, `Locations`)

```rust
pub struct AnnotationManager {
    pub annotations: Vec<Annotation>,
}

impl AnnotationManager {
    pub fn new() -> Self;
    pub fn add_annotation(&mut self, cfi: Cfi, type_: AnnotationType, color: Option<String>, note: Option<String>);
    pub fn remove_annotation(&mut self, id: &str) -> bool;
    pub fn to_json(&self) -> Result<String, String>;
    pub fn from_json(json_str: &str) -> Result<Self, String>;
}
```

---

## 10. Font De-Obfuscation (`FontDeobfuscator`)

```rust
pub struct FontDeobfuscator;

impl FontDeobfuscator {
    pub fn deobfuscate_idpf(data: &mut [u8], identifier: &str);
    pub fn deobfuscate_adobe(data: &mut [u8], identifier: &str);
    pub fn parse_encryption_xml(xml: &str) -> Self;
    pub fn is_encrypted(&self, path: &str) -> bool;
}
```

---

## 11. Multi-Format Engine Support (`CbzBook`, `MobiBook`, `Fb2Book`, `LitBook`)

```rust
// CBZ Comic Reader Engine
pub struct CbzBook;
impl CbzBook {
    pub fn parse(bytes: &[u8], title_fallback: &str) -> Result<Book, String>;
}

// MOBI / AZW3 Reader Engine
pub struct MobiBook;
impl MobiBook {
    pub fn parse(bytes: &[u8]) -> Result<Book, String>;
}

// FB2 Reader Engine
pub struct Fb2Book;
impl Fb2Book {
    pub fn parse(bytes: &[u8]) -> Result<Book, String>;
}

// LIT Reader Engine
pub struct LitBook;
impl LitBook {
    pub fn parse(bytes: &[u8]) -> Result<Book, String>;
}
```

---

## 12. WebAssembly Bindings (`WasmBook`)

Gated under `features = ["wasm"]`.

```rust
#[wasm_bindgen]
pub struct WasmBook;

#[wasm_bindgen]
impl WasmBook {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<WasmBook, JsValue>;
    pub fn get_metadata_json(&self) -> Result<String, JsValue>;
    pub fn get_toc_json(&self) -> Result<String, JsValue>;
    pub fn get_spine_json(&self) -> Result<String, JsValue>;
    pub fn get_section_html(&self, index: usize) -> Result<String, JsValue>;
    pub fn get_resource_bytes(&self, path: &str) -> Result<Vec<u8>, JsValue>;
    pub fn get_section_analytics_json(&self, index: usize) -> Result<String, JsValue>;
    pub fn paginate_section_json(&self, index: usize) -> Result<String, JsValue>;
    pub fn search_json(&self, query: &str) -> Result<String, JsValue>;
    pub fn cfi_to_spine_index(&self, cfi_str: &str) -> Result<usize, JsValue>;
}
```

---

## 13. Embedded HTTP Reader Server (`ReaderServer`)

Gated under `features = ["server"]`.

```rust
pub struct ReaderServer {
    pub addr: String,
    pub book_path: String,
}

impl ReaderServer {
    pub fn new(addr: &str, book_path: &str) -> Result<Self, String>;
    pub fn listen(&self) -> Result<(), String>;
}
```
