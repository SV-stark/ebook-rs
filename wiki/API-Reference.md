# 📚 `ebook-rs` Complete API Reference & Parity Guide

This document provides complete API documentation for `ebook-rs` (v0.2.0), bifurcated into **Drop-in Replacements for `epub.js`** and **`ebook-rs` Native Extensions**.

---

## 🟢 Section A: Drop-in Replacement for `epub.js` APIs

These `ebook-rs` APIs match the concepts, structures, and behavior of **`epub.js`** (e.g. `ePub(url)`, `book.loaded.*`, `rendition.display()`, `rendition.annotations`), enabling web developers and Rust engineers to migrate seamlessly.

| `epub.js` Feature / API | 🚀 `ebook-rs` Equivalent API | Description |
|---|---|---|
| `ePub(url)` / `ePub(buffer)` | `Book::from_file(path)` / `Book::from_bytes(bytes)` | Loads and parses package files into an in-memory `Book` struct. |
| `book.loaded.metadata` | `book.metadata()` | Returns `Metadata` containing `title`, `creators`, `publishers`, `languages`, `pub_date`, etc. |
| `book.loaded.spine` | `book.spine()` | Returns `Vec<SpineItem>` containing ordered chapter items (`idref`, `href`, `linear`). |
| `book.loaded.navigation.toc` | `book.toc()` | Returns `Vec<NavPoint>` hierarchical Table of Contents points. |
| `book.loaded.navigation.landmarks` | `book.landmarks()` | Returns `Vec<Landmark>` (cover, titlepage, bodymatter). |
| `book.loaded.navigation.pageList` | `book.page_list()` | Returns `Vec<PageListItem>` printed page numbers. |
| `book.locations.generate()` | `book.generate_locations(chunk_size)` | Generates discrete location progress markers across chapters. |
| `book.locations.percentageFromCfi()` | `book.locations.percentage_from_location(loc)` | Converts location integer indices to decimal progress percentage (`0.0 .. 1.0`). |
| `rendition.display(cfi)` | `book.get_section(spine_idx)` / `Cfi::parse()` | Retrieves raw and processed HTML content at specified CFI location. |
| `rendition.annotations.add()` | `book.annotations.add_annotation()` | Creates, updates, and serializes highlights, notes, and bookmarks. |

### Code Comparison: `epub.js` vs `ebook-rs`

#### JavaScript (`epub.js`)
```javascript
const book = ePub("alice.epub");
await book.opened;

console.log("Title:", book.packaging.metadata.title);
console.log("Spine:", book.spine.spineItems);

await book.locations.generate(1000);
const progress = book.locations.percentageFromCfi("epubcfi(/6/4!/4/2)");
```

#### Rust (`ebook-rs`)
```rust
use ebook_rs::Book;

let mut book = Book::from_file("alice.epub")?;

println!("Title: {}", book.metadata().title);
println!("Spine: {:?}", book.spine());

book.generate_locations(1000);
let progress = book.locations.percentage_from_location(5);
```

---

## 🔵 Section B: `ebook-rs` Native Extensions (Beyond `epub.js`)

Where `ebook-rs` extends far beyond `epub.js` to provide native multi-format support, zero-alloc searching, font de-obfuscation, and built-in HTTP reader server capabilities.

### 1. Multi-Format Native Parsers (`MobiBook`, `Fb2Book`, `LitBook`)
`epub.js` only supports `.epub` files. `ebook-rs` natively supports MOBI, AZW3, FB2, KEPUB, and LIT formats out of the box:

```rust
use ebook_rs::{MobiBook, Fb2Book, LitBook};

// Native MOBI / AZW3 PalmDOC LZ77 decompressor & parser
let mobi = MobiBook::parse(&mobi_bytes)?;

// Native FictionBook 2 XML parser with Base64 images
let fb2 = Fb2Book::parse(&fb2_bytes)?;

// Native Microsoft Reader LIT parser
let lit = LitBook::parse(&lit_bytes)?;
```

### 2. Zero-Allocation Full-Text Search Engine
`epub.js` lacks built-in full-text searching (requiring custom JS DOM loops). `ebook-rs` includes a pre-computed search engine that executes searches across millions of characters in < 1ms:

```rust
let matches = book.search("Wonderland");
for m in matches {
    println!("Spine #{}, char {}: {}", m.spine_index, m.char_offset, m.snippet);
}
```

### 3. Pre-Display Transformation Hooks
Custom transformation pipelines executed before rendering HTML sections:

```rust
book.add_before_display_hook(|html| {
    html.replace("old-class", "new-class")
});
```

### 4. Font De-Obfuscation Engine (`FontDeobfuscator`)
Native IDPF & Adobe font decryption supporting SHA-1 key generation and XOR transformations:

```rust
use ebook_rs::FontDeobfuscator;

let deobfuscator = FontDeobfuscator::parse_encryption_xml(&encryption_xml);
let decrypted_font_bytes = deobfuscator.deobfuscate("fonts/custom.otf", &raw_font_bytes, "urn:uuid:12345");
```

### 5. Multithreaded HTTP Reader Server (`ReaderServer`)
Built-in HTTP server serving interactive reader web UI and API endpoints:

```rust
use ebook_rs::server::ReaderServer;

let server = ReaderServer::new(book, 8080);
server.start()?; // Starts server on http://localhost:8080
```
