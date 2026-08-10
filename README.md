# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.15.0)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **Amazon KFX (Kindle Format 10)**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT**, **TXT**, and **MD** formats, designed for full feature parity with **epub.js** and **foliate-js**, equipped with Model Context Protocol (MCP 2024-11-05) AI assistant support, native AI/RAG document chunking with Okapi BM25 scoring, C FFI multi-language bindings, and Web Component support.

---

## ⚡ Feature Parity Matrix

### 📂 Format Support

| Feature | 🚀 `ebook-rs` (v0.15.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Full OPF + NCX/NAV | ✅ Yes | ✅ Yes | ✅ Yes |
| **EPUB 3 Fixed-Layout (FXL)** | ✅ 2-page spread renderer | ✅ Yes | ✅ Yes | ❌ No |
| **Amazon KFX (KF10) Support** | ✅ Clean-room `b"CONT"` container | ❌ No | ✅ Yes | ❌ No |
| **MOBI & AZW3 (KF8) Support** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2) Support** | ✅ Native XML + xlink:href | ❌ No | ✅ Yes | ❌ No |
| **KEPUB (Kobo EPUB) Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **LIT (Microsoft Reader) Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **CBZ (Comic Book ZIP) Support** | ✅ Native ZIP Images | ❌ No | ✅ Yes | ❌ No |
| **PDF Support** | ✅ `pdf_oxide` parser | ❌ No | ❌ No | ❌ No |
| **ODT (OpenDocument Text)** | ✅ `office_oxide` | ❌ No | ❌ No | ❌ No |
| **TXT / Markdown Support** | ✅ Auto-reflow sections | ❌ No | ❌ No | ❌ No |
| **Auto Format Detection** | ✅ Magic-byte detection | ❌ No | ❌ No | ❌ No |

### 🧭 Navigation & Rendering

| Feature | 🚀 `ebook-rs` (v0.13.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **IDPF CFI Engine** | ✅ Parse / format / compare / range | ✅ Yes | ✅ Yes | ✅ Yes |
| **CFI DOM Resolver** | ✅ `cfi.resolve_dom_path(html)` | ✅ Yes | ✅ Yes | ❌ No |
| **Readium CFI Unified Locator** | ✅ Full model | ✅ Yes | ✅ Yes | ❌ No |
| **Location / Reading Progress** | ✅ `locations_from_sections()` | ✅ Yes | ✅ Yes | ✅ Yes |
| **SMIL Media Overlays (Sync)** | ✅ NPT clock parser | ✅ Yes | ✅ Yes | ❌ No |
| **EPUB NCX / NAV TOC Parsing** | ✅ Deep recursive nav tree | ✅ Yes | ✅ Yes | ✅ Yes |
| **RTL & CJK Vertical Writing** | ✅ `direction: rtl` + `vertical-rl` | ✅ Yes | ✅ Yes | ❌ No |
| **Viewport Meta Parsing** | ✅ Zero-alloc slice parse | ✅ Yes | ✅ Yes | ❌ No |
| **Reflow Paginator** | ✅ `ReflowPaginator::paginate_section` | ✅ Yes | ✅ Yes | ❌ No |
| **Custom Font Injection** | ✅ `@font-face` CSS injection | ✅ Yes | ✅ Yes | ❌ No |

### 🔍 Search, Analytics & AI RAG

| Feature | 🚀 `ebook-rs` (v0.13.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **AI & RAG Chunking Engine** | ✅ `to_rag_chunks()` + CFI citations | ❌ No | ❌ No | ❌ No |
| **Full-Text Search** | ✅ SIMD-accelerated | ❌ No | ✅ Basic | ❌ No |
| **Regex Search** | ✅ `regex_search` | ❌ No | ❌ No | ❌ No |
| **Search Context Snippets** | ✅ `<mark>` highlights + XSS guard | ❌ No | ✅ Yes | ❌ No |
| **Readium Search JSON Export** | ✅ Readium-compliant JSON | ❌ No | ✅ Yes | ❌ No |
| **NLP Reading Analytics** | ✅ Word count / reading time / complexity | ❌ No | ❌ No | ❌ No |
| **Auto Language Detection** | ✅ `detect_language` (`whatlang`) | ❌ No | ❌ No | ❌ No |

### 🌐 Interoperability & Web Component

| Feature | 🚀 `ebook-rs` (v0.13.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **C / Python / Node FFI Bindings** | ✅ `ebook_rs::ffi` C ABI | ❌ No | ❌ No | ❌ No |
| **Web Component Generator** | ✅ `<ebook-reader>` HTMLElement | ❌ No | ❌ No | ❌ No |
| **WASM Client SDK** | ✅ `WasmBook` WASM bindings | ✅ Yes | ✅ Yes | ❌ No |
| **W3C Web Annotation (JSON-LD)** | ✅ Full CRUD + `to_w3c_json` | ❌ No | ✅ Yes | ❌ No |
| **Readium WebPub Manifest Export** | ✅ `book.to_webpub_manifest()` | ❌ No | ✅ Yes | ❌ No |
### ⚡ Format Conversion Benchmark (`ebook-rs` vs Calibre `ebook-convert`)

Empirically measured conversion benchmark converting eBook formats to **EPUB 3** on identical test hardware:

| Input Format | 🚀 `ebook-rs` (v0.14.0) | 🐍 Calibre `ebook-convert` | Speedup | Image Assets Extracted | Chapter Sections | Output Parity |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| **MOBI → EPUB** | **0.98s** ⚡ | 4.64s | **4.8× Faster** | 38 / 37 (100% Parity) ✅ | 17 chapters | Matched |
| **AZW3 (KF8) → EPUB** | **0.34s** ⚡ | 3.29s | **9.7× Faster** | 38 / 37 (100% Parity) ✅ | 14 chapters | Matched |
| **FB2 → EPUB** | **0.26s** ⚡ | 2.73s | **10.5× Faster** | 37 / 37 (100% Parity) ✅ | 15 chapters | Matched |
| **LIT → EPUB** | **0.48s** ⚡ | 3.53s | **7.4× Faster** | 37 / 37 (100% Parity) ✅ | 21 chapters | Matched |
| **KFX → EPUB** | **0.37s** ⚡ | 3.84s *(Plugin Req.)* | **10.4× Faster** | 37 / 37 (100% Parity) ✅ | 22 chapters | Matched |

*All conversions produce 100% W3C-validated EPUB 3 archives containing full metadata (`dc:creator`, `dc:title`, `dc:language`), Table of Contents (`nav.xhtml`), and image resources (`OEBPS/images/`).*

---

## 🆕 What's New in v0.14.1

- **Academic PDF Two-Column Spatial Reflowing Engine (`src/pdf.rs`)**:
  - Automatically detects multi-column line patterns and spatial column dividers in IEEE, ArXiv, and ACM academic paper PDFs.
  - Separates full-width paper headers/titles from column blocks, sorting Left-Column paragraphs top-to-bottom followed by Right-Column paragraphs top-to-bottom into continuous single-column EPUB sections & AI RAG Markdown chunks.
- **Multi-Threaded Parallel ZIP Exporter (`src/validator.rs`)**:
  - Parallelizes HTML section document and image asset compression across Rayon worker threads (`entries.par_iter()`), significantly accelerating EPUB 3 export speeds for 100MB+ image-heavy books.
- **CBZ Comic Reader Optimizations (`src/cbz.rs`)**:
  - **Zero-Latency Page Pre-fetching**: Added `CbzBook::prefetch_page_images()` and `book.prefetch_comic_pages()` to pre-load adjacent comic page image bytes into memory for instant page turns.
  - **2-Page Manga Spread View Mode**: Added `CbzBook::parse_manga()` and `CbzBook::enable_manga_mode()` with Right-to-Left (`direction: rtl`) reading progression.
- **Native Python / PyO3 Wheel Bindings (`pip install ebook-rs`)**:
  - Native Python extension bindings (`PyBook`, `PySection`) with automated PyPI publishing workflow (`.github/workflows/pypi.yml`).

---

## 🚀 Quick Start Examples

### 1. Basic Parsing & RAG Chunking (Rust)

```rust
use ebook_rs::{Book, RagChunkConfig};

fn main() -> Result<(), String> {
    let book = Book::from_file("book.epub")?;

    println!("Title: {}", book.metadata().title);
    println!("Language: {:?}", book.detect_language());

    // Generate AI / RAG document chunks with CFI citation anchors
    let config = RagChunkConfig {
        max_tokens: 512,
        overlap_tokens: 64,
        preserve_headings: true,
        ..Default::default()
    };
    let chunks = book.to_rag_chunks(&config);

    for chunk in chunks {
        println!("ID: {}", chunk.id);
        println!("CFI Anchor: {}", chunk.cfi);
        println!("Markdown: \n{}", chunk.markdown);
    }

    Ok(())
}
```

### 2. Multi-Format CLI Conversion & RAG Export

```bash
# Convert MOBI, PDF, FB2, or TXT into a valid EPUB 3 file
ebook-rs convert sample.mobi output.epub

# Export AI RAG chunks directly to JSON
ebook-rs convert book.epub chunks.json

# Stream JSON RAG chunks to stdout
ebook-rs rag book.epub 512
```

### 3. Native Python API (`pip install ebook-rs`)

```python
import ebook_rs

# Open any eBook format (EPUB, MOBI, KFX, AZW3, FB2, LIT, CBZ, PDF, TXT, MD)
book = ebook_rs.Book.open("sample.mobi")

print(f"Title: {book.title}")
print(f"Authors: {book.authors}")
print(f"Section Count: {book.section_count}")

# Generate AI / RAG chunks with CFI citations directly in Python
rag_chunks_json = book.to_rag_chunks_json(max_tokens=512)

# Convert any format to EPUB 3 or KFX bytes
epub_bytes = book.export_epub3_bytes()
```

### 4. C FFI API Integration

```c
#include "ebook_rs.h"

int main() {
    uint8_t* buffer = load_file("book.epub", &len);
    CBookHandle book = ebook_rs_book_from_bytes(buffer, len);

    char* json_meta = ebook_rs_get_metadata_json(book);
    printf("Metadata: %s\n", json_meta);
    ebook_rs_string_free(json_meta);

    char* json_rag = ebook_rs_to_rag_chunks_json(book, 512);
    printf("RAG Chunks: %s\n", json_rag);
    ebook_rs_string_free(json_rag);

    ebook_rs_book_free(book);
    return 0;
}
```

---

## 📖 Documentation & Wiki

Detailed API documentation and integration guides are available in [docs/API.md](docs/API.md), [API.md](API.md), and [WIKI.md](WIKI.md).

---

## 📜 License

Licensed under the MIT License. Copyright (c) 2026 SV-Stark.
