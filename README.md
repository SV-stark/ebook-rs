# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.5.0)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, and **CBZ (Comic Book ZIP)** formats, designed for full feature parity with **epub.js** and **foliate-js**.

---

## ⚡ Feature Parity Matrix

| Feature | 🚀 `ebook-rs` (v0.5.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **MOBI & AZW3 Support** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2) Support** | ✅ Native XML | ❌ No | ✅ Yes | ❌ No |
| **KEPUB & LIT Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **CBZ Comic Archive Support** | ✅ Native ZIP Images | ❌ No | ✅ Yes | ❌ No |
| **Rayon Multi-Core Parallel Parsing** | ✅ `parallel` feature | ❌ Single Thread | ❌ Single Thread | ❌ Single Thread |
| **DOM-Free Reflow Paginator** | ✅ `ReflowPaginator` | ❌ DOM-based | ❌ DOM-based | ❌ None |
| **NLP Reading Analytics & Keywords** | ✅ `ReadingAnalytics` | ❌ No | ❌ No | ❌ No |
| **Remote ZIP Central Directory Streamer** | ✅ `ZipHeaderReader` | ❌ Unpacked only | ❌ Download full | ❌ No |
| **Footnote & Endnote Previewer** | ✅ `extract_footnotes()` | ❌ No | ✅ Yes | ❌ No |
| **OPDS Catalog Feed Client** | ✅ OPDS 1.2 & 2.0 | ❌ No | ✅ Yes | ❌ No |
| **Resource Streaming API** | ✅ `AssetDeliveryStrategy` | ✅ Blob URIs | ✅ Blob URIs | ❌ No |
| **Embedded Script Sanitizer** | ✅ `strip_script_content()` | ✅ Sandbox | ✅ Sandbox | ❌ No |
| **Fixed Layout (FXL) Scaling** | ✅ `compute_fxl_scale()` | ✅ Yes | ✅ Yes | ❌ No |
| **Full-Text Search Engine** | ✅ 0.59ms / 0-alloc | ❌ Third-party | ✅ Yes | ❌ No |
| **EPUB Canonical Fragment Identifiers (CFI)** | ✅ Complete IDPF Spec | ✅ Yes | ✅ Yes | ❌ No |
| **Locations & Progress Indexing** | ✅ Discrete Chunks | ⚠️ Slow (DOM) | ✅ Yes | ❌ No |
| **Font De-Obfuscation (IDPF & Adobe)** | ✅ Native SHA-1/XOR | ❌ No | ✅ Yes | ❌ No |
| **WebAssembly Browser Support** | ✅ `wasm-bindgen` | ❌ JS Only | ❌ JS Only | ❌ No |
| **Embedded HTTP Reader App** | ✅ Built-in Server | ❌ No | ❌ No | ❌ No |
| **Readium Webpub Manifest Export** | ✅ `to_webpub_manifest` | ❌ No | ✅ Yes | ❌ No |

---

## 🚀 Quick Start Example

```rust
use ebook_rs::{Book, ReflowPaginator, AssetDeliveryStrategy};

fn main() -> Result<(), String> {
    // Auto-detects and opens EPUB, MOBI, AZW3, FB2, KEPUB, LIT, or CBZ files
    let mut book = Book::from_file("book.epub")?;

    println!("Title: {}", book.metadata().title);
    println!("Sections: {}", book.spine().len());

    // Extract NLP reading analytics and keywords
    let section = book.get_section(0)?;
    let analytics = section.analytics();
    println!("Word Count: {}", analytics.word_count);
    println!("Reading Time: {} mins", analytics.reading_time_minutes);
    println!("Top Keywords: {:?}", analytics.top_keywords);

    // Calculate DOM-free virtual page breaks
    let paginator = ReflowPaginator::new(16, 1.6, 800, 600, 32);
    let page_map = section.paginate(Some(&paginator));
    println!("Total Virtual Pages: {}", page_map.total_pages);

    Ok(())
}
```

---

## 📖 Documentation & Wiki

Detailed API documentation and integration guides are available in [docs/API.md](docs/API.md) and [WIKI.md](WIKI.md).

---

## 📜 License

Licensed under the MIT License. Copyright (c) 2026 SV-Stark.
