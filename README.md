# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.11.2)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT**, **TXT**, and **MD** formats, designed for full feature parity with **epub.js** and **foliate-js**.

---

## ⚡ Feature Parity Matrix

| Feature | 🚀 `ebook-rs` (v0.11.2) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **MOBI & AZW3 Support** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2) Support** | ✅ Native XML | ❌ No | ✅ Yes | ❌ No |
| **KEPUB & LIT Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **CBZ Comic Archive Support** | ✅ Native ZIP Images | ❌ No | ✅ Yes | ❌ No |
| **SpeechSynthesis TTS Word Synchronizer** | ✅ `get_tts_tokens` | ❌ No | ✅ Yes | ❌ No |
| **Legacy Charset Decoding** | ✅ `decode_bytes_with_encoding` | ❌ No | ❌ No | ❌ No |
| **Auto Language Detection** | ✅ `detect_language` | ❌ No | ❌ No | ❌ No |
| **Zstd Compressed State Caching** | ✅ `export_zstd_cache` | ❌ No | ❌ No | ❌ No |
| **Universal EPUB 3 Exporter** | ✅ `export_epub3_bytes` | ❌ No | ❌ No | ❌ No |
| **Zstd Compressed State Caching** | ✅ `export_zstd_cache` | ❌ No | ❌ No | ❌ No |
| **Universal EPUB 3 Exporter** | ✅ `export_epub3_bytes` | ❌ No | ❌ No | ❌ No |
| **Zero-Copy Memory-Mapped I/O** | ✅ `Book::from_mmap` | ❌ No | ❌ No | ❌ No |
| **Fuzzy XML Recovery Parser** | ✅ `sanitize_and_repair_xml` | ❌ No | ❌ No | ❌ No |

---

## 🆕 What's New in v0.11.0

- **Legacy Non-UTF-8 Charset Decoding (`encoding_rs`)** — Decodes legacy charsets (`Windows-1252`, `Shift-JIS`, `GBK`, `ISO-8859-1`) into clean UTF-8 strings for 100% parse success across legacy MOBI, FB2, and TXT files.
- **Automatic Language Detection (`whatlang`)** — Fast statistical language identification on text contents (`book.detect_language()`) when OPF metadata `dc:language` is missing.
- **Zstd Compressed State Caching (`zstd`)** — Sub-millisecond instant serialization and restoration of parsed book states (`book.export_zstd_cache()`, `Book::from_zstd_cache()`) for high-throughput servers and WASM runtimes.

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
