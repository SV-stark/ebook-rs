# 📖 EBook-RS Developer Wiki (v0.12.0)

Welcome to the official developer wiki for **`ebook-rs`**.

---

## 📚 Quick Links

- [Complete API Reference Guide](docs/API.md)
- [Project Changelog](CHANGELOG.md)
- [GitHub Repository](https://github.com/SV-stark/ebook-rs)
- [Crates.io Package](https://crates.io/crates/ebook-rs)

---

## ⚡ Overview & Key Architecture

`ebook-rs` (v0.12.0) is designed for:
1. **Extreme Performance**: 100% pure Rust accelerated with SIMD UTF-8 (`simdutf8`), SIMD string searching (`memchr`), SIMD zlib decompression (`zlib-rs`), AHash maps (`ahash`), SSO stack-allocated strings (`compact_str`), and 1-byte mutexes (`parking_lot`).
2. **Multi-Format Parity**: EPUB 2, EPUB 3, MOBI (PalmDOC LZ77), AZW3 (KF8), FB2, KEPUB, LIT, CBZ (Comic Book ZIP), PDF, ODT, TXT, and MD.
3. **Native AI & RAG Document Chunking**: `book.to_rag_chunks()` splits books into semantic chunks with Markdown heading hierarchy, token estimations, and exact `epubcfi` citation anchors for Vector DBs.
4. **C & Multi-Language FFI Bindings**: C-compatible ABI (`#[unsafe(no_mangle)] extern "C"`) functions for Python, Node.js, C/C++, Swift (iOS), Kotlin (Android), and Go.
5. **Zero-Dependency `<ebook-reader>` Web Component**: HTML Custom Element JS generator (`get_custom_element_js()`) for WASM applications.
6. **CJK Vertical & RTL Layout Engine**: Support for Arabic/Hebrew (`direction: rtl`) and Japanese/Chinese/Korean vertical writing (`writing-mode: vertical-rl`).
7. **Universal Converter Engine**: Native EPUB 3 Exporter (`export_epub3_bytes()`) converting any format to EPUB 3 ZIP archives.
8. **Universal Portability**: Runs natively in CLI/HTTP applications, desktop GUIs, or inside web browsers via WebAssembly (`wasm-bindgen`).
