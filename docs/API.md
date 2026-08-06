# 📚 EBook-RS API Documentation & Wiki (v0.4.0)

Welcome to the **`ebook-rs`** API Documentation and Developer Wiki.

---

## 📑 Table of Contents

1. [Core Reader Engine (`Book`)](#1-core-reader-engine-book)
2. [Section & Footnotes API (`Section`, `Footnote`)](#2-section--footnotes-api-section-footnote)
3. [Rendition & Layout Configuration (`RenditionLayout`, `AssetDeliveryStrategy`)](#3-rendition--layout-configuration-renditionlayout-assetdeliverystrategy)
4. [OPDS Catalog Feed Client (`OpdsFeed`, `OpdsEntry`)](#4-opds-catalog-feed-client-opdsfeed-opdsentry)
5. [CFI Engine (`Cfi`)](#5-cfi-engine-cfi)
6. [Annotations & Locations (`AnnotationManager`, `Locations`)](#6-annotations--locations-annotationmanager-locations)
7. [WebAssembly (`wasm-bindgen`) & HTTP Web UI Integration](#7-webassembly-wasm-bindgen--http-web-ui-integration)

---

## 1. Core Reader Engine (`Book`)

`Book` is the main entry point for loading, parsing, and searching eBook files.

### Opening eBooks

```rust
use ebook_rs::Book;

// Load from a file path (auto-detects EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ)
let book = Book::from_file("book.epub")?;

// Load from an in-memory byte slice
let bytes = std::fs::read("comic.cbz")?;
let book = Book::from_bytes(&bytes)?;
```

### Retrieving Metadata, Spine, and Navigation

```rust
let metadata = book.metadata();
println!("Title: {}", metadata.title);
println!("Creators: {:?}", metadata.creators);

let spine = book.spine();
println!("Total Chapters/Pages: {}", spine.len());

let toc = book.toc();
for point in toc {
    println!("TOC: {} -> {}", point.label, point.href);
}
```

### Retrieving Chapter Sections

```rust
// Get section by 0-based spine index
let section = book.get_section(0)?;

// Get section by relative href string
let section = book.get_section_by_href("OEBPS/ch1.xhtml")?;
```

---

## 2. Section & Footnotes API (`Section`, `Footnote`)

### `Section` Methods

```rust
let mut section = book.get_section(0)?;

// HTML ready for display
println!("HTML: {}", section.processed_html);

// Extracted clean plain text
println!("Plain Text: {}", section.plain_text);

// Strip embedded <script> blocks and inline event handlers for XSS security
section.strip_script_content();

// Extract EPUB 3 noteref footnotes & endnotes
let footnotes = section.extract_footnotes();
for fn_item in footnotes {
    println!("Footnote [{}]: {}", fn_item.label, fn_item.plain_text);
}
```

### `Footnote` Model

| Field | Type | Description |
|---|---|---|
| `id` | `String` | Reference element ID |
| `href` | `String` | Target anchor URI (`#fn1`) |
| `target_id` | `String` | Target container DOM ID |
| `label` | `String` | Display label (`"1"`) |
| `html_content` | `String` | Full footnote target HTML |
| `plain_text` | `String` | Extracted plain text for popup preview |

---

## 3. Rendition & Layout Configuration (`RenditionLayout`, `AssetDeliveryStrategy`)

### Asset Delivery Strategies

- **`AssetDeliveryStrategy::InlinedBase64` (Default)**: Inlines all images/fonts/CSS as Base64 Data URIs (`data:image/png;base64,...`).
- **`AssetDeliveryStrategy::ResourceStream`**: Keeps resource paths as clean relative URLs (`src="resource/images/p1.jpg"`), eliminating 33% Base64 size inflation on 100MB+ books.

```rust
use ebook_rs::{Book, AssetDeliveryStrategy, FlowMode, Theme};

let mut book = Book::from_file("huge_comic.cbz")?;

// Configure Layout Strategy
book.layout.asset_delivery = AssetDeliveryStrategy::ResourceStream;
book.layout.theme = Theme::Dark;
book.layout.flow_mode = FlowMode::Scrolled;
book.layout.allow_scripted_content = false;

// Calculate Fixed Layout (FXL) Aspect Ratio Scale Matrix
if let Some((scale, css_transform)) = book.layout.compute_fxl_scale(1024.0, 768.0, 512.0, 384.0) {
    println!("FXL Scale: {}", scale);
    println!("CSS: {}", css_transform);
}
```

---

## 4. OPDS Catalog Feed Client (`OpdsFeed`, `OpdsEntry`)

Gated under the optional `opds` feature flag in `Cargo.toml`.

```toml
[dependencies]
ebook-rs = { version = "0.4.0", features = ["opds"] }
```

### Parsing OPDS 1.2 (Atom XML) & OPDS 2.0 (JSON) Feeds

```rust
use ebook_rs::OpdsFeed;

// Parse OPDS 1.2 Atom XML Feed (e.g. Standard Ebooks)
let xml_feed = OpdsFeed::parse_atom_xml(&xml_str)?;

for entry in &xml_feed.entries {
    println!("Book Title: {}", entry.title);
    println!("Authors: {:?}", entry.authors);

    if let Some(link) = entry.download_link(Some("application/epub+zip")) {
        println!("Download EPUB URL: {}", link.href);
    }
}

// Parse OPDS 2.0 JSON Feed
let json_feed = OpdsFeed::parse_json(&json_str)?;
```

---

## 5. CFI Engine (`Cfi`)

`Cfi` provides 100% IDPF Canonical Fragment Identifier spec compliance.

```rust
use ebook_rs::Cfi;

// Parse EPUB CFI
let cfi = Cfi::parse("epubcfi(/6/4[chap01]!/4/2/1:0)")?;

println!("Spine Index: {}", cfi.spine_index()); // 0-based spine index

// Create CFI range
let cfi_start = Cfi::parse("epubcfi(/6/4!/4/2/1:0)")?;
let cfi_end = Cfi::parse("epubcfi(/6/4!/4/2/1:25)")?;
let range_cfi = Cfi::create_range(&cfi_start, &cfi_end)?;
```

---

## 6. Annotations & Locations (`AnnotationManager`, `Locations`)

```rust
use ebook_rs::{AnnotationManager, AnnotationType, Cfi};

let mut annotations = AnnotationManager::new();

// Create highlight
let cfi = Cfi::parse("epubcfi(/6/4!/4/2/1:0)")?;
annotations.add_annotation(cfi, AnnotationType::Highlight, Some("#ffff00".to_string()), Some("Key quote".to_string()));

// Serialize to JSON for persistence
let json = annotations.to_json()?;
```

---

## 7. WebAssembly (`wasm-bindgen`) & HTTP Web UI Integration

### WASM Integration (`wasm-bindgen`)

```javascript
import { WasmBook } from "ebook-rs";

// Initialize WASM book from Uint8Array
const book = new WasmBook(uint8Array);

const metadata = JSON.parse(book.get_metadata_json());
const html = book.get_section_html(0);

// Get raw resource byte array for Blob URL creation
const coverBytes = book.get_resource_bytes("OEBPS/images/cover.png");
const blob = new Blob([coverBytes], { type: "image/png" });
const blobUrl = URL.createObjectURL(blob);
```

### HTTP Reader App Server (`tiny_http`)

```rust
use ebook_rs::ReaderServer;

let server = ReaderServer::new("127.0.0.1:8080", "book.epub")?;
println!("Server listening on http://127.0.0.1:8080");
server.listen()?;
```
