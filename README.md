# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.2.0)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, and **LIT (Microsoft Reader)** formats, designed for full feature parity with **epub.js** and **foliate-js**.

---

## ⚡ Feature Parity Matrix

| Feature | 🚀 `ebook-rs` (v0.2.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **MOBI & AZW3 Support** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2) Support** | ✅ Native XML | ❌ No | ✅ Yes | ❌ No |
| **KEPUB & LIT Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
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

---

## 🗺️ Project Roadmap

- [x] **v0.1.0**: Core EPUB 2/3 parser, CFI Engine, Location Progress, Annotations, Web UI Server, WASM Bindings.
- [x] **v0.2.0**: Native Multi-Format Support (**MOBI**, **AZW3**, **FB2**, **KEPUB**, **LIT**), IDPF & Adobe Font De-obfuscation, EPUB 3 Landmarks & Page List, Pre-Display Transformation Hooks, 100% Audit Bug Fixes.
- [ ] **v0.3.0** *(Planned)*: PDF Rendering Bridge, CBZ/CBR Comic Archive Parser, Readium Webpub Manifest Export.

---

## 🚀 Quick Start Example

```rust
use ebook_rs::Book;

fn main() -> Result<(), String> {
    // Auto-detects and opens EPUB, MOBI, AZW3, FB2, KEPUB, or LIT files
    let mut book = Book::from_file("book.azw3")?;

    println!("Title: {}", book.metadata().title);
    println!("Sections: {}", book.spine().len());

    // Search across all chapters
    let matches = book.search("Rabbit");
    println!("Found {} search matches", matches.len());

    Ok(())
}
```

---

## 📄 License

Dual-licensed under [MIT](LICENSE) or Apache-2.0.
