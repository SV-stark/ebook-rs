# 📖 Welcome to the `ebook-rs` Wiki

`ebook-rs` (**v0.19.0**) is a high-performance, 100% pure-Rust eBook engine supporting **30 input formats and 27 output formats** — **EPUB 2/3**, **KEPUB**, **Amazon KFX (KF10)**, **MOBI**, **AZW3 (KF8)**, **PDB (PalmDOC/zTXT)**, **FB2**, **LIT**, **CBZ**, **PDF**, **DOCX**, **ODT**, **RTF**, **PPTX**, **HTML**, **HTMLZ**, **TXT**, **TXTZ**, **Markdown**, **OEB**, **ZIP**, **TCR**, **PMLZ**, **RB**, **Typst**, **AsciiDoc**, **Org-mode**, **reStructuredText**, and **LaTeX** — with native AI RAG chunking, an MCP `2024-11-05` server, Mozilla UniFFI/PyO3/C FFI bindings, and WebAssembly support.

---

## 🧭 Complete Wiki Sitemap

1. **[Getting Started](Getting-Started)**
   - Installation & Cargo / CLI Setup (`v0.19.0`)
   - Auto-Detecting 30 Input Formats & Exporting to 27 Output Formats
   - Full-Text Search & Reading Progress
2. **[Multi-Format Architecture](Multi-Format-Architecture)**
   - Unified `Book` Model & `src/formats/` Codec Layer
   - Clean-Room Amazon KFX Ion 1.0 `CONT` v2 Codec
   - Bidirectional Calibre 9.18 CLI Interoperability
3. **[CFI and Locations Engine](CFI-and-Locations)**
   - EPUB Canonical Fragment Identifiers (CFI) & DOM Resolution
   - Range Parsing, Comparing, and In-Section Character Offsets
   - Discrete Location Chunk Indexing
4. **[WASM and Server Integration](WASM-and-Server-Integration)**
   - WebAssembly Browser Bindings & `<ebook-reader>` Web Component
   - Built-in Multithreaded HTTP Reader Server
   - Built-in Model Context Protocol (`MCP 2024-11-05`) Server
5. **[API Reference & `epub.js` Parity](API-Reference)**
   - Drop-in `epub.js` Replacements
   - Universal 27-Format Pure-Rust Exporters (`Book::export_format_bytes`)
   - Native AI RAG BM25 Chunking, FFI, Python (`PyO3`), and MCP Tools

---

## ⚡ Performance Highlights (`v0.19.0` vs Calibre 9.18)
- **Universal 27-Format Pure-Rust Converter**: `EPUB → KFX` (**~195× faster**), `EPUB → PDF` (**60.3× faster**), `DOCX → EPUB` (**18.1× faster**), `TCR → EPUB` (**15.0× faster**), `AZW3 → EPUB` (**14.7× faster**), `PDB → EPUB` (**14.7× faster**).
- **Native AI & RAG Chunking + MCP Server**: Semantic chunking with Okapi BM25 ranking, CFI citations, sandboxed 27-format `convert_ebook`, and inline `export_ebook_text`.
- **Multi-Language Bindings**: Rust crate, CLI, Python (`PyO3`), WebAssembly (`wasm-bindgen`), Mobile (`Mozilla UniFFI` Swift/Kotlin), and C ABI (`ebook_rs::ffi`).
- **Pure Rust 2024 Edition**: Zero C/C++/Python/JVM runtime dependencies.
