# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.4.0)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, and **CBZ (Comic Book ZIP)** formats, designed for full feature parity with **epub.js** and **foliate-js**.

---

## ⚡ Feature Parity Matrix

| Feature | 🚀 `ebook-rs` (v0.4.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **MOBI & AZW3 Support** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2) Support** | ✅ Native XML | ❌ No | ✅ Yes | ❌ No |
| **KEPUB & LIT Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **CBZ Comic Archive Support** | ✅ Native ZIP Images | ❌ No | ✅ Yes | ❌ No |
| **Footnote & Endnote Previewer** | ✅ `extract_footnotes()` | ❌ No | ✅ Yes | ❌ No |
| **OPDS Catalog Feed Client** | ✅ OPDS 1.2 & 2.0 | ❌ No | ✅ Yes | ❌ No |
| **Resource Streaming API** | ✅ `AssetDeliveryStrategy` | ✅ Blob URIs | ✅ Blob URIs | ❌ No |
| **Embedded Script Sanitizer** | ✅ `strip_script_content()` | ✅ Sandbox | ✅ Sandbox | ❌ No |
| **Fixed Layout (FXL) Scaling** | ✅ `compute_fxl_scale()` | ✅ Yes | ✅ Yes | ❌ No |
| **Full-Text Search Engine** | ✅ 0.59ms / 0-alloc | ❌ Third-party | ✅ Yes | ❌ No |
| **EPUB Canonical Fragment Identifiers (CFI)** | ✅ Complete IDPF Spec | ✅ Yes | ✅ Yes | ❌ No |
| **Locations & Progress Indexing** | ✅ Discrete Chunks | ⚠️ Slow (DOM) | ✅ Yes | ❌ No |
| **Font De-Obfuscation (IDPF & Adobe)** | ✅ Native SHA-1/XOR | ❌ No | ✅ Yes | ❌ No |
| **Pre-Display Transformation Hooks** | ✅ Pipeline Hooks | ❌ No | ✅ Yes | ❌ No |
| **WebAssembly Browser Support** | ✅ `wasm-bindgen` | ❌ JS Only | ❌ JS Only | ❌ No |
| **Embedded HTTP Reader App** | ✅ Built-in Server | ❌ No | ❌ No | ❌ No |
| **Readium Webpub Manifest Export** | ✅ `to_webpub_manifest` | ❌ No | ✅ Yes | ❌ No |

---

## 🚀 Quick Start Example

```rust
use ebook_rs::{Book, AssetDeliveryStrategy};

fn main() -> Result<(), String> {
    // Auto-detects and opens EPUB, MOBI, AZW3, FB2, KEPUB, LIT, or CBZ files
    let mut book = Book::from_file("book.epub")?;

    println!("Title: {}", book.metadata().title);
    println!("Sections: {}", book.spine().len());

    // Extract footnotes for popup previewing
    let section = book.get_section(0)?;
    let footnotes = section.extract_footnotes();
    for fn_item in footnotes {
        println!("Footnote [{}]: {}", fn_item.label, fn_item.plain_text);
    }

    Ok(())
}
```

---

## 🗺️ Project Roadmap

- [x] **v0.1.0**: Core EPUB 2/3 parser, CFI Engine, Location Progress, Annotations, Web UI Server, WASM Bindings.
- [x] **v0.2.0**: Native Multi-Format Support (**MOBI**, **AZW3**, **FB2**, **KEPUB**, **LIT**), IDPF & Adobe Font De-obfuscation, EPUB 3 Landmarks & Page List, Pre-Display Transformation Hooks.
- [x] **v0.2.5**: Readium Webpub Manifest Export (`application/webpub+json`), PalmDOC LZ77 distance fallback.
- [x] **v0.3.0**: **CBZ (Comic Book Archive)** zero-copy base64 image renderer, format auto-detection guard.
- [x] **v0.4.0**: **Footnote Popup Previewer**, **OPDS 1.2/2.0 Catalog Feed Client**, **Asset Delivery Resource Streaming API**, **Embedded Script Sanitizer**, **Fixed Layout (FXL) Viewport Scaling Matrix**, **HTTP Range Request Header Resolver**.

---

## 📖 Documentation & Wiki

Detailed API documentation and integration guides are available in [docs/API.md](docs/API.md).

---

## 📜 License

Licensed under the MIT License. Copyright (c) 2026 SV-Stark.
