# 📚 EBook-RS Complete API Reference Guide (v0.10.5)

Welcome to the comprehensive, exhaustive API Reference Guide for **`ebook-rs`** (v0.10.5).

---

## 📑 Table of Contents

1. [Core Reader Engine & Memory Mapping (`Book`)](#1-core-reader-engine--memory-mapping-book)
2. [Performance Accelerators (`compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot`)](#2-performance-accelerators-compact_str-ahash-simdutf8-zlib-rs-memchr-parking_lot)
3. [Universal EPUB 3 Exporter (`export_epub3_bytes`)](#3-universal-epub-3-exporter-export_epub3_bytes)
4. [Lightweight DOM AST Engine (`EbookDomTree`, `DomNode`)](#4-lightweight-dom-ast-engine-ebookdomtree-domnode)
5. [Fuzzy Malformed XML Recovery Engine (`sanitize_and_repair_xml`)](#5-fuzzy-malformed-xml-recovery-engine-sanitize_and_repair_xml)
6. [Search Engine & Context Snippets (`SearchEngine`, `SearchResult`)](#6-search-engine--context-snippets-searchengine-searchresult)
7. [CFI Engine & Deep DOM Resolver (`Cfi`, `CfiDomTarget`)](#7-cfi-engine--deep-dom-resolver-cfi-cfidomtarget)
8. [Annotations & W3C Web Annotation Data Model (`AnnotationManager`)](#8-annotations--w3c-web-annotation-data-model-annotationmanager)
9. [WebAssembly Bindings (`WasmBook`)](#9-webassembly-bindings-wasmbook)

---

## 1. Core Reader Engine & Memory Mapping (`Book`)

```rust
use ebook_rs::Book;

// Auto-detects and loads EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ, PDF, ODT, TXT, MD
let book = Book::from_file("book.epub")?;

// Zero-copy mmap file loading (requires `mmap` feature)
#[cfg(feature = "mmap")]
let mmap_book = Book::from_mmap("huge_comic.cbz")?;
```

---

## 2. Performance Accelerators (`compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot`)

`ebook-rs` v0.10.5 integrates 6 SIMD and stack-optimization performance crates:
- **`compact_str`**: Small String Optimization (`CompactString`) storing strings <= 24 bytes directly on the stack to eliminate heap allocations.
- **`ahash`**: `AHashMap` & `AHashSet` for 3x-5x faster hash lookups in OPF manifest, DOM attributes, and annotations.
- **`simdutf8`**: SIMD-accelerated UTF-8 validation (`simdutf8::basic::from_utf8`) for 10x-20x faster HTML/XML byte-stream decoding.
- **`zlib-rs`**: `flate2` with `zlib-rs` SIMD zlib decompression for 3x faster EPUB/CBZ ZIP archive reading.
- **`memchr`**: SIMD substring searching (`memchr::memchr` / `memchr::memmem`) for ultra-fast HTML tag scanning, attribute extraction, and script stripping.
- **`parking_lot`**: Fast 1-byte non-poisoning mutex locks (`parking_lot::Mutex`) for concurrent section caches.

---

## 3. Universal EPUB 3 Exporter (`export_epub3_bytes`)

Compile any opened book (MOBI, AZW3, FB2, CBZ, ODT, TXT, MD) directly into a binary EPUB 3 (`.epub`) ZIP archive:

```rust
let epub3_bytes = book.export_epub3_bytes()?;
std::fs::write("converted_book.epub", &epub3_bytes)?;
```

---

## 4. Lightweight DOM AST Engine (`EbookDomTree`, `DomNode`)

```rust
use ebook_rs::EbookDomTree;

let html = "<div><h1>Title</h1><script>alert(1)</script><p>Text</p></div>";
let mut tree = EbookDomTree::parse(html);

// Find matching elements
let h1_nodes = tree.find_elements_by_tag("h1");

// Strip forbidden elements
tree.strip_elements(&["script"]);
let clean_html = tree.to_html();
```

---

## 5. Fuzzy Malformed XML Recovery Engine (`sanitize_and_repair_xml`)

```rust
use ebook_rs::sanitize_and_repair_xml;

let broken_xml = "<package><title>AT&T & R&D Guide</title></package>";
let repaired = sanitize_and_repair_xml(broken_xml);
assert!(repaired.contains("&amp;"));
```

---

## 6. Search Engine & Context Snippets (`SearchEngine`, `SearchResult`)

```rust
let results = book.search("quantum");
for match_item in results {
    println!("CFI: {}", match_item.cfi);
    println!("Snippet: {}", match_item.snippet); // "...<mark>quantum</mark> mechanics..."
}
```

---

## 7. CFI Engine & Deep DOM Resolver (`Cfi`, `CfiDomTarget`)

```rust
use ebook_rs::Cfi;

let cfi = Cfi::parse("epubcfi(/6/4[chap01]!/4/2/1:10)")?;
let section = book.get_section(0)?;

if let Some(dom_target) = cfi.resolve_dom_path(&section.raw_html) {
    println!("CSS Selector: {}", dom_target.css_selector); // "body > div:nth-child(2) > p:nth-child(1)"
}
```

---

## 8. Annotations & W3C Web Annotation Data Model (`AnnotationManager`)

```rust
let mut manager = ebook_rs::AnnotationManager::new();
manager.create_highlight("epubcfi(/6/4!/4/2/1:0)", "#ffff00", Some("quote text"), Some("my note"));

let json_ld = manager.to_w3c_json()?;
```

---

## 9. WebAssembly Bindings (`WasmBook`)

```javascript
import { WasmBook } from "ebook-rs";

const book = new WasmBook(uint8Array);
const domTarget = JSON.parse(book.resolve_cfi_dom_json("epubcfi(/6/4[chap01]!/4/2/1:10)", 0));
```
