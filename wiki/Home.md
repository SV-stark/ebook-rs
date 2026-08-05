# 📖 Welcome to the `ebook-rs` Wiki

`ebook-rs` (v0.2.0) is a high-performance, 100% pure Rust eBook parser, renderer, and indexer for **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, and **LIT (Microsoft Reader)** formats.

---

## 🧭 Navigation Sitemap

1. [Getting Started](Getting-Started)
   - Installation & Cargo Setup
   - Quick Start Code Examples
   - Auto-Detecting Formats
2. [Multi-Format Architecture](Multi-Format-Architecture)
   - How Format Parsing Works
   - EPUB Archive vs Binary Containers (PDB/FB2/LIT)
   - Asset Inlining & Data URIs
3. [EPUB Canonical Fragment Identifiers (CFI)](CFI-and-Locations)
   - CFI Range Parsing & Compare
   - DOM Element IDs & Character Offsets
   - Discrete Location Chunk Indexing
4. [WASM & Reader Server Integration](WASM-and-Server-Integration)
   - WebAssembly Browser Bindings (`wasm-bindgen`)
   - Built-in HTTP Reader Server
5. [API Reference](API-Reference)
   - Core API Cheat Sheet
   - Structs & Method Signatures

---

## ⚡ Key Highlights
- **Zero-Allocation Full-Text Search**: Pre-computed search engine finding text matches in < 1ms.
- **Pure Rust 2024 Edition**: No C bindings or external dynamic library dependencies.
- **WASM & Native Reader Ready**: Integrates seamlessly with browser web apps, desktop apps, and servers.
