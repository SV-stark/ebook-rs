# 📚 EBook-RS API Reference & Complete Documentation

`ebook-rs` is a pure Rust EPUB 2 and EPUB 3 parsing, reading, location-mapping, full-text searching, and rendering engine.

---

## 📋 Table of Contents
1. [Book Core API (`ebook_rs::Book`)](#1-book-core-api-ebook_rsbook)
2. [Section Module (`ebook_rs::Section`)](#2-section-module-ebook_rssection)
3. [EPUB CFI Engine (`ebook_rs::Cfi`)](#3-epub-cfi-engine-ebook_rscfi)
4. [Locations Progress Engine (`ebook_rs::Locations`)](#4-locations-progress-engine-ebook_rslocations)
5. [Annotations Manager (`ebook_rs::AnnotationManager`)](#5-annotations-manager-ebook_rsannotationmanager)
6. [Full-Text Search Engine (`ebook_rs::SearchEngine`)](#6-full-text-search-engine-ebook_rssearchengine)
7. [Layout & Themes (`ebook_rs::RenditionLayout`)](#7-layout--themes-ebook_rsrenditionlayout)
8. [Font De-Obfuscation (`ebook_rs::FontDeobfuscator`)](#8-font-de-obfuscation-ebook_rsfontdeobfuscator)
9. [HTTP Reader Server (`ebook_rs::ReaderServer`)](#9-http-reader-server-ebook_rsreaderserver)
10. [WebAssembly Client API (`ebook_rs::WasmBook`)](#10-webassembly-client-api-ebook_rswasmbook)

---

## 1. Book Core API (`ebook_rs::Book`)

`Book` is the entry point struct for opening, inspecting, searching, and extracting EPUB content.

### Loading EPUB Publications

```rust
use ebook_rs::Book;

// Load from local filesystem path
let mut book = Book::from_file("my_book.epub")?;

// Load from in-memory byte slice (e.g. HTTP download or WASM Uint8Array)
let bytes: Vec<u8> = std::fs::read("my_book.epub")?;
let mut book = Book::from_bytes(&bytes)?;
```

### Inspecting Metadata & Structure

```rust
// Extracted metadata fields (Title, Creators, Publishers, Identifier, PubDate, etc.)
let meta = book.metadata();
println!("Title: {}", meta.title);
println!("Creators: {:?}", meta.creators);
println!("Publisher: {:?}", meta.publishers);
println!("Identifier: {:?}", meta.identifier);

// Spine items in reading order
for spine_item in book.spine() {
    println!("Spine #{}: idref={} href={}", spine_item.index, spine_item.idref, spine_item.href);
}

// Table of Contents (Dual EPUB 2 NCX & EPUB 3 NAV support)
for nav_point in book.toc() {
    println!("- {} -> {}", nav_point.label, nav_point.href);
}

// EPUB 3 Landmarks (Cover, Titlepage, TOC, Bodymatter)
for landmark in book.landmarks() {
    println!("Landmark {}: {}", landmark.epub_type, landmark.href);
}

// EPUB 3 Page List (Print book page numbers)
for page in book.page_list() {
    println!("Page {}: {}", page.page, page.href);
}
```

### Extracting Cover Images

```rust
// Automatically resolves binary cover image bytes and MIME type (e.g. "image/jpeg", "image/png")
if let Some((bytes, mime)) = book.cover_image() {
    std::fs::write("extracted_cover.jpg", &bytes)?;
    println!("Extracted cover image with MIME: {}", mime);
}
```

### Retrieving Chapter Sections & HTML Resources

```rust
// Get processed section by spine index (with inlined Base64 Data URIs for images/fonts)
let sec0 = book.get_section(0)?;
println!("Section 0 plain text char count: {}", sec0.char_count);
println!("Processed HTML:\n{}", sec0.processed_html);

// Get section by relative href string
let sec_ch1 = book.get_section_by_href("OEBPS/ch1.xhtml")?;

// Get section by EPUB Canonical Fragment Identifier (CFI)
let sec_cfi = book.get_section_by_cfi("epubcfi(/6/4!/4/2)")?;
```

### Pre-Display HTML Transformation Hooks

```rust
// Register a closure hook that mutates section HTML before display (e.g. watermark/script injection)
book.register_before_display_hook(|html, path| {
    html.push_str(&format!("<div class='footer-note'>Read via EBook-RS ({})</div>", path));
});
```

---

## 2. Section Module (`ebook_rs::Section`)

`Section` represents a single chapter or document item from the EPUB spine.

```rust
pub struct Section {
    pub index: usize,            // Spine index (0-based)
    pub idref: String,           // OPF spine idref attribute
    pub href: String,            // Manifest href attribute
    pub full_path: String,       // Full normalized Zip internal path
    pub raw_html: String,        // Unmodified XHTML content
    pub processed_html: String,  // HTML with Base64 Data URI inlined images, CSS, and fonts
    pub plain_text: String,      // Clean extracted text (excluding tags, styles, scripts)
    pub char_count: usize,      // Total character count of plain text
}
```

---

## 3. EPUB CFI Engine (`ebook_rs::Cfi`)

Full implementation of the IDPF EPUB Canonical Fragment Identifier (CFI) Specification v1.0.

```rust
use ebook_rs::Cfi;

// 1. Parsing CFI strings
let cfi = Cfi::parse("epubcfi(/6/4[chap01ref]!/4/2/10/1:42)")?;
assert_eq!(cfi.spine_index(), 1);       // /6/4 -> spine index 1 ( (4/2)-1 )
assert_eq!(cfi.char_offset(), 42);

// 2. Generating CFI from spine index and text offset
let generated_cfi = Cfi::from_spine_index(0, None, 100);
println!("CFI String: {}", generated_cfi.to_string()); // Output: "epubcfi(/6/2!/4/2/1:100)"

// 3. Parsing Selection Range CFIs
let range_cfi = Cfi::parse("epubcfi(/6/2!/4/2/1,:10,:45)")?;
assert!(range_cfi.range_start.is_some());
assert!(range_cfi.range_end.is_some());
```

---

## 4. Locations Progress Engine (`ebook_rs::Locations`)

Generates discrete location chunks across the publication, enabling progress calculation (`CFI <-> Location <-> Percentage`).

```rust
let mut book = Book::from_file("my_book.epub")?;

// Total location count (default 1000 char chunk size)
println!("Total locations: {}", book.locations.total_locations);

// Map Location ID to CFI
if let Some(cfi) = book.locations.cfi_from_location(5) {
    println!("Location 5 CFI: {}", cfi);
}

// Map CFI to Location ID
if let Some(entry) = book.locations.location_from_cfi("epubcfi(/6/4!/4/2/1:100)") {
    println!("CFI maps to Location #{}, Spine Index #{}", entry.location, entry.spine_index);
}

// Map CFI to Percentage Progress (0.0 to 1.0)
let pct = book.locations.percentage_from_cfi("epubcfi(/6/4!/4/2/1:100)");
println!("Progress: {:.1}%", pct * 100.0);
```

---

## 5. Annotations Manager (`ebook_rs::AnnotationManager`)

Manages CFI-anchored highlights, bookmarks, underlines, and user notes.

```rust
use ebook_rs::AnnotationType;

// Create Highlight
let ann = book.annotations.create_highlight(
    "epubcfi(/6/4!/4/2/1:10,/4/2/1:50)",
    "#fde047", // Yellow hex color
    Some("Highlighted text snippet"),
    Some("User note on highlight"),
);

// Create Bookmark
let bm = book.annotations.create_bookmark(
    "epubcfi(/6/4!/4/2/1:0)",
    Some("Chapter 2 Bookmark"),
);

// List and remove annotations
for annotation in book.annotations.list() {
    println!("ID: {} Type: {:?}", annotation.id, annotation.type_);
}
book.annotations.remove(&ann.id);
```

---

## 6. Full-Text Search Engine (`ebook_rs::SearchEngine`)

Fast full-text search across all spine sections, returning surrounding context snippets and exact target CFIs.

```rust
let results = book.search("Rabbit");

for result in results {
    println!("Match at Spine #{}:", result.spine_index);
    println!("  CFI:     {}", result.cfi);
    println!("  Snippet: {}\n", result.snippet);
}
```

---

## 7. Layout & Themes (`ebook_rs::RenditionLayout`)

Rendition styling, spread layout modes, flow modes, and CSS overrides.

```rust
use ebook_rs::{RenditionLayout, Theme, SpreadMode, FlowMode};

let mut layout = RenditionLayout::default();

// Themes: Light, Dark, Sepia, Solarized, HighContrast
layout.theme = Theme::Sepia;
layout.font_size = 20;

// Spread Modes: Single, Double (Multi-Column Spreads), Auto
layout.spread_mode = SpreadMode::Double;

// Flow Modes: Paginated, Scrolled (Continuous Vertical Scroll)
layout.flow_mode = FlowMode::Scrolled;

// Generate CSS stylesheet override string
let css_override = layout.to_css_override();
```

---

## 8. Font De-Obfuscation (`ebook_rs::FontDeobfuscator`)

De-obfuscate embedded fonts encrypted with IDPF XOR or Adobe GUID algorithms.

```rust
use ebook_rs::FontDeobfuscator;

let xml_content = archive.read_string("META-INF/encryption.xml")?;
let deobfuscator = FontDeobfuscator::parse_encryption_xml(&xml_content);

if deobfuscator.is_encrypted("OEBPS/Fonts/CustomFont.otf") {
    let mut font_bytes = archive.read_bytes("OEBPS/Fonts/CustomFont.otf")?;
    let identifier = book.metadata().identifier.as_deref().unwrap_or("");
    
    // De-obfuscates in-place
    deobfuscator.deobfuscate("OEBPS/Fonts/CustomFont.otf", &mut font_bytes, identifier);
}
```

---

## 9. HTTP Reader Server (`ebook_rs::ReaderServer`)

Embedded HTTP server serving the interactive web application interface (Gated under `feature = ["server"]`).

```rust
#[cfg(feature = "server")]
use ebook_rs::{Book, ReaderServer};

fn main() -> Result<(), String> {
    let book = Book::from_file("my_book.epub")?;
    let server = ReaderServer::new(book, 8080);
    server.listen()?;
    Ok(())
}
```

---

## 10. WebAssembly Client API (`ebook_rs::WasmBook`)

WebAssembly JS client bindings for browser web applications (Gated under `feature = ["wasm"]`).

```rust
// Rust WASM exported wrapper class
#[cfg(feature = "wasm")]
use ebook_rs::WasmBook;
```

```javascript
// JavaScript Client Browser Code (npm / WebAssembly)
import { WasmBook } from 'ebook-rs';

const arrayBuffer = await fetch('my_book.epub').then(r => r.arrayBuffer());
const book = new WasmBook(new Uint8Array(arrayBuffer));

console.log("Metadata:", JSON.parse(book.get_metadata_json()));
console.log("TOC:", JSON.parse(book.get_toc_json()));

// Get processed HTML for Chapter 0
const chapterHtml = book.get_section_html(0);

// Search inside WASM
const searchResults = JSON.parse(book.search_json("Rabbit"));
```
