# 📖 EBook-RS: Pure Rust EPUB Parser and Reader Engine

`ebook-rs` is a high-performance, 100% pure Rust EPUB 2 & EPUB 3 parser and reader engine designed for feature parity with **epub.js** and **foliate-js**.

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
        |                   |                     |                   |
        v                   v                     v                   v
 +--------------+   +---------------+     +---------------+   +---------------+
 |  CFI Engine  |   |  Locations    |     | Search Engine |   |  Annotations  |
 | Parser/Range |   |  Progress     |     | Full-Text/CFI |   | Highlights/BM |
 +--------------+   +---------------+     +---------------+   +---------------+
                                       |
                                       v (Optional feature: "server")
                          +--------------------------+
                          | ReaderServer & Web AppUI |
                          |   (http://localhost:8080)|
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
| **4. Core Viewport Architecture** | Standard `<iframe>` | Shadow DOM `<foliate-view>` | None (Library) | Data URI Frame + `ReaderServer` | 🚀 Data URIs |
| **5. Zip Extraction & RAM Loader** | ✅ (`JSZip`) | ✅ (`fflate`) | ✅ (`zip`) | ✅ Zero-copy [`EpubArchive`](file:///E:/ebook-rs/src/archive.rs) | 🟢 100% |
| **6. `container.xml` Rootfile Lookup** | ✅ | ✅ | ✅ | ✅ [`parse_container_xml`](file:///E:/ebook-rs/src/opf.rs) | 🟢 100% |
| **7. OPF Package & Manifest Parser** | ✅ | ✅ | ✅ | ✅ Fast [`parse_opf`](file:///E:/ebook-rs/src/opf.rs) | 🟢 100% |
| **8. Metadata & DC Term Extraction** | ✅ | ✅ | ✅ | ✅ Serde [`Metadata`](file:///E:/ebook-rs/src/metadata.rs) struct | 🟢 100% |
| **9. Automatic Cover Image Extractor** | ✅ | ✅ | ❌ Manual | ✅ Auto binary & MIME resolution | 🟢 100% |
| **10. Table of Contents (EPUB 2 `toc.ncx`)** | ✅ | ✅ | ✅ | ✅ [`parse_ncx`](file:///E:/ebook-rs/src/nav.rs) tree builder | 🟢 100% |
| **11. Table of Contents (EPUB 3 `nav.xhtml`)** | ✅ | ✅ | ✅ | ✅ [`parse_nav_xhtml`](file:///E:/ebook-rs/src/nav.rs) tree builder | 🟢 100% |
| **12. EPUB Canonical Fragment Identifier (CFI)** | ✅ Full IDPF Spec | ✅ Custom `cfi.js` | ❌ None | ✅ Full [`Cfi`](file:///E:/ebook-rs/src/cfi.rs) spec parser | 🟢 100% |
| **13. Range CFI & Step Indirection (`!`)** | ✅ | ✅ | ❌ None | ✅ Step indirection & range CFI | 🟢 100% |
| **14. DOM Range & Selection CFI Generation** | ✅ `Range` walker | ✅ Custom DOM Range | ❌ None | ✅ Text node indexer & CFI resolver | 🟢 100% |
| **15. Location Chunk Generator** | ✅ `locations.generate` | ✅ Granular chunks | ❌ None | ✅ [`Locations`](file:///E:/ebook-rs/src/locations.rs) chunk manager | 🟢 100% |
| **16. `CFI <-> Location <-> Progress %` Mapping** | ✅ | ✅ | ❌ None | ✅ Precise location progress engine | 🟢 100% |
| **17. Full-Text Search Engine Across Spine** | ✅ | ✅ | ❌ None | ✅ [`SearchEngine`](file:///E:/ebook-rs/src/search.rs) across sections | 🟢 100% |
| **18. Search Match CFI Target Generation** | ✅ | ✅ | ❌ None | ✅ Exact target CFI for every result | 🟢 100% |
| **19. Annotations Engine (Highlights/Bookmarks)**| ✅ | ✅ | ❌ None | ✅ [`AnnotationManager`](file:///E:/ebook-rs/src/annotations.rs) | 🟢 100% |
| **20. Highlight Rendering Implementation** | SVG Overlay | CSS Custom Highlight API | ❌ None | Inlined HTML Spans & Struct | 🚀 Native |
| **21. Asset Resolution Mechanism** | Network Blobs | Blob URLs | Raw HTML | Self-contained Base64 Data URIs | 🚀 Zero external links |
| **22. SMIL Audio Sync / Media Overlays**| ✅ `book.media` | ✅ SMIL Audio Sync | ❌ None | 🟡 Planned (Phase 3) | 🚧 Phase 3 |
| **23. RTL & CJK Vertical Writing Modes** | ⚠️ Partial | ✅ Full Support | ❌ None | ✅ `PageProgressionDirection` (RTL/LTR) | 🟢 100% |
| **24. Reflowable & Fixed Layout Modes** | ✅ Iframe viewport | ✅ SVG Scaling | ❌ None | ✅ [`RenditionLayout`](file:///E:/ebook-rs/src/layout.rs) & themes | 🟢 100% |
| **25. Embedded Zero-Dependency HTTP Reader Server**| ❌ Browser | ❌ Browser | ❌ Library | ⚙️ **Optional `server` feature** | 🚀 Superior |
| **26. Zero-Dependency Headless Engine Option** | ❌ JS runtime | ❌ JS runtime | ✅ Library | ✅ `default-features = false` | 🟢 100% |

---

## 📦 Cargo Feature Flags

| Feature | Description | Dependencies | Default |
|---|---|---|:---:|
| **`default`** | Includes `server` (headless parser + embedded web server & UI) | `zip`, `roxmltree`, `serde`, `base64`, `tiny_http`, `url` | ✅ Enabled |
| **`server`** | Enables `ReaderServer` HTTP server & embedded browser reader UI | `tiny_http`, `url` | ✅ Enabled |
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

### 2. Command Line Tools (CLI)
```bash
# Parse metadata, manifest, spine & TOC to JSON:
cargo run -- parse my_book.epub

# Full-text search with exact CFIs and context snippets:
cargo run -- search my_book.epub "Rust"

# Generate location chunks mapping:
cargo run -- locations my_book.epub

# Export sample EPUB 3 file to disk:
cargo run -- sample sample.epub
```

---

## 💻 Rust Library API Usage

### Headless Load & Metadata Inspection
```rust
use ebook_rs::Book;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let book = Book::from_file("sample.epub")?;

    println!("Book Title: {}", book.metadata().title);
    println!("Authors: {:?}", book.metadata().creators);
    println!("Publisher: {:?}", book.metadata().publishers);
    println!("Spine Sections: {}", book.spine().len());

    // Iterate Table of Contents
    for nav_point in book.toc() {
        println!("- {} -> {}", nav_point.label, nav_point.href);
    }

    Ok(())
}
```

### EPUB Canonical Fragment Identifier (CFI) Operations
```rust
use ebook_rs::Cfi;

// Parse a standard EPUB CFI string
let cfi = Cfi::parse("epubcfi(/6/4[chap01ref]!/4/2/10/1:5)")?;
assert_eq!(cfi.spine_index(), 1);
assert_eq!(cfi.char_offset(), 5);

// Format CFI back to string
println!("Formatted: {}", cfi.to_string());
```

---

## 🗺 Roadmap

### 🏁 Phase 1: Core Engine & Parity (v0.1 - Completed)
- [x] Pure Rust Zip container extraction and case-insensitive file lookup.
- [x] Full EPUB 2 and EPUB 3 metadata & manifest OPF package parser.
- [x] Dual NCX and NAV XHTML Table of Contents parser.
- [x] Full EPUB Canonical Fragment Identifier (CFI) spec parser, stringifier, range parser, and evaluator.
- [x] Location chunking and progress calculation engine (`CFI <-> Location <-> Percentage`).
- [x] Full-text search engine with snippet extraction and CFI generation.
- [x] Annotation manager for highlights, bookmarks, underlines, and notes.
- [x] Self-contained section resource inliner (converting images/CSS/fonts to Data URIs).
- [x] Optional `server` feature for built-in `ReaderServer` and responsive HTML5 web interface.

### 🚀 Phase 2: WebAssembly & Frontend Bindings (v0.2)
- [ ] Export `wasm-bindgen` JS wrappers for direct client-side web browser integration.
- [ ] Add direct canvas/SVG rendering options for pre-paginated fixed layout EPUBs.
- [ ] Implement text node DOM range resolution in browser iframe environment.

### 🎨 Phase 3: Advanced Reader Capabilities (v0.3)
- [ ] Media Overlays (EPUB 3 Synchronized Audio & SMIL XML parsing).
- [ ] Multi-format ebook loader (Kindle MOBI / AZW3 & FB2 parsers).
- [ ] Encrypted EPUB / DRM extension hook interfaces.
- [ ] Multi-column reflow CSS column engine for double-spread layout rendering.

### 💎 Phase 4: Native GUI & TUI Applications (v1.0)
- [ ] Built-in Terminal User Interface (TUI) reader using `ratatui`.
- [ ] Cross-platform desktop GUI reader application built with `egui` / `wgpu`.

---

## 📜 License
Licensed under the MIT License.
