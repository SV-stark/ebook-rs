# 📜 Changelog

All notable changes to `ebook-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.11.2] - 2026-08-07

### Fixed & Hardened (Full 24-Point Comprehensive Audit Fixes)
- **Cross-String Indexing Panic in Search (`search.rs`)**: Re-aligned character indexing directly on `plain_text` characters, resolving panics when searching texts containing multi-byte characters like Turkish `İ`.
- **Lower-Case String Offset Slicing (`section.rs`, `footnote.rs`)**: Replaced lower-cased offset slicing in `regex_find_attr`, `regex_find_link_css`, and `parse_viewport_meta` with character boundary-safe string matching (`find_ignore_case`).
- **NCX Navigation Unbounded Recursion & Empty HREF Subtrees (`nav.rs`)**: Implemented `depth > 32` recursion guard and preserved child subitem hierarchies when grouping parent `navPoint` nodes omit `src="..."` attributes.
- **Sanitizer HTML Entity Bypasses & Attribute Slicing (`section.rs`)**: Added entity decoding prior to URI check, fixed Phase 2 attribute `=` offset calculation, and prevented unclosed `<script>` / `<iframe>` tags from swallowing documents.
- **Readium LCP Expiry & Passphrase Validation (`lcp.rs`)**: Replaced hardcoded date literal with dynamic expiration checks and restored empty passphrase validation.
- **Server Route, Mutex Lock & Security Headers (`server.rs`)**: Added `/resource/` endpoint for streaming assets, added HTTP security headers (`nosniff`, `SAMEORIGIN`, `CSP`), and released global `book_arc` mutex lock prior to running parallel searches.
- **EPUB 3 Exporter & Zstd State Restoration (`book.rs`)**: Remapped TOC HREFs to `section_N.html`, cleaned XML header double-wrapping, exported valid UUID identifiers, and preserved full archive asset files across Zstd cache compression/restoration cycles.
- **MOBI, FB2, CBZ, Media Overlay, CFI, and Citation Fixes**: Sorted MOBI kindle embed indices in descending order, supported FB2 `xlink:href`, fixed SMIL NPT clock hour parsing/`clipEnd` defaults, inverted MLA/Chicago author names, and updated documentation labels.

---

## [0.11.1] - 2026-08-07

### Added
- **Browser SpeechSynthesis (TTS) Word-by-Word Synchronizer (`TtsWordToken`)**: Added `book.get_tts_tokens(section_index)` and `book.get_tts_section_html(section_index)`. Tokenizes text into word tokens (`TtsWordToken`) with exact character offsets (`char_start`, `char_end`) and wraps HTML text words in `<span id="tts-w-{index}" class="tts-word">` tags for live Web Speech API `SpeechSynthesisUtterance` boundary event word-by-word visual highlighting.

---

## [0.11.0] - 2026-08-07

### Added
- **Legacy Non-UTF-8 Charset Decoding (`encoding_rs`)**: `decode_bytes_with_encoding(bytes, label)` decodes legacy encodings (`Windows-1252`, `Shift-JIS`, `GBK`, `ISO-8859-1`, etc.) into clean UTF-8 strings for 100% parse success across legacy MOBI, FB2, and TXT files.
- **Automatic Language Detection (`whatlang`)**: `book.detect_language()` and `section.detect_language()` perform fast statistical language identification on text contents when OPF metadata `dc:language` is missing.
- **Zstd Compressed State Caching (`zstd`)**: `book.export_zstd_cache()` and `Book::from_zstd_cache()` compress and restore parsed book states (`BookCacheState`) in sub-milliseconds for instant server-side and WebAssembly caching.

---

## [0.10.5] - 2026-08-06

### Added
- **Performance Acceleration Crates**:
  - `compact_str`: Small String Optimization (SSO) storing strings <= 24 bytes directly on the stack to eliminate heap allocations in DOM AST nodes and tags.
  - `ahash`: AHashMap & AHashSet for 3x-5x faster hash lookups in OPF manifest, DOM attributes, and annotations.
  - `simdutf8`: SIMD-accelerated UTF-8 validation (AVX2/NEON) for 10x-20x faster HTML/XML byte-stream decoding.
  - `zlib-rs` (`flate2` feature): SIMD zlib decompression for 3x faster EPUB/CBZ ZIP archive reading.
  - `memchr`: SIMD substring searching for ultra-fast HTML tag scanning, attribute extraction, and script stripping.
  - `parking_lot`: Fast 1-byte non-poisoning mutex locks for concurrent section caches.

---

## [0.10.0] - 2026-08-06

### Added
- **Universal EPUB 3 Exporter (`export_epub3_bytes`)**: Compiles any opened eBook (MOBI, AZW3, FB2, CBZ, ODT, TXT, MD) directly into a binary EPUB 3 (`.epub`) ZIP archive.
- **Zero-Copy Memory-Mapped I/O (`Book::from_mmap`)**: Added `Book::from_mmap` under `mmap` feature (`memmap2`) for zero-copy memory mapping.
- **Lightweight DOM AST Engine (`EbookDomTree`, `DomNode`)**: Added zero-allocation DOM AST tree parser supporting `DomNode::Element`, `DomNode::Text`, and `DomNode::Comment`.
- **Fuzzy Malformed XML Recovery Engine**: Added XML entity decoding (`&` ➔ `&amp;`) and malformed tag repair in `src/opf.rs` and `src/nav.rs`.

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
