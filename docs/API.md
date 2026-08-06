# 📚 EBook-RS Complete API Reference Guide (v0.5.2)

Welcome to the comprehensive, exhaustive API Reference Guide for **`ebook-rs`** (v0.5.2).

---

## 📑 Table of Contents

1. [Core Reader Engine (`Book`)](#1-core-reader-engine-book)
2. [Search Engine & Context Snippets (`SearchEngine`, `SearchResult`)](#2-search-engine--context-snippets-searchengine-searchresult)
3. [CFI Engine & Deep DOM Resolver (`Cfi`, `CfiDomTarget`)](#3-cfi-engine--deep-dom-resolver-cfi-cfidomtarget)
4. [Annotations & W3C Web Annotation Data Model (`AnnotationManager`)](#4-annotations--w3c-web-annotation-data-model-annotationmanager)
5. [Rendition Layout & Reader Font Injection (`RenditionLayout`)](#5-rendition-layout--reader-font-injection-renditionlayout)
6. [Section & NLP Reading Analytics (`Section`, `ReadingAnalytics`)](#6-section--nlp-reading-analytics-section-readinganalytics)
7. [Deterministic Reflow Paginator (`ReflowPaginator`, `SectionPageMap`)](#7-deterministic-reflow-paginator-reflowpaginator-sectionpagemap)
8. [Remote ZIP Central Directory Streamer (`ZipHeaderReader`, `HttpRangeRequest`)](#8-remote-zip-central-directory-streamer-zipheaderreader-httprangerequest)
9. [Footnotes & OPDS Catalog Feed Client (`Footnote`, `OpdsFeed`)](#9-footnotes--opds-catalog-feed-client-footnote-opdsfeed)
10. [WebAssembly Bindings (`WasmBook`)](#10-webassembly-bindings-wasmbook)

---

## 1. Core Reader Engine (`Book`)

```rust
use ebook_rs::Book;

// Auto-detects and loads EPUB, MOBI, AZW3, FB2, KEPUB, LIT, or CBZ
let book = Book::from_file("book.epub")?;
```

---

## 2. Search Engine & Context Snippets (`SearchEngine`, `SearchResult`)

Full-text search returning 40-character prefix/suffix context snippets with `<mark>` query highlights.

```rust
let results = book.search("quantum");
for match_item in results {
    println!("Spine Index: {}", match_item.spine_index);
    println!("CFI: {}", match_item.cfi);
    println!("Snippet: {}", match_item.snippet); // "...<mark>quantum</mark> mechanics..."
}
```

---

## 3. CFI Engine & Deep DOM Resolver (`Cfi`, `CfiDomTarget`)

Resolves IDPF element steps (`/4/2/1`) and IDs (`[chap01]`) to target element IDs and CSS selector paths.

```rust
use ebook_rs::Cfi;

let cfi = Cfi::parse("epubcfi(/6/4[chap01]!/4/2/1:10)")?;
let section = book.get_section(0)?;

if let Some(dom_target) = cfi.resolve_dom_path(&section.raw_html) {
    println!("Target Element ID: {:?}", dom_target.element_id);
    println!("CSS Selector: {}", dom_target.css_selector); // "body > div:nth-child(2) > p:nth-child(1)"
    println!("Character Offset: {}", dom_target.char_offset);
}
```

---

## 4. Annotations & W3C Web Annotation Data Model (`AnnotationManager`)

Export annotations and highlights in standard W3C JSON-LD format (`http://www.w3.org/ns/anno.jsonld`).

```rust
use ebook_rs::AnnotationManager;

let mut manager = AnnotationManager::new();
manager.create_highlight("epubcfi(/6/4!/4/2/1:0)", "#ffff00", Some("quote text"), Some("my note"));

// Export to standard W3C JSON-LD
let json_ld = manager.to_w3c_json()?;
```

---

## 5. Rendition Layout & Reader Font Injection (`RenditionLayout`)

Inject custom `@font-face` definitions and font-family rules into `RenditionLayout`.

```rust
let mut layout = book.layout;
layout.set_custom_font("Roboto", "https://fonts.googleapis.com/css2?family=Roboto");

let css_override = layout.to_css_override();
```

---

## 6. Section & NLP Reading Analytics (`Section`, `ReadingAnalytics`)

```rust
let section = book.get_section(0)?;

// Automatic RTL dir="rtl" injection applies if book.metadata().direction == PageProgressionDirection::Rtl
println!("HTML: {}", section.processed_html);

let analytics = section.analytics();
println!("Word Count: {}", analytics.word_count);
println!("Top Keywords: {:?}", analytics.top_keywords);
```

---

## 7. Deterministic Reflow Paginator (`ReflowPaginator`, `SectionPageMap`)

```rust
use ebook_rs::ReflowPaginator;

let paginator = ReflowPaginator::new(16, 1.6, 800, 600, 32);
let page_map = section.paginate(Some(&paginator));
println!("Total Virtual Pages: {}", page_map.total_pages);
```

---

## 8. Remote ZIP Central Directory Streamer (`ZipHeaderReader`, `HttpRangeRequest`)

```rust
use ebook_rs::ZipHeaderReader;

let entries = ZipHeaderReader::parse_central_directory(&tail_bytes)?;
for entry in entries {
    let (header, val) = entry.to_http_range_header();
    println!("File: {}, Header: {}: {}", entry.file_name, header, val);
}
```

---

## 9. Footnotes & OPDS Catalog Feed Client (`Footnote`, `OpdsFeed`)

```rust
let footnotes = section.extract_footnotes();

#[cfg(feature = "opds")]
let feed = ebook_rs::OpdsFeed::parse_atom_xml(&xml_str)?;
```

---

## 10. WebAssembly Bindings (`WasmBook`)

```javascript
import { WasmBook } from "ebook-rs";

const book = new WasmBook(uint8Array);

// Resolve CFI to DOM selector JSON
const domTarget = JSON.parse(book.resolve_cfi_dom_json("epubcfi(/6/4[chap01]!/4/2/1:10)", 0));

// Inject custom font
book.set_custom_font("Roboto", "https://fonts.com/roboto.woff2");
```
