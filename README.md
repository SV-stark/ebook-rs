# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.13.0)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **Amazon KFX (Kindle Format 10)**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT**, **TXT**, and **MD** formats, designed for full feature parity with **epub.js** and **foliate-js**, equipped with native AI/RAG document chunking, C FFI multi-language bindings, and Web Component support.

---

## ⚡ Feature Parity Matrix

### 📂 Format Support

| Feature | 🚀 `ebook-rs` (v0.13.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
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
| **OPDS 1.2 / 2.0 Catalog** | ✅ Atom XML + JSON parser | ❌ No | ❌ No | ❌ No |

---

## 🆕 What's New in v0.12.0

- **Native AI & RAG Document Chunking Engine (`src/rag.rs`)**:
  - `book.to_rag_chunks(&config)` splits eBooks into AI-ready semantic chunks with Markdown heading hierarchy, token estimations, and exact `epubcfi` citation anchors for Vector DBs and LLM prompt ingestion.
- **C / Multi-Language FFI Bindings (`src/ffi.rs`)**:
  - C-compatible ABI (`#[unsafe(no_mangle)] extern "C"`) functions for zero-copy integration across Python (`ctypes`/`pyo3`), Node.js (`ffi-napi`), C/C++, Swift (iOS), Kotlin (Android), and Go.
- **Zero-Dependency `<ebook-reader>` Web Component (`src/wasm.rs`)**:
  - `WasmBook::get_custom_element_js()` generates a standalone `<ebook-reader>` custom element for plug-and-play web reader integration in React, Vue, Svelte, and Next.js.
- **CJK Vertical Text & RTL Reading Modes (`src/layout.rs`)**:
  - Added `WritingMode` enum (`HorizontalLtr`, `HorizontalRtl`, `VerticalRl`, `VerticalLr`) with dynamic CSS overrides for Arabic, Hebrew, Japanese, Chinese, and Korean vertical reading layout.
- **CLI Converter & RAG Commands (`src/main.rs`)**:
  - `ebook-rs convert <input> <output.epub>` converts any supported format into valid EPUB 3 using `UniversalEpub3Exporter`.
  - `ebook-rs rag <input.epub> [max_tokens]` generates JSON AI RAG chunks directly from the terminal.

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

### 3. C / Python FFI API Integration

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
