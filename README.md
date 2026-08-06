# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.10.5)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT**, **TXT**, and **MD** formats, designed for full feature parity with **epub.js** and **foliate-js**.

---

## ⚡ Feature Parity Matrix

| Feature | 🚀 `ebook-rs` (v0.10.5) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **MOBI & AZW3 Support** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2) Support** | ✅ Native XML | ❌ No | ✅ Yes | ❌ No |
| **KEPUB, LIT, PDF, ODT, TXT, MD Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **CBZ Comic Archive Support** | ✅ Native ZIP Images | ❌ No | ✅ Yes | ❌ No |
| **Performance Accelerators** | ✅ `compact_str`, `ahash`, `simdutf8`, `zlib-rs`, `memchr`, `parking_lot` | ❌ No | ❌ No | ❌ No |
| **Universal EPUB 3 Exporter** | ✅ `export_epub3_bytes()` | ❌ No | ❌ No | ❌ No |
| **Zero-Copy Memory-Mapped I/O** | ✅ `Book::from_mmap()` | ❌ No | ❌ No | ❌ No |
| **Lightweight DOM AST Tree** | ✅ `EbookDomTree` | ❌ DOM-based | ❌ DOM-based | ❌ None |
| **Fuzzy Malformed XML Recovery** | ✅ `sanitize_and_repair_xml` | ❌ Strict | ❌ Strict | ❌ Strict |
| **Rayon Multi-Core Parallel Parsing** | ✅ `parallel` feature | ❌ Single Thread | ❌ Single Thread | ❌ Single Thread |
| **DOM-Free Reflow Paginator** | ✅ `ReflowPaginator` | ❌ DOM-based | ❌ DOM-based | ❌ None |
| **NLP Reading Analytics & Keywords** | ✅ `ReadingAnalytics` | ❌ No | ❌ No | ❌ No |
| **Search Context Snippets & Highlights** | ✅ `<mark>` Highlights | ⚠️ Basic | ✅ Yes | ❌ No |
| **Deep DOM-Element CFI Resolver** | ✅ `resolve_dom_path()` | ✅ Yes | ✅ Yes | ❌ No |
| **W3C Web Annotation Data Model (JSON-LD)** | ✅ `to_w3c_json()` | ❌ No | ❌ No | ❌ No |
| **Automatic RTL (`dir="rtl"`) Injection** | ✅ Render-time | ✅ Yes | ✅ Yes | ❌ No |
| **Reader Custom Font Injection** | ✅ `@font-face` CSS | ✅ Yes | ✅ Yes | ❌ No |
| **Remote ZIP Central Directory Streamer** | ✅ `ZipHeaderReader` | ❌ Unpacked only | ❌ Download full | ❌ No |
| **Footnote & Endnote Previewer** | ✅ `extract_footnotes()` | ❌ No | ✅ Yes | ❌ No |
| **OPDS Catalog Feed Client** | ✅ OPDS 1.2 & 2.0 | ❌ No | ✅ Yes | ❌ No |

---

## 🚀 Quick Start Example

```rust
use ebook_rs::{Book, ReflowPaginator, Cfi};

fn main() -> Result<(), String> {
    // Auto-detects and opens EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ, PDF, ODT, TXT, MD
    let mut book = Book::from_file("book.epub")?;

    println!("Title: {}", book.metadata().title);
    println!("Sections: {}", book.spine().len());

    // Export any format to standard binary EPUB3 ZIP bytes
    let epub3_bytes = book.export_epub3_bytes()?;

    // Search with surrounding context snippets and <mark> highlights
    let search_results = book.search("quantum");
    for res in search_results {
        println!("Snippet: {}", res.snippet);
    }

    Ok(())
}
```

---

## 📖 Documentation & Wiki

Detailed API documentation and integration guides are available in [docs/API.md](docs/API.md) and [WIKI.md](WIKI.md).

---

## 📜 License

Licensed under the MIT License. Copyright (c) 2026 SV-Stark.
