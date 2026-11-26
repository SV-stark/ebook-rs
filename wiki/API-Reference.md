# 📚 `ebook-rs` Complete API Reference & Parity Guide (v0.19.4)

This document summarizes `ebook-rs` (**v0.19.4**) APIs across **Drop-in `epub.js` Replacements** and **`ebook-rs` Native Extensions** (see root [`API.md`](https://github.com/SV-stark/ebook-rs/blob/main/API.md) for all 47 sections).

---

## 🟢 Section A: Drop-in Replacement for `epub.js` APIs

| `epub.js` Feature / API | 🚀 `ebook-rs` Equivalent API | Description |
|---|---|---|
| `ePub(url)` / `ePub(buffer)` | `Book::from_file(path)` / `Book::from_bytes(bytes)` / `Book::from_mmap(path)` | Auto-detects and loads 30 formats into a unified `Book`. |
| `book.loaded.metadata` | `book.metadata()` | Returns `&Metadata` (`title`, `creators`, `publishers`, `languages`, `direction`, `accessibility`, etc.). |
| `book.loaded.spine` | `book.spine()` | Returns `&[SpineItem]` ordered chapter items. |
| `book.loaded.navigation.toc` | `book.toc()` | Returns `&[NavPoint]` hierarchical Table of Contents. |
| `book.loaded.navigation.landmarks` | `book.landmarks()` | Returns `&[Landmark]` (cover, titlepage, bodymatter). |
| `book.loaded.navigation.pageList` | `book.page_list()` | Returns `&[PageListItem]` printed page numbers. |
| `book.locations.generate()` | `book.generate_locations(chunk_size)` / `book.locations_from_sections(chunk_size)` | Generates discrete location progress markers across chapters. |
| `book.locations.percentageFromCfi()` | `book.locations().percentage_from_cfi(cfi)` / `percentage_from_location(loc)` | Converts CFIs or location indices to decimal progress (`0.0..=1.0`). |
| `rendition.display(cfi)` | `book.get_section(idx)` / `book.resolve_cfi(cfi, &html)` | Retrieves rendered section HTML, offset, and DOM CSS selector. |
| `rendition.annotations.add()` | `book.annotations_mut().create_highlight(...)` / `create_bookmark(...)` / `add(...)` | Creates, updates, and exports W3C JSON annotations. |

---

## 🔵 Section B: `ebook-rs` Native Extensions (Beyond `epub.js`)

### 1. Universal 28-Format Pure-Rust Exporters (`Book::export_format_bytes`)
Export any loaded `Book` to 28+ pure-Rust output formats (29 identifiers in `Book::supported_output_formats()`) without external tools:

```rust
use ebook_rs::Book;

let book = Book::from_file("alice.epub")?;

// Dynamic dispatch by format name or extension
for fmt in Book::supported_output_formats() {
    let bytes = book.export_format_bytes(fmt)?;
    println!("{fmt}: {} bytes", bytes.len());
}

// Or call typed exporters directly
let kfx  = book.export_kfx_bytes()?;
let mobi = book.export_mobi_bytes()?;
let docx = book.export_docx_bytes()?;
let pdf  = book.export_pdf_bytes()?;
book.export_oeb_dir("alice.oeb")?;
```

Python (`PyO3`) and `UniBook` (UniFFI) bindings also expose `export_format_bytes("docx")` (and all other supported formats).

### 2. Zero-Allocation Full-Text & Bounded Regex Search

```rust
let matches = book.search("Wonderland");
let regex_matches = book.search_regex(r"Chapter\s+\d+")?;
```

### 3. AI RAG Chunking & Okapi BM25 Ranking

```rust
use ebook_rs::{RagChunkConfig, RagChunker};

let chunks = book.to_rag_chunks(&RagChunkConfig::default());
let top = RagChunker::rank_chunks_bm25(&chunks, "Cheshire Cat", 5);
```

### 4. Pre-Display Transformation Hooks & Font Deobfuscation

```rust
use ebook_rs::FontDeobfuscator;

book.register_before_display_hook(|html, _href| {
    *html = html.replace("old-class", "new-class");
});

let deobf = FontDeobfuscator::parse_encryption_xml(encryption_xml);
```

### 5. Built-in MCP (`2024-11-05`) & HTTP Reader Servers
- **MCP Server**: `ebook-rs mcp --sandbox-root ./books` exposes `get_metadata`, `get_toc`, `read_section`, `search_book`, `chunk_book_for_rag`, `convert_ebook` (all 28+ formats + RAG JSON), `export_ebook_text` (inline `md`/`txt`/`html`/`fb2`/`typst`/`adoc`/`org`/`rst`/`tex`), and `validate_epub`, plus `ebook://info` and prompt templates.
- **HTTP Reader Server**: `ReaderServer::new(book, 8080).listen()?` (with optional `.try_with_auth_token(...)` and `.with_sandbox_root(...)`) serves the interactive reader UI and JSON APIs on `127.0.0.1`.

### 6. Optimization, Caching, Overlays & Cross-Language Bindings
- **Lossless EPUB 3 Optimizer**: `EpubOptimizer::optimize(&mut book, &EpubOptimizerOptions::default())` and `book.export_optimized_epub3_bytes(&options)`.
- **Pure-Rust State Cache**: `book.export_cache()` / `Book::from_cache(&bytes)` (`flate2`/`zlib-rs`; legacy `export_zstd_cache` / `from_zstd_cache` are deprecated under the opt-in `zstd` feature).
- **EPUB 3 Media Overlays & LCP DRM**: `book.media_overlays()`, `MediaOverlayPackage`, `LcpLicense`, `LcpDecryptor`, and `Book::from_bytes_with_passphrase`.
- **Cross-Language Bindings**: `UniBook` (Mozilla UniFFI), `PyBook` (`python` feature), `WasmBook` (`wasm` feature), and C FFI (`ebook_rs::ffi`).
