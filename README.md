# 📖 EBook-RS: Pure Rust EPUB, MOBI & AZW3 Reader Engine

`ebook-rs` is a high-performance, 100% pure Rust EPUB 2, EPUB 3, MOBI, and AZW3 parser and reader engine designed for feature parity with **epub.js** and **foliate-js**.

---

## ⚡ Overview & Key Architecture

`ebook-rs` provides a complete library and interactive reader application for reading, parsing, querying, searching, and annotating EPUB publications in Rust and WebAssembly environments.

```
                           +------------------------+
                           |  EPUB Archive (.epub)  |
                           +-----------+------------+
                                       |
                                       v
                          +--------------------------+
                          |   EpubArchive (Zip/RAM)  |
                          +------------+-------------+
                                       |
                   +-------------------+-------------------+
                   |                   |                   |
                   v                   v                   v
           +---------------+   +---------------+   +---------------+
           |  OPF Package  |   |  TOC Engine   |   | Section Loader|
           | Metadata/Spine|   |  NCX / NAV    |   | Inliner/Base64|
           +-------+-------+   +-------+-------+   +-------+-------+
                                       |                   |
                                       +-------------------+
                                       |
                                       v
                          +--------------------------+
                          |   Book Core API Engine   |
                          +------------+-------------+
                                       |
        +-------------------+----------+----------+-------------------+
        |                   |          |          |                   |
        v                   v          v          v                   v
 +--------------+   +---------------+ +--------+ +---------------+ +---------------+
 |  CFI Engine  |   |  Locations    | | WASM   | | Search Engine | |  Annotations  |
 | Parser/Range |   |  Progress     | | Bindings| | Full-Text/CFI | | Highlights/BM |
 +--------------+   +---------------+ +--------+ +---------------+ +---------------+
                                       |
                                       v (Optional feature: "server")
                          +--------------------------+
                          | ReaderServer & Web AppUI |
                          | (Double-Spread/Scroll)   |
                          +--------------------------+
```

---

## 📊 Comprehensive 26-Feature Architectural Matrix

This matrix presents a side-by-side comparison of standard reading capabilities and key architectural differentiators across **`epub.js`**, **`foliate-js`**, **`rbook`**, and **`ebook-rs`**:

| Feature Category / Architectural Capability | `epub.js` (JavaScript) | `foliate-js` (JavaScript) | `rbook` (Rust Crate) | `ebook-rs` (Our Pure Rust Engine) | Status |
|---|:---:|:---:|:---:|:---:|:---:|
| **1. EPUB 2 Specification Support** | ✅ | ✅ | ✅ | ✅ | 🟢 100% |
| **2. EPUB 3 Specification Support** | ✅ | ✅ | ✅ | ✅ | 🟢 100% |
| **3. Non-EPUB Formats (MOBI / FB2 / CBZ)** | ❌ EPUB only | ✅ Multi-format | ❌ EPUB only | 🟡 EPUB (MOBI planned) | 🚧 Phase 3 |
| **4. WebAssembly (WASM) Client Bindings** | ❌ JS Runtime | ❌ JS Runtime | ⚠️ Basic | ✅ `wasm-bindgen` (`WasmBook`) | 🟢 100% |
| **5. Core Viewport Architecture** | Standard `<iframe>` | Shadow DOM `<foliate-view>` | None (Library) | Data URI Frame + `ReaderServer` | 🟢 100% |
| **6. Zip Extraction & RAM Loader** | ✅ (`JSZip`) | ✅ (`fflate`) | ✅ (`zip`) | ✅ Zero-copy [`EpubArchive`](file:///E:/ebook-rs/src/archive.rs) | 🟢 100% |
| **7. `container.xml` Rootfile Lookup** | ✅ | ✅ | ✅ | ✅ [`parse_container_xml`](file:///E:/ebook-rs/src/opf.rs) | 🟢 100% |
| **8. OPF Package & Manifest Parser** | ✅ | ✅ | ✅ | ✅ Fast [`parse_opf`](file:///E:/ebook-rs/src/opf.rs) | 🟢 100% |
| **9. Metadata & DC Term Extraction** | ✅ | ✅ | ✅ | ✅ Serde [`Metadata`](file:///E:/ebook-rs/src/metadata.rs) struct | 🟢 100% |
| **10. Automatic Cover Image Extractor** | ✅ | ✅ | ❌ Manual | ✅ Auto binary & MIME resolution | 🟢 100% |
| **11. Table of Contents (EPUB 2 `toc.ncx`)** | ✅ | ✅ | ✅ | ✅ [`parse_ncx`](file:///E:/ebook-rs/src/nav.rs) tree builder | 🟢 100% |
| **12. Table of Contents (EPUB 3 `nav.xhtml`)** | ✅ | ✅ | ✅ | ✅ [`parse_nav_xhtml`](file:///E:/ebook-rs/src/nav.rs) tree builder | 🟢 100% |
| **13. EPUB 3 Landmarks & Page-List Nav** | ✅ | ✅ | ❌ None | ✅ `book.landmarks()`, `book.page_list()` | 🟢 100% |
| **14. EPUB Canonical Fragment Identifier (CFI)** | ✅ Full IDPF Spec | ✅ Custom `cfi.js` | ❌ None | ✅ Full [`Cfi`](file:///E:/ebook-rs/src/cfi.rs) spec parser | 🟢 100% |
| **15. Range CFI & Step Indirection (`!`)** | ✅ | ✅ | ❌ None | ✅ Step indirection & range CFI | 🟢 100% |
| **16. DOM Range & Selection CFI Generation** | ✅ `Range` walker | ✅ Custom DOM Range | ❌ None | ✅ Live DOM Selection <-> CFI Bridge | 🟢 100% |
| **17. Location Chunk Generator** | ✅ `locations.generate` | ✅ Granular chunks | ❌ None | ✅ [`Locations`](file:///E:/ebook-rs/src/locations.rs) chunk manager | 🟢 100% |
| **18. `CFI <-> Location <-> Progress %` Mapping** | ✅ | ✅ | ❌ None | ✅ Precise location progress engine | 🟢 100% |
| **19. Full-Text Search Engine Across Spine** | ✅ | ✅ | ❌ None | ✅ [`SearchEngine`](file:///E:/ebook-rs/src/search.rs) across sections | 🟢 100% |
| **20. Annotations Engine (Highlights/Bookmarks)**| ✅ | ✅ | ❌ None | ✅ [`AnnotationManager`](file:///E:/ebook-rs/src/annotations.rs) | 🟢 100% |
| **21. Pre-Display Pipeline Hooks** | ✅ `beforeDisplay` | ❌ None | ❌ None | ✅ `register_before_display_hook` | 🟢 100% |
| **22. IDPF & Adobe Font De-Obfuscation** | ✅ `encryption.xml` | ✅ `encryption.xml` | ❌ None | ✅ [`FontDeobfuscator`](file:///E:/ebook-rs/src/deobfuscate.rs) engine | 🟢 100% |
| **23. Double-Spread & Column Pagination** | ✅ Iframe columns | ✅ Multi-column | ❌ None | ✅ Single & Double Page Spread Engine | 🟢 100% |
| **24. Continuous Vertical Scroll (`scrolled-doc`)**| ✅ Scrolled flow | ✅ Vertical scroll | ❌ None | ✅ `scrolled-doc` Continuous Scroll | 🟢 100% |
| **25. Embedded Zero-Dependency HTTP Reader Server**| ❌ Browser | ❌ Browser | ❌ Library | ⚙️ **Optional `server` feature** | 🚀 Superior |
| **26. Zero-Dependency Headless Engine Option** | ❌ JS runtime | ❌ JS runtime | ✅ Library | ✅ `default-features = false` | 🟢 100% |

---

## 📦 Cargo Feature Flags

| Feature | Description | Dependencies | Default |
|---|---|---|:---:|
| **`default`** | Includes `server` (headless parser + embedded web server & UI) | `zip`, `roxmltree`, `serde`, `base64`, `tiny_http`, `url` | ✅ Enabled |
| **`server`** | Enables `ReaderServer` HTTP server & embedded browser reader UI | `tiny_http`, `url` | ✅ Enabled |
| **`wasm`** | Enables WebAssembly client bindings (`WasmBook`) for browser JS apps | `wasm-bindgen` | ⚪ Optional |
| *(None)* (`default-features = false`) | Minimal, lightweight headless EPUB parser & core reader engine | `zip`, `roxmltree`, `serde`, `base64` | ⚪ Headless |

---

## 🛠 Quick Start

### 1. Launch Interactive Reader Server
Start the embedded reader web application on `http://localhost:8080`:

```bash
# Launch with built-in sample EPUB 3:
cargo run -- serve

# Launch with your own EPUB file:
cargo run -- serve /path/to/my_book.epub
```

---

## 💻 Rust Library API Usage

### Headless Load & Pre-Display Hooks
```rust
use ebook_rs::Book;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut book = Book::from_file("sample.epub")?;

    // Register a custom pre-display HTML transformation hook
    book.register_before_display_hook(|html, path| {
        html.push_str("<div class='footer-note'>Read via EBook-RS Engine</div>");
    });

    println!("Book Title: {}", book.metadata().title);
    println!("Landmarks count: {}", book.landmarks().len());
    println!("Page List count: {}", book.page_list().len());

    Ok(())
}
```

---

## 📜 License
Licensed under the MIT License.
