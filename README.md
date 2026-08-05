# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.3.0)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, and **CBZ (Comic Book ZIP)** formats, designed for full feature parity with **epub.js** and **foliate-js**.

---

## ⚡ Feature Parity Matrix

| Feature | 🚀 `ebook-rs` (v0.3.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **MOBI & AZW3 Support** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2) Support** | ✅ Native XML | ❌ No | ✅ Yes | ❌ No |
| **KEPUB & LIT Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **CBZ Comic Archive Support** | ✅ Native ZIP Images | ❌ No | ✅ Yes | ❌ No |
| **CBR Comic Archive Detection** | ℹ️ Pure-Rust Error Guidance | ❌ No | ✅ Yes (libarchive) | ❌ No |
| **Full-Text Search Engine** | ✅ 0.59ms / 0-alloc | ❌ Third-party | ✅ Yes | ❌ No |
| **EPUB Canonical Fragment Identifiers (CFI)** | ✅ Complete IDPF Spec | ✅ Yes | ✅ Yes | ❌ No |
| **Locations & Progress Indexing** | ✅ Discrete Chunks | ⚠️ Slow (DOM) | ✅ Yes | ❌ No |
| **Font De-Obfuscation (IDPF & Adobe)** | ✅ Native SHA-1/XOR | ❌ No | ✅ Yes | ❌ No |
| **Pre-Display Transformation Hooks** | ✅ Pipeline Hooks | ❌ No | ✅ Yes | ❌ No |
| **EPUB 3 Landmarks & Page-List** | ✅ Full Parser | ✅ Yes | ✅ Yes | ❌ No |
| **Base64 Asset Inlining** | ✅ Auto Images/Fonts/CSS | ❌ Blob URIs | ❌ Blob URIs | ❌ No |
| **WebAssembly Browser Support** | ✅ `wasm-bindgen` | ❌ JS Only | ❌ JS Only | ❌ No |
| **Embedded HTTP Reader App** | ✅ Built-in Server | ❌ No | ❌ No | ❌ No |
| **Double-Spread & Continuous Scroll** | ✅ CSS Column / Vertical | ✅ Yes | ✅ Yes | ❌ No |
| **Readium Webpub Manifest Export** | ✅ `to_webpub_manifest` | ❌ No | ✅ Yes | ❌ No |

---

## 🗺️ Project Roadmap

- [x] **v0.1.0**: Core EPUB 2/3 parser, CFI Engine, Location Progress, Annotations, Web UI Server, WASM Bindings.
- [x] **v0.2.0**: Native Multi-Format Support (**MOBI**, **AZW3**, **FB2**, **KEPUB**, **LIT**), IDPF & Adobe Font De-obfuscation, EPUB 3 Landmarks & Page List, Pre-Display Transformation Hooks.
- [x] **v0.2.5**: Readium Webpub Manifest Export (`application/webpub+json`), PalmDOC LZ77 distance fallback, FB2 image reference matching fix, complex CFI range assertion safety.
- [x] **v0.3.0**: **CBZ (Comic Book Archive)** zero-copy base64 image renderer, format auto-detection guard, pure Rust CBR error guidance.

---

## 🚀 Quick Start Example

```rust
use ebook_rs::Book;

fn main() -> Result<(), String> {
    // Auto-detects and opens EPUB, MOBI, AZW3, FB2, KEPUB, LIT, or CBZ files
    let mut book = Book::from_file("comic.cbz")?;

    println!("Title: {}", book.metadata().title);
    println!("Sections/Pages: {}", book.spine().len());

    // Retrieve processed HTML page with embedded base64 images
    let page1 = book.get_section(0)?;
    println!("HTML: {}", page1.processed_html);

    Ok(())
}
```

---

## 📜 License

Licensed under the MIT License.
