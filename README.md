# 📖 EBook-RS: Multi-Format Rust EBook Parser and Reader Engine (v0.11.6)

`ebook-rs` is a high-performance, 100% pure Rust parser and reader engine for **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT**, **TXT**, and **MD** formats, designed for full feature parity with **epub.js** and **foliate-js**.

---

## ⚡ Feature Parity Matrix

### 📂 Format Support

| Feature | 🚀 `ebook-rs` (v0.11.5) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Full OPF + NCX/NAV | ✅ Yes | ✅ Yes | ✅ Yes |
| **EPUB 3 Fixed-Layout (FXL)** | ✅ 2-page spread renderer | ✅ Yes | ✅ Yes | ❌ No |
| **MOBI & AZW3 (KF8) Support** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2) Support** | ✅ Native XML + xlink:href | ❌ No | ✅ Yes | ❌ No |
| **KEPUB (Kobo EPUB) Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **LIT (Microsoft Reader) Support** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **CBZ (Comic Book ZIP) Support** | ✅ Native ZIP Images | ❌ No | ✅ Yes | ❌ No |
| **PDF Support** | ✅ `pdf_oxide` parser | ❌ No | ❌ No | ❌ No |
| **ODT (OpenDocument Text)** | ✅ `office_oxide` | ❌ No | ❌ No | ❌ No |
| **TXT / Markdown Support** | ✅ Auto-reflow sections | ❌ No | ❌ No | ❌ No |
| **Auto Format Detection** | ✅ Magic-byte detection | ❌ No | ❌ No | ❌ No |

### 🧭 Navigation & Rendering

| Feature | 🚀 `ebook-rs` (v0.11.5) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **IDPF CFI Engine** | ✅ Parse / format / compare / range | ✅ Yes | ✅ Yes | ✅ Yes |
| **CFI DOM Resolver** | ✅ `cfi.resolve_dom_path(html)` | ✅ Yes | ✅ Yes | ❌ No |
| **Readium CFI Unified Locator** | ✅ Full model | ✅ Yes | ✅ Yes | ❌ No |
| **Location / Reading Progress** | ✅ `locations_from_sections()` | ✅ Yes | ✅ Yes | ✅ Yes |
| **SMIL Media Overlays (Sync)** | ✅ NPT clock parser | ✅ Yes | ✅ Yes | ❌ No |
| **EPUB NCX / NAV TOC Parsing** | ✅ Deep recursive nav tree | ✅ Yes | ✅ Yes | ✅ Yes |
| **RTL Text Auto-Injection** | ✅ `dir="rtl"` + `text-align:right` | ✅ Yes | ✅ Yes | ❌ No |
| **Viewport Meta Parsing** | ✅ Zero-alloc slice parse | ✅ Yes | ✅ Yes | ❌ No |
| **Reflow Paginator** | ✅ `ReflowPaginator::paginate_section` | ✅ Yes | ✅ Yes | ❌ No |
| **Custom Font Injection** | ✅ `@font-face` CSS injection | ✅ Yes | ✅ Yes | ❌ No |

### 🔍 Search & Analytics

| Feature | 🚀 `ebook-rs` (v0.11.5) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **Full-Text Search** | ✅ SIMD-accelerated | ❌ No | ✅ Basic | ❌ No |
| **Regex Search** | ✅ `regex_search` | ❌ No | ❌ No | ❌ No |
| **Search Context Snippets** | ✅ `<mark>` highlights + offsets | ❌ No | ✅ Yes | ❌ No |
| **Readium Search JSON Export** | ✅ Readium-compliant JSON | ❌ No | ✅ Yes | ❌ No |
| **NLP Reading Analytics** | ✅ Word count / reading time / complexity | ❌ No | ❌ No | ❌ No |
| **Auto Language Detection** | ✅ `detect_language` (`whatlang`) | ❌ No | ❌ No | ❌ No |

### 📝 Annotations & Standards

| Feature | 🚀 `ebook-rs` (v0.11.5) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **W3C Web Annotation (JSON-LD)** | ✅ Full CRUD + `to_w3c_json` | ❌ No | ✅ Yes | ❌ No |
| **Readium WebPub Manifest Export** | ✅ `book.to_webpub_manifest()` | ❌ No | ✅ Yes | ❌ No |
| **OPDS 1.2 Atom XML Catalog** | ✅ `OpdsFeed::parse_atom_xml` | ❌ No | ❌ No | ❌ No |
| **OPDS 2.0 JSON Catalog** | ✅ `OpdsFeed::parse_json` | ❌ No | ❌ No | ❌ No |
| **Readium LCP DRM Detection** | ✅ License parse + expiry | ❌ No | ✅ Yes | ❌ No |
| **MLA / Chicago Citations** | ✅ `book.to_mla_citation()` | ❌ No | ❌ No | ❌ No |

### ⚙️ Performance & Architecture

| Feature | 🚀 `ebook-rs` (v0.11.6) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **Zero-Copy Memory-Mapped I/O** | ✅ `Book::from_mmap` | ❌ No | ❌ No | ❌ No |
| **Lazy On-Demand Section Loading** | ✅ `load_section_lazy(i)` | ❌ No | ✅ Yes | ❌ No |
| **Async tokio API** | ✅ `book::async_api` feature | ❌ No | ❌ No | ❌ No |
| **Zstd Compressed State Cache** | ✅ `export_zstd_cache` | ❌ No | ❌ No | ❌ No |
| **O(1) Archive Asset Lookup** | ✅ Dual-HashMap ZIP index | ❌ No | ❌ No | ❌ No |
| **SIMD UTF-8 Validation** | ✅ `simdutf8` (AVX2/NEON) | ❌ No | ❌ No | ❌ No |
| **SIMD Substring Search** | ✅ `memchr` SIMD scanning | ❌ No | ❌ No | ❌ No |
| **AHash Fast Hash Maps** | ✅ 3–5× faster lookups | ❌ No | ❌ No | ❌ No |
| **Single-Pass CSS Inliner** | ✅ O(N) streaming builder | ❌ No | ❌ No | ❌ No |
| **CSS Versioned Href Fix** | ✅ `style.css?v=2` inlined | ❌ No | ❌ No | ❌ No |
| **Async Non-Blocking Web Font** | ✅ `media="print" onload` | ✅ Yes | ✅ Yes | ❌ No |
| **Built-in Localhost Reader** | ✅ `tiny_http` web server | ✅ Yes | ✅ Yes | ❌ No |

### 🛡️ Security & Robustness

| Feature | 🚀 `ebook-rs` (v0.11.6) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **Script Sanitizer (XSS Guard)** | ✅ 3-phase + entity decode | ✅ Partial | ✅ Partial | ❌ No |
| **Fuzzy Malformed XML Recovery** | ✅ Entity repair + tag heal | ❌ No | ❌ No | ❌ No |
| **Legacy Charset Decoding** | ✅ `decode_bytes_with_encoding` (encoding_rs) | ❌ No | ❌ No | ❌ No |
| **LCP AES-256-CBC Decryption** | ✅ Real crypto via `aes`+`cbc` | ❌ No | ✅ Partial | ❌ No |
| **DRM-Protected Book Detection** | ✅ ADEPT + LCP detection | ❌ No | ✅ Yes | ❌ No |
| **HTTP Security Headers** | ✅ CSP / nosniff / SAMEORIGIN | ❌ No | ❌ No | ❌ No |
| **CBR RAR Format Error Guard** | ✅ RAR v4 + v5 magic detection | ❌ No | ❌ No | ❌ No |
| **libFuzzer Fuzz Harnesses** | ✅ `fuzz_from_bytes` + `fuzz_cfi_parse` | ❌ No | ❌ No | ❌ No |

### 🧩 Advanced & Developer Features

| Feature | 🚀 `ebook-rs` (v0.11.6) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **Universal EPUB 3 Exporter** | ✅ Any format → EPUB3 bytes | ❌ No | ❌ No | ❌ No |
| **Book Fingerprint / Identity** | ✅ SHA-based fingerprint | ❌ No | ❌ No | ❌ No |
| **SpeechSynthesis TTS Tokens** | ✅ `get_tts_tokens` with char offsets | ❌ No | ✅ Yes | ❌ No |
| **TTS Annotated HTML** | ✅ `<span class="tts-word">` injection | ❌ No | ✅ Yes | ❌ No |
| **TreeSitter Code Highlighter** | ✅ AST extraction | ❌ No | ❌ No | ❌ No |
| **Lightweight DOM AST Engine** | ✅ `EbookDomTree` / `DomNode` | ❌ No | ❌ No | ❌ No |
| **EPUB Structural Validator** | ✅ OPF + Spine + Manifest checks | ❌ No | ❌ No | ❌ No |
| **Optional `serde` Feature** | ✅ Compile without serde overhead | ❌ N/A | ❌ N/A | ❌ No |
| **WASM-Ready (no_std compat)** | ✅ WASM build support | ✅ Yes | ✅ Yes | ❌ No |

---

## 🆕 What's New in v0.11.6

- **Lazy On-Demand Section Loading** — `book.load_section_lazy(i)` parses one section directly from the archive without caching all sections in RAM — essential for large books.
- **Real AES-256-CBC LCP Decryption** — `LcpDecryptor` now uses proper `aes` + `cbc` + `cipher` crates (key = `SHA-256(passphrase)`, IV = first 16 bytes of ciphertext) per the Readium LCP spec.
- **Async API** — New `book::async_api` module with `from_file_async`, `from_bytes_async`, and `load_section_lazy_async` via tokio (enable with `features = ["async"]`).
- **`serde` Feature Flag** — `serde` / `serde_json` are now optional (default on). Opt out with `default-features = false` to slash compile time for pure parsing use-cases.
- **CSS Versioned Href Fix** — `style.css?v=2`, `style.css?cache=abc` now correctly resolve and inline in the archive.
- **cargo-fuzz Harnesses** — `fuzz_from_bytes` and `fuzz_cfi_parse` libFuzzer harnesses in `fuzz/`. Run with `cargo +nightly fuzz run fuzz_from_bytes`.
- **Dynamic LCP Expiry** — `LcpDecryptor` now evaluates license expiration against the real system clock instead of a hardcoded date.

## 🆕 What's New in v0.11.5

- **Full Blackbox Test Suite (38 tests, 4.4 s)** — Complete coverage across all features: OPDS, charset decoding, EPUB3 exporter roundtrip, reflow paginator, mmap, fuzzy XML, and TTS word synchronization.
- **O(1) Archive Lookups** — `Archive::contains` now uses dual-HashMap constant-time lookups instead of O(N) linear scan.
- **Zero-Alloc Viewport Parsing** — `parse_viewport_meta` rewritten with zero-copy byte slices.
- **Single-Pass CSS Inliner** — O(N²) `.replace()` loops replaced with streaming O(N) builder.
- **Zstd Cache Deduplication** — `export_zstd_cache` deduplicates identical `processed_html` / `raw_html` entries, reducing cache size by up to 50%.
- **Non-Blocking Font Loading** — Localhost reader now uses async `media="print" onload` font loading with instant system font fallbacks, eliminating render-blocking delays.

---

## 🚀 Quick Start Example

```rust
use ebook_rs::{Book, ReflowPaginator, Cfi};

fn main() -> Result<(), String> {
    // Auto-detects and opens EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ, PDF, ODT, TXT, MD
    let mut book = Book::from_file("book.epub")?;

    println!("Title: {}", book.metadata().title);
    println!("Language: {:?}", book.detect_language());
    println!("Sections: {}", book.spine().len());

    // Export any format to standard binary EPUB3 ZIP bytes
    let epub3_bytes = book.export_epub3_bytes()?;

    // Zstd-compress parsed state for instant restoration
    let cache = book.export_zstd_cache()?;
    let restored = Book::from_zstd_cache(&cache)?;

    // Search with surrounding context snippets and <mark> highlights
    let search_results = book.search("quantum");
    for res in search_results {
        println!("Snippet: {}", res.snippet);
    }

    // TTS word-by-word synchronization tokens
    let tts_tokens = book.get_tts_tokens(0);
    for token in tts_tokens {
        println!("[{}..{}] {}", token.char_start, token.char_end, token.word);
    }

    Ok(())
}
```

---

## 📖 Documentation & Wiki

Detailed API documentation and integration guides are available in [docs/API.md](docs/API.md) and [WIKI.md](WIKI.md).

---

## 📜 License

Licensed under the MIT License. Copyright (c) 2026 SV-Stark.
