# 📚 `ebook-rs` Complete API Reference & Parity Guide (v0.20.0)

This document summarizes `ebook-rs` (**v0.20.0**) APIs across **Drop-in `epub.js` Replacements** and **`ebook-rs` Native Extensions** (see root [`API.md`](https://github.com/SV-stark/ebook-rs/blob/main/API.md) for all 48 sections).

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
| `book.locations.percentageFromCfi()` | `book.locations().percentage_from_cfi(cfi)` / `progression_from_cfi(cfi)` / `progression_from_char_offset(idx, off)` | Converts CFIs or character offsets to decimal progress (`0.0..=1.0`), including smooth character-weighted progression. |
| `book.locations.cfiFromPercentage()` | `book.position_from_progression(pct)` / `book.locator_from_progression(pct)` / `book.locations().location_from_percentage(pct)` / `book.book_progress(idx, off, wpm)` / `book.book_reading_time_minutes(wpm)` | Reverse-seeks from `0.0..=1.0` slider progress and builds unified e-reader progress snapshots (`BookProgress`) with reading-time estimates. |
| `rendition.display(cfi)` | `book.get_section(idx)` / `book.get_section_by_href(href)` / `book.resolve_cfi(cfi, &html)` | Retrieves rendered section HTML (by index, href, or CFI), offset, and DOM CSS selector. |
| `book.coverUrl()` | `book.cover_image()` | Returns `Option<(Vec<u8>, &'static str)>` cover image bytes and MIME type. |
| `book.archive.getBlob(url)` | `book.get_resource_bytes(path)` | Returns `(Vec<u8>, &'static str)` raw resource bytes and MIME type. |
| `rendition.annotations.add()` | `book.annotations_mut().create_highlight(...)` / `create_bookmark(...)` / `to_w3c_json()` / `to_w3c_json_with_source(iri)` | Creates, updates, and exports W3C JSON-LD annotations (with ISO-8601 UTC timestamps and optional `SpecificResource` `source`). |

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
let limited = book.search_limited("Wonderland", Some(25));
let regex_matches = book.search_regex(r"Chapter\s+\d+")?;
let limited_regex = book.search_regex_limited(r"Chapter\s+\d+", Some(25))?;
let readium_search_json = book.to_readium_search_json(&matches, "Wonderland")?;
```

### 3. AI RAG Chunking & Okapi BM25 Ranking

```rust
use ebook_rs::{RagChunkConfig, RagChunker};

let chunks = book.to_rag_chunks(&RagChunkConfig::default());
let top = RagChunker::rank_chunks_bm25(&chunks, "Cheshire Cat", 5);
```

### 4. Pre-Display Transformation Hooks & Font Deobfuscation

```rust
use ebook_rs::{FontDeobfuscator, inline_stylesheet_rules};

book.register_before_display_hook(|html, _href| {
    *html = html.replace("old-class", "new-class");
});
book.invalidate_render_cache();

let inlined = inline_stylesheet_rules(&section.raw_html);
let footnotes = section.extract_footnotes();
let tts_tokens = section.tokenize_tts_words();
let tts_html = section.to_tts_annotated_html();

let deobf = book.font_deobfuscator();
let parsed_deobf = FontDeobfuscator::parse_encryption_xml(encryption_xml);
```

Section HTML utilities in `ebook_rs::section`: `extract_plain_text`, `sanitize_html_scripts`, `Section::strip_script_content`, `parse_viewport_meta`, and `process_section_resources`.

### 5. Built-in MCP (`2024-11-05`) & HTTP Reader Servers
- **MCP Server**: `ebook-rs mcp --sandbox-root ./books` exposes `get_metadata`, `get_toc`, `read_section`, `search_book`, `chunk_book_for_rag`, `convert_ebook` (all 28+ formats + RAG JSON), `export_ebook_text` (inline `md`/`txt`/`html`/`fb2`/`typst`/`adoc`/`org`/`rst`/`tex`), and `validate_epub`, plus `ebook://info` and prompt templates.
- **HTTP Reader Server**: `ReaderServer::new(book, 8080).listen()?` (with optional `.try_with_auth_token(...)` and `.with_sandbox_root(...)`) serves the interactive reader UI and JSON APIs on `127.0.0.1`.

### 6. Optimization, Caching, Overlays & Cross-Language Bindings
- **Lossless EPUB 3 Optimizer**: `EpubOptimizer::optimize(&mut book, &EpubOptimizerOptions::default())` and `book.export_optimized_epub3_bytes(&options)`.
- **Pure-Rust State Cache**: `book.export_cache()` / `Book::from_cache(&bytes)` (`flate2`/`zlib-rs`; legacy `export_zstd_cache` / `from_zstd_cache` are deprecated under the opt-in `zstd` feature).
- **EPUB 3 Media Overlays & LCP DRM**: `book.media_overlays()`, `MediaOverlayPackage`, `LcpLicense` (including `encryption.content_key`), `LcpDecryptor`, and `Book::from_bytes_with_passphrase`.
- **Lazy Hydration & Async I/O**: `book.get_section_raw(idx)`, `book.load_section_lazy(idx)`, `book.hydrate_all_sections()`, `book.get_all_sections_hydrated()`, and `ebook_rs::book::async_api::{from_file_async, from_bytes_async, load_section_lazy_async}`.
- **WebPub, OPDS Feeds & Sample Builder**: `book.to_webpub_manifest()` (recursive `toc`, non-spine `resources`, `identifier`/`description`), `OpdsFeed::parse_atom_xml`, `OpdsFeed::parse_json` (`modified`, `images`, `navigation`, `groups`), and `sample_builder::generate_sample_epub()`.
- **Cross-Language Bindings**: `UniBook` (Mozilla UniFFI), `PyBook` (`python` feature), `WasmBook` (`wasm` feature), and C FFI (`ebook_rs::ffi`), all exposing Readium locator, `Locations`, reverse-seek, and `BookProgress` JSON helpers.
