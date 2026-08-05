# 📖 Welcome to the `ebook-rs` Wiki

`ebook-rs` (v0.2.0) is a high-performance, 100% pure Rust eBook parser, renderer, and indexer supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, and **LIT (Microsoft Reader)** formats.

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
   - WebAssembly Browser Bindings (`wasm-bindgen`)
   - Built-in Multithreaded HTTP Reader Server
5. **[API Reference & `epub.js` Parity](API-Reference)**
   - 🟢 **Section A: Drop-in Replacements for `epub.js` APIs**
   - 🔵 **Section B: Native `ebook-rs` Extensions Beyond `epub.js`**

---

## ⚡ Performance Highlights
- **Zero-Allocation Full-Text Search**: Pre-computed search engine finding text matches in < 1ms.
- **Pure Rust 2024 Edition**: No C bindings or external dynamic library dependencies.
- **WASM & Reader App Ready**: Integrates seamlessly with web applications, desktop engines, and HTTP reader servers.
