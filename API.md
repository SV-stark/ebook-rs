# 📚 EBook-RS API Reference & Complete Documentation (v0.2.0)

`ebook-rs` (v0.2.0) is a multi-format pure Rust eBook engine supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, and **LIT (Microsoft Reader)** formats.

---

## 📋 Table of Contents
1. [Multi-Format Book Core API (`ebook_rs::Book`)](#1-multi-format-book-core-api-ebook_rsbook)
2. [Supported Formats Matrix](#2-supported-formats-matrix)
3. [Format-Specific Parsers (`MobiBook`, `Fb2Book`, `LitBook`)](#3-format-specific-parsers)
4. [Section Module (`ebook_rs::Section`)](#4-section-module-ebook_rssection)
5. [EPUB CFI Engine (`ebook_rs::Cfi`)](#5-epub-cfi-engine-ebook_rscfi)
6. [Locations Progress Engine (`ebook_rs::Locations`)](#6-locations-progress-engine-ebook_rslocations)
7. [Annotations Manager (`ebook_rs::AnnotationManager`)](#7-annotations-manager-ebook_rsannotationmanager)
8. [Full-Text Search Engine (`ebook_rs::SearchEngine`)](#8-full-text-search-engine-ebook_rssearchengine)
9. [Layout & Themes (`ebook_rs::RenditionLayout`)](#9-layout--themes-ebook_rsrenditionlayout)
10. [Font De-Obfuscation (`ebook_rs::FontDeobfuscator`)](#10-font-de-obfuscation-ebook_rsfontdeobfuscator)
11. [HTTP Reader Server (`ebook_rs::ReaderServer`)](#11-http-reader-server-ebook_rsreaderserver)
12. [WebAssembly Client API (`ebook_rs::WasmBook`)](#12-webassembly-client-api-ebook_rswasmbook)

---

## 1. Multi-Format Book Core API (`ebook_rs::Book`)

`Book` is the unified entry point struct for opening, inspecting, searching, and rendering eBooks across all formats.

### Opening eBooks (EPUB, MOBI, AZW3, FB2, KEPUB, LIT)

```rust
use ebook_rs::Book;

// Auto-detects and loads EPUB, MOBI, AZW3, FB2, KEPUB, or LIT from file path
let mut book = Book::from_file("my_book.azw3")?;

// Load from in-memory byte slice (e.g. HTTP download or WASM Uint8Array)
let bytes: Vec<u8> = std::fs::read("my_book.fb2")?;
let mut book = Book::from_bytes(&bytes)?;
```

---

## 2. Supported Formats Matrix

| Extension | Format Name | Auto-Detected Header Magic | Inlining & Media |
|---|---|---|---|
| `.epub` | EPUB 2 & EPUB 3 | `PK\x03\x04` | Full Base64 Data URIs |
| `.kepub` | Kobo EPUB | `PK\x03\x04` | KoboSpan DOM Aware |
| `.mobi` | Mobipocket / PalmDOC | `BOOKMOBI` / `TEXtREDR` | PalmDOC LZ77 Decompressor |
| `.azw3` | Kindle Format 8 (KF8) | `BOOKMOBI` PDB Header | HTML5 / CSS3 Extracted |
| `.fb2` | FictionBook 2 XML | `<FictionBook>` XML | Embedded Base64 Images |
| `.lit` | Microsoft Reader LIT | `ITOL` / `ITLS` | HTML Stream Extracted |

---

## 3. Format-Specific Parsers

### MOBI & AZW3 (`ebook_rs::MobiBook`)

```rust
use ebook_rs::MobiBook;

let bytes = std::fs::read("book.mobi")?;
let mobi_book = MobiBook::parse(&bytes)?;
println!("Title: {}", mobi_book.metadata().title);
```

### FictionBook 2 (`ebook_rs::Fb2Book`)

```rust
use ebook_rs::Fb2Book;

let bytes = std::fs::read("book.fb2")?;
let fb2_book = Fb2Book::parse(&bytes)?;
println!("Author: {:?}", fb2_book.metadata().creators);
```

### Microsoft Reader LIT (`ebook_rs::LitBook`)

```rust
use ebook_rs::LitBook;

let bytes = std::fs::read("book.lit")?;
let lit_book = LitBook::parse(&bytes)?;
```

---

## 4. Full-Text Search Engine Across Formats

Fast full-text search across all chapter sections regardless of original file format:

```rust
let matches = book.search("Rabbit");
for m in matches {
    println!("Match at Spine #{}: {}", m.spine_index, m.snippet);
}
```
