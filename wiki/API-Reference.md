# 📚 `ebook-rs` Complete API Reference & Parity Guide (v0.19.0)

This document summarizes `ebook-rs` (**v0.19.0**) APIs across **Drop-in `epub.js` Replacements** and **`ebook-rs` Native Extensions** (see root [`API.md`](https://github.com/SV-stark/ebook-rs/blob/main/API.md) for all 38 sections).

---

## 🟢 Section A: Drop-in Replacement for `epub.js` APIs

| `epub.js` Feature / API | 🚀 `ebook-rs` Equivalent API | Description |
|---|---|---|
| `ePub(url)` / `ePub(buffer)` | `Book::from_file(path)` / `Book::from_bytes(bytes)` | Auto-detects and loads 30 formats into a unified `Book`. |
| `book.loaded.metadata` | `book.metadata()` | Returns `&Metadata` (`title`, `creators`, `publishers`, `languages`, `direction`, etc.). |
| `book.loaded.spine` | `book.spine()` | Returns `&[SpineItem]` ordered chapter items. |
| `book.loaded.navigation.toc` | `book.toc()` | Returns `&[NavPoint]` hierarchical Table of Contents. |
| `book.loaded.navigation.landmarks` | `book.landmarks()` | Returns `&[Landmark]` (cover, titlepage, bodymatter). |
| `book.loaded.navigation.pageList` | `book.page_list()` | Returns `&[PageListItem]` printed page numbers. |
| `book.locations.generate()` | `book.generate_locations(chunk_size)` | Generates discrete location progress markers across chapters. |
| `book.locations.percentageFromCfi()` | `book.locations().percentage_from_location(loc)` | Converts location indices to decimal progress (`0.0..=1.0`). |
| `rendition.display(cfi)` | `book.get_section(idx)` / `book.resolve_cfi(...)` | Retrieves rendered section HTML, offset, and DOM CSS selector. |
| `rendition.annotations.add()` | `book.annotations_mut().add_annotation(...)` | Creates, updates, and exports W3C JSON-LD annotations. |

---

## 🔵 Section B: `ebook-rs` Native Extensions (Beyond `epub.js`)

### 1. Universal 27-Format Pure-Rust Exporters (`Book::export_format_bytes`)
Export any loaded `Book` to 27 pure-Rust output formats without external tools:

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

Python (`PyO3`) bindings also expose `book.export_format_bytes("docx")` (and all other supported formats).

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
book.add_before_display_hook(|html| html.replace("old-class", "new-class"));
```

### 5. Built-in MCP (`2024-11-05`) & HTTP Reader Servers
- **MCP Server**: `ebook-rs mcp --sandbox-root ./books` exposes `get_metadata`, `get_toc`, `read_section`, `search_book`, `chunk_book_for_rag`, `convert_ebook` (all 27+ formats + RAG JSON), `export_ebook_text` (inline `md`/`txt`/`html`/`fb2`/`typst`/`adoc`/`org`/`rst`/`tex`), and `validate_epub`.
- **HTTP Reader Server**: `ReaderServer::new(book, 8080).start()?` serves the interactive reader UI and JSON APIs on `127.0.0.1`.
