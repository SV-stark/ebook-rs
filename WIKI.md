# 📖 EBook-RS Developer Wiki (v0.10.5)

Welcome to the official developer wiki for **`ebook-rs`**.

---

## 📚 Quick Links

- [Complete API Reference Guide](docs/API.md)
- [Project Changelog](CHANGELOG.md)
- [GitHub Repository](https://github.com/SV-stark/ebook-rs)
- [Crates.io Package](https://crates.io/crates/ebook-rs)

---

## ⚡ Overview & Key Architecture

`ebook-rs` (v0.10.5) is designed for:
1. **Extreme Performance**: 100% pure Rust accelerated with SIMD UTF-8 (`simdutf8`), SIMD string searching (`memchr`), SIMD zlib decompression (`zlib-rs`), AHash maps (`ahash`), SSO stack-allocated strings (`compact_str`), and 1-byte mutexes (`parking_lot`).
2. **Multi-Format Parity**: EPUB 2, EPUB 3, MOBI (PalmDOC LZ77), AZW3 (KF8), FB2, KEPUB, LIT, CBZ (Comic Book ZIP), PDF, ODT, TXT, and MD.
3. **Universal Converter Engine**: Native EPUB 3 Exporter (`export_epub3_bytes()`) converting any format to EPUB 3 ZIP archives.
4. **Zero-Copy Memory-Mapping**: Memory-mapped file loading (`Book::from_mmap`) for multi-hundred MB files.
5. **Lightweight DOM AST Tree**: `EbookDomTree` for zero-allocation HTML node parsing and manipulation.
6. **Fuzzy XML Recovery**: `sanitize_and_repair_xml` repairing unescaped entities (`&` ➔ `&amp;`) and malformed tags.
7. **Security & Sandboxing**: Default script sanitization (`strip_script_content()`), removing `<script>` blocks and inline event attributes.
8. **Universal Portability**: Runs natively in CLI/HTTP applications, desktop GUIs, or inside web browsers via WebAssembly (`wasm-bindgen`).
