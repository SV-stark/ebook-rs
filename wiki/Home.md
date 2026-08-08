# 📖 Welcome to the `ebook-rs` Wiki

`ebook-rs` (v0.12.0) is a high-performance, 100% pure Rust eBook parser, renderer, and indexer supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT**, **TXT**, and **MD** formats with native AI RAG chunking, C/Python FFI bindings, and Web Component support.

---

## 🧭 Complete Wiki Sitemap

1. **[Getting Started](Getting-Started)**
   - Installation & Cargo Setup
   - Quick Start Code Examples
   - Auto-Detecting eBook Formats
2. **[Multi-Format Architecture](Multi-Format-Architecture)**
   - How Format Parsing Works
   - EPUB Archive vs Binary Containers (PDB/FB2/LIT)
   - Asset Inlining & Data URIs
3. **[CFI and Locations Engine](CFI-and-Locations)**
   - EPUB Canonical Fragment Identifiers (CFI)
   - Range Parsing, Comparing, and Character Offsets
   - Discrete Location Chunk Indexing
4. **[WASM and Server Integration](WASM-and-Server-Integration)**
   - WebAssembly Browser Bindings & `<ebook-reader>` Web Component
   - Built-in Multithreaded HTTP Reader Server
5. **[API Reference & `epub.js` Parity](API-Reference)**
   - Native AI & RAG Document Chunking Engine (`RagChunk`, `RagChunkConfig`)
   - C / Python / Multi-Language FFI Bindings (`ebook_rs::ffi`)
   - RTL & CJK Vertical Layout Engine (`WritingMode`)

---

## ⚡ Performance Highlights
- **Native AI & RAG Chunking**: Splits books into semantic vector DB chunks with CFI citation anchors.
- **C & Multi-Language FFI**: C-compatible ABI for Python (`pyo3`), Node.js (`ffi-napi`), C/C++, Swift, and Kotlin.
- **Zero-Allocation Full-Text Search**: Pre-computed search engine finding text matches in < 1ms.
- **Pure Rust 2024 Edition**: No C bindings or external dynamic library dependencies.
