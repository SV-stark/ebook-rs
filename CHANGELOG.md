# 📜 Changelog

All notable changes to `ebook-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.9.0] - 2026-08-06

### Added
- **Synthetic FXL 2-Page Spreads (`SyntheticSpread`)**: Auto-synthesizes responsive side-by-side two-page spread containers (`book.get_synthetic_spread(left_idx, Some(right_idx))`) for EPUB 3 Fixed-Layout and comic book readers.
- **Table of Contents Deep Search & Flattening (`NavPoint::search`, `NavPoint::flatten`)**: Deep case-insensitive TOC searching across any hierarchy level with breadcrumb trails (`book.search_toc("query")`) and TOC tree flattening (`book.flatten_toc()`).

---

## [0.8.3] - 2026-08-06

### Fixed & Improved
- **MOBI Control Character Sanitization (`sanitize_mobi_control_chars`)**: Filtered out raw NUL (`\0`) bytes and non-printable control characters from PalmDOC streams that produced strange characters in MOBI output.
- **MOBI Image Extraction & Base64 Inlining (`process_mobi_images`)**: Implemented parsing of MOBI PDB image records from `first_image_index` and automated inlining into Base64 Data URIs (`data:image/jpeg;base64,...`) for `<img recindex="...">`, `src="kindle:embed:..."`, and `src="0000X.jpg"` image tags.

---

## [0.8.2] - 2026-08-06

### Fixed
- **Quote-Aware HTML Tag & Text Parsing (`find_tag_end`)**: Fixed a flaw where `>` characters inside tag attribute quotes (e.g. `alt="x > y"`, `title="A > B"`) prematurely split HTML tags and emitted stray `>` characters into body text.

---

## [0.8.0] - 2026-08-06

### Added
- **Tree-sitter Syntax Engine (`TreeSitterEngine`)**: Parses concrete syntax trees (`SyntaxNodeInfo`), extracts code blocks (`book.extract_code_blocks()`), and highlights embedded `<pre><code>` blocks across technical eBooks and Markdown/ODT sections.

---

## [0.7.1] - 2026-08-06

### Added
- Synchronized documentation, GitHub Pages site, and GitHub Wiki across all 11 format parsers (EPUB 2/3, MOBI, AZW3, FB2, KEPUB, LIT, CBZ, PDF, ODT, TXT, MD) and Readium APIs.

---

## [0.7.0] - 2026-08-06

### Added
- **OpenDocument Text (.odt) Parser**: Native parsing for `.odt` archives (`OdtBook`), extracting `content.xml` headings/paragraphs and `meta.xml` metadata.
- **Plain Text (.txt) & Markdown (.md) Parsers**: Native parsing for `.txt` and `.md` files (`TxtBook`), mapping Markdown headings to `NavPoint` TOC nodes and section breaks.
- **Regex Full-Text Search**: Full regular expression pattern search across chapter sections with `<mark>` context highlighting (`book.search_regex()`).
- **EPUB Structural Validator**: Structural diagnostics engine (`EpubValidator` / `book.validate()`), returning `ValidationReport` with severity levels (`Error`, `Warning`, `Info`).
- **Content Fingerprinting & Deduplication**: Metadata-independent content hashing (`BookFingerprint` / `book.fingerprint()`), calculating similarity scores (`match_score()`) to detect duplicate books across formats.
- **Academic Citation Exporter**: Format eBook metadata into standard academic citations (**BibTeX**, **APA 7th**, **MLA 9th**, **Chicago 17th**) via `book.to_bibtex()`, `book.to_apa()`, `book.to_mla()`, and `book.to_chicago()`.

---

## [0.6.1] - 2026-08-06

### Added
- **Readium LCP DRM Parsing**: Parse `META-INF/license.lcpl` into `LcpLicense`, check expiry, and decrypt AES-256 encrypted content via `LcpDecryptor`.
- **Readium Unified Locator Model**: Generate W3C/Readium-compliant `ReadiumLocator` JSON structures (CFI + position + section/total progression) via `book.to_readium_locator(spine_idx, char_offset)`.
- **Readium Search Web Service API**: Format full-text `SearchResult` into Readium standard `application/vnd.readium.search+json` schema via `SearchEngine::to_readium_search_json(&results, query)`.

---

## [0.6.0] - 2026-08-06

### Added
- **PDF Text Extraction Support (`pdf` feature)**: Integrated `pdf_oxide` for opening pre-OCR PDF documents (`PdfBook`), converting pages into Markdown/HTML sections.
- **EPUB 3 Accessibility Metadata (a11y)**: Parsed W3C EPUB Accessibility 1.1 and Schema.org metadata into `AccessibilityMetadata` (`schema:accessMode`, `schema:accessibilityFeature`, `a11y:certifiedBy`).
- **EPUB 3 Media Overlays (SMIL Sync)**: Native SMIL 3.0 audio-text synchronization engine (`MediaOverlayPackage`), supporting reverse timestamp lookup (`find_text_ref_by_timestamp()`), text href lookup (`find_audio_clip_by_text_href()`), and SMIL JSON export.

---

## [0.5.3] - 2026-08-06

### Fixed
- **B2 (Non-ASCII Attribute Extraction)**: Added character boundary safety checks in `extract_attr` (`tag_str.is_char_boundary`), preventing string slice panics when parsing non-ASCII attributes (`alt="über"`, `title="日本語"`) and CJK image filenames.
- **B6 (RAR v4 & RAR v5 CBR Detection)**: Updated CBR file signature detection to explicitly match both RAR v4 (`Rar!\x1a\x07\x00`) and RAR v5 (`Rar!\x1a\x07\x01\x00`) magic headers, returning actionable error messages instructing users to convert CBR to CBZ format.

---

## [0.5.2] - 2026-08-06

### Added
- **F8: Search Surrounding Context Snippets (`SearchSnippet`)**: Enhanced `SearchResult` with UTF-8 char-safe context snippets, character offset bounds, and `<mark>` query highlights.
- **F1: Deep DOM-Element CFI Resolver (`resolve_dom_path`)**: Added `CfiDomTarget` and `cfi.resolve_dom_path(html)` to resolve IDPF element steps (`/4/2/1`) and IDs (`[chap01]`) to target element IDs and CSS selector paths.
- **F10: W3C Web Annotation Data Model Export (`to_w3c_json`)**: Added standard W3C JSON-LD export (`http://www.w3.org/ns/anno.jsonld`) for annotations and highlights.
- **F6: Automatic RTL Render-Time Injection (`dir="rtl"`)**: Added automatic `dir="rtl"` and `text-align: right;` injection for RTL books (Arabic, Hebrew, Persian, Urdu).
- **F5: Reader Custom Font Injection (`set_custom_font`)**: Added `custom_font_family` and `custom_font_url` fields in `RenditionLayout` for `@font-face` CSS injection.
- **New WASM Bindings**: Exposed `resolve_cfi_dom_json` and `set_custom_font` to WebAssembly JS exports.

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
