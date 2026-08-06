# 📜 Changelog

All notable changes to `ebook-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.5.1] - 2026-08-06

### Fixed
- **B1 & B2**: Fixed multibyte UTF-8 string slicing and char conversion in `sanitize_html_scripts`, preventing CJK, Arabic, Hebrew, and Emoji text corruption or `byte index X is not a char boundary` panics.
- **B3**: Fixed tag skipping in `extract_plain_text` to recover text when unclosed `<style>` or `<script>` tags occur in non-conforming eBooks.
- **B4 & E3**: Fixed MOBI header EXTH tag parsing for language and text direction (`Rtl` vs `Ltr`); fixed `text_record_count == 0` fallback to avoid reading image or metadata PDB records as raw text.
- **B5**: Fixed `spine_index()` in `Cfi` to return `try_spine_index() -> Result<usize, String>`, returning an explicit error if no indirection step `!` is present instead of silently defaulting to section 0.
- **E1**: Fixed preservation of multiple `<dc:title>` tags (e.g. main title + subtitle concatenation).
- **E4 & E5**: Isolated attribute boundary matching in `regex_find_attr` so `src="..."` replacements do not overwrite `data-src="..."` or `srcset="..."`; added case-insensitive `.CSS`, `.css?v=1`, and `rel="stylesheet"` inlining.
- **E6 & E7**: Fixed section path normalization in `get_section_by_href` to avoid false suffix matches; added manifest `cover_id` fallback in `cover_image()` when `cover_href` is `None`.
- **E8**: Added actionable error guard detecting DRM-protected eBooks (ADEPT / LCP) informing users that decryption keys are required.

---

## [0.5.0] - 2026-08-06

### Added
- **Rayon Multi-Threaded Parallel Chapter Parser (`parallel` feature)**: Added Rayon parallel iteration support for concurrent chapter XML parsing and asset processing across all CPU cores.
- **Deterministic Reflow Paginator (`ReflowPaginator`)**: Added pure-Rust, DOM-free virtual reflow pagination calculating line wraps, page break boundaries, and character intervals without browser layout reflow.
- **Structural NLP Reading Analytics Engine (`ReadingAnalytics`)**: Added chapter word counting, WPM reading time estimation, TF-IDF top keyword extraction, and Flesch-Kincaid style difficulty scoring.
- **Remote ZIP Central Directory Streamer (`ZipHeaderReader`)**: Added EOCD `PK\x05\x06` parser generating HTTP Range requests (`bytes=start-end`) to open remote ZIP/CBZ archives by downloading < 1% of the total file payload.
- **New WASM JSON Bindings**: Exposed `get_section_analytics_json` and `paginate_section_json` in `WasmBook`.

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

---

## [0.2.0] - 2026-08-04

### Added
- **Multi-Format eBook Support**: Added native parsing for **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, and **LIT (Microsoft Reader)** formats.
- **Font De-Obfuscation**: Added SHA-1 / XOR de-scrambling for IDPF 2008 and Adobe font obfuscation specs.
- **EPUB 3 Landmarks & Page List**: Added `landmarks()` and `page_list()` navigation parsers.
- **Pre-Display Transformation Hooks**: Added `register_before_display_hook` pipeline for modifying chapter HTML before rendering.

---

## [0.1.0] - 2026-08-01

### Added
- Core EPUB 2 and EPUB 3 parser engine, CFI Engine, Location Progress Indexer, Full-Text Search Engine, Annotations Manager, HTTP Reader Server, and WebAssembly bindings.
