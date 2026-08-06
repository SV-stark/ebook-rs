# 📖 EBook-RS Developer Wiki

Welcome to the official developer wiki for **`ebook-rs`**.

---

## 📚 Quick Links

- [Complete API Reference Guide](docs/API.md)
- [Project Changelog](CHANGELOG.md)
- [GitHub Repository](https://github.com/SV-stark/ebook-rs)
- [Crates.io Package](https://crates.io/crates/ebook-rs)

---

## ⚡ Overview & Key Architecture

`ebook-rs` is designed for:
1. **High Performance**: 100% pure Rust, zero-allocation full-text search engine (0.59ms), zero-copy base64 comic viewports.
2. **Multi-Format Parity**: EPUB 2, EPUB 3, MOBI (PalmDOC LZ77), AZW3 (KF8), FB2, KEPUB, LIT, and CBZ (Comic Book ZIP).
3. **Security & Sandboxing**: Default script sanitization (`strip_script_content()`), removing `<script>` blocks and inline event attributes.
4. **Memory Efficiency**: Resource Streaming API (`AssetDeliveryStrategy::ResourceStream`) to eliminate Base64 inflation on 100MB+ books.
5. **Universal Portability**: Runs natively in CLI/HTTP applications, desktop GUIs, or inside web browsers via WebAssembly (`wasm-bindgen`).
