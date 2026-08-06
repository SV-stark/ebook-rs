# 📜 Changelog

All notable changes to `ebook-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.4.0] - 2026-08-06

### Added
- **Footnote & Endnote Popup Previewer**: Added `Footnote` model and `section.extract_footnotes()` for instant popup previews of `epub:type="noteref"` and `<aside>` elements.
- **OPDS Catalog Feed Client (`opds` feature)**: Added OPDS 1.2 (Atom XML) and OPDS 2.0 (JSON) catalog feed parsers (`OpdsFeed`, `OpdsEntry`, `OpdsLink`) for browsing online library catalogs (Standard Ebooks, Gutenberg).
- **Unpacked Remote HTTP Range Loader**: Added `HttpRangeRequest` helper for parsing and generating HTTP `Range` headers (`bytes=start-end`) for remote byte-slice streaming.
- **Asset Delivery Strategy & Resource Streaming**: Added `AssetDeliveryStrategy` enum (`InlinedBase64` vs `ResourceStream`) to `RenditionLayout`, eliminating 33% Base64 size inflation on 100MB+ image-heavy books and comics.
- **Raw Resource Resolver API**: Added `book.get_resource_bytes("path")` and WASM export for browser `URL.createObjectURL(blob)` creation.
- **Embedded Script Sanitizer & Sandboxing**: Added `strip_script_content()` and `allow_scripted_content` toggle in `RenditionLayout` for protection against XSS in untrusted eBooks.
- **Fixed Layout (FXL) Viewport Scaling**: Added `<meta name="viewport">` parsing and `compute_fxl_scale` matrix calculation for fixed-layout rendering.
- **Continuous Viewport Manager Config**: Added `ViewportManagerConfig` for off-screen section preloading and intersection observer continuous scrolling.

### Changed
- Upgraded `zip` crate dependency to `v8.6.0` (`zip = "8.6"`).
- Updated package manifest to exclude `samples/*` from crates.io package payload (reduced crate size from 42.6MB to 63.2KiB).
- Updated copyright ownership to `SV-Stark`.

---

## [0.3.1] - 2026-08-05

### Changed
- Upgraded underlying zip decompressor to `zip = "8.6"`.
- Added `exclude = ["samples/*"]` to `Cargo.toml` to optimize crates.io payload size.

---

## [0.3.0] - 2026-08-05

### Added
- **CBZ (Comic Book Archive) Support**: Added native parsing for `.cbz` (ZIP image archive) comic books with zero-copy Base64 image viewports and natural page sorting.
- **CBR Format Auto-Detection Guard**: Added pure-Rust RAR magic header check (`Rar!\x1a\x07`) returning actionable error messages directing users to convert CBR to CBZ to maintain 100% pure Rust licensing.

### Fixed
- Fixed format auto-detection bug in `Book::from_bytes_with_title` where non-EPUB ZIP archives failed on missing `META-INF/container.xml`.

---

## [0.2.5] - 2026-08-05

### Added
- **Readium Webpub Manifest Export**: Implemented `to_webpub_manifest()` for Readium `application/webpub+json` interop.

### Fixed
- Fixed PalmDOC LZ77 distance fallback bounds.
- Fixed FB2 image reference matching.
- Fixed complex CFI range assertion parsing safety.

---

## [0.2.0] - 2026-08-04

### Added
- **Multi-Format eBook Support**: Added native parsing for **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, and **LIT (Microsoft Reader)** formats.
- **Font De-Obfuscation**: Added SHA-1 / XOR de-scrambling for IDPF 2008 (`http://www.idpf.org/2008/embedding`) and Adobe font obfuscation specs.
- **EPUB 3 Landmarks & Page List**: Added `landmarks()` and `page_list()` navigation parsers.
- **Pre-Display Transformation Hooks**: Added `register_before_display_hook` pipeline for modifying chapter HTML before rendering.

---

## [0.1.0] - 2026-08-01

### Added
- Core EPUB 2 and EPUB 3 parser engine (OPF package, manifest, spine, NCX, NAV XHTML).
- Canonical Fragment Identifier (CFI) Engine (parsing, formatting, range CFI, 0-alloc sorting).
- Locations & Progress Indexer (character offset discrete location chunks).
- Full-Text Search Engine (0.59ms / 0-allocation string search).
- Annotations Manager (Highlights, Notes, Underlines, Bookmark serialization).
- Built-in HTTP Reader Server (`tiny_http`) and WebAssembly bindings (`wasm-bindgen`).
