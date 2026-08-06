# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.7.0)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, and **PDF** formats, designed for full feature parity with **epub.js** and **foliate-js**.

---

## ⚡ Feature Parity Matrix

| Feature | 🚀 `ebook-rs` (v0.7.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **MOBI & AZW3 Support** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2) Support** | ✅ Native XML | ❌ No | ✅ Yes | ❌ No |
| **KEPUB & LIT Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **CBZ Comic Archive Support** | ✅ Native ZIP Images | ❌ No | ✅ Yes | ❌ No |
| **PDF Document Text Extraction** | ✅ `PdfBook` (pdf_oxide) | ❌ No | ✅ Yes | ❌ No |
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
| **Resource Streaming API** | ✅ `AssetDeliveryStrategy` | ✅ Blob URIs | ✅ Blob URIs | ❌ No |
| **Embedded Script Sanitizer** | ✅ `strip_script_content()` | ✅ Sandbox | ✅ Sandbox | ❌ No |
| **Fixed Layout (FXL) Scaling** | ✅ `compute_fxl_scale()` | ✅ Yes | ✅ Yes | ❌ No |
| **Full-Text Search Engine** | ✅ 0.59ms / 0-alloc | ❌ Third-party | ✅ Yes | ❌ No |
| **EPUB Canonical Fragment Identifiers (CFI)** | ✅ Complete IDPF Spec | ✅ Yes | ✅ Yes | ❌ No |
| **Locations & Progress Indexing** | ✅ Discrete Chunks | ⚠️ Slow (DOM) | ✅ Yes | ❌ No |
| **Font De-Obfuscation (IDPF & Adobe)** | ✅ Native SHA-1/XOR | ❌ No | ✅ Yes | ❌ No |
| **WebAssembly Browser Support** | ✅ `wasm-bindgen` | ❌ JS Only | ❌ JS Only | ❌ No |
| **Embedded HTTP Reader App** | ✅ Built-in Server | ❌ No | ❌ No | ❌ No |
| **EPUB 3 Accessibility Metadata (a11y)** | ✅ `AccessibilityMetadata` | ❌ No | ⚠️ Partial | ❌ No |
| **EPUB 3 Media Overlays (SMIL Sync)** | ✅ `MediaOverlayPackage` | ❌ No | ✅ Yes | ❌ No |
| **Readium Webpub Manifest Export** | ✅ `to_webpub_manifest` | ❌ No | ✅ Yes | ❌ No |
| **Readium LCP DRM Parsing** | ✅ `LcpLicense`, `LcpDecryptor` | ❌ No | ❌ No | ❌ No |
| **Readium Unified Locator Model** | ✅ `to_readium_locator()` | ❌ No | ✅ Yes | ❌ No |
| **Readium Search API (JSON)** | ✅ `to_readium_search_json()` | ❌ No | ❌ No | ❌ No |
| **Regex Full-Text Search** | ✅ `search_regex()` | ❌ No | ❌ No | ❌ No |
| **EPUB Structural Validator** | ✅ `EpubValidator` | ❌ No | ❌ No | ❌ No |
| **Content Fingerprinting & Deduplication** | ✅ `BookFingerprint` | ❌ No | ❌ No | ❌ No |
| **Academic Citation Exporter** | ✅ BibTeX, APA, MLA, Chicago | ❌ No | ❌ No | ❌ No |

---

## 🚀 Quick Start Example

```rust
use ebook_rs::{Book, ReflowPaginator, Cfi};

fn main() -> Result<(), String> {
    // Auto-detects and opens EPUB, MOBI, AZW3, FB2, KEPUB, LIT, or CBZ files
    let mut book = Book::from_file("book.epub")?;

    println!("Title: {}", book.metadata().title);
    println!("Sections: {}", book.spine().len());

    // Search with surrounding context snippets and <mark> highlights
    let search_results = book.search("quantum");
    for res in search_results {
        println!("Snippet: {}", res.snippet);
    }

    // Resolve CFI element step to CSS DOM selector
    let cfi = Cfi::parse("epubcfi(/6/4[chap01]!/4/2/1:10)")?;
    let section = book.get_section(0)?;
    if let Some(target) = cfi.resolve_dom_path(&section.raw_html) {
        println!("CSS Selector: {}", target.css_selector);
        println!("Target Element ID: {:?}", target.element_id);
    }

    Ok(())
}
```

---

## 🆕 What's New in v0.7.0

- **Regex Full-Text Search** — Search eBook text using full regular expression patterns (`book.search_regex("(?i)quantum|physics")`).
- **EPUB Structural Validator** — Validate eBook package integrity, OPF metadata, and spine references with `book.validate()` (`EpubValidator`).
- **Book Fingerprinting & Deduplication** — Generate metadata-independent content hashes to detect duplicate books across formats with `book.fingerprint()`.
- **Academic Citation Exporter** — Instantly export scholarly citations in **BibTeX**, **APA**, **MLA**, and **Chicago** formats (`book.to_bibtex()`, `book.to_apa()`, `book.to_mla()`, `book.to_chicago()`).

```rust
use ebook_rs::Book;

let book = Book::from_file("book.epub")?;

// 1. Regex Search
let matches = book.search_regex("(?i)alice|rabbit")?;

// 2. Structural Validation
let report = book.validate();
if !report.is_valid {
    println!("Validation errors: {:?}", report.errors);
}

// 3. Content Fingerprinting & Match Score
let fp = book.fingerprint();
println!("Content Hash: {}", fp.content_hash);

// 4. Academic Citations
println!("BibTeX:\n{}", book.to_bibtex());
println!("APA: {}", book.to_apa());
```

---

## 📖 Documentation & Wiki

Detailed API documentation and integration guides are available in [API.md](API.md) and the [GitHub Wiki](https://github.com/SV-stark/ebook-rs/wiki).

---

## 📜 License

Licensed under the MIT License. Copyright (c) 2026 SV-Stark.
