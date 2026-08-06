# 📚 EBook-RS API Reference & Complete Documentation (v0.6.0)

`ebook-rs` (v0.6.0) is a multi-format pure Rust eBook engine supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, and **PDF** formats.

---

## 📋 Table of Contents
1. [Multi-Format Book Core API (`ebook_rs::Book`)](#1-multi-format-book-core-api-ebook_rsbook)
2. [Supported Formats Matrix](#2-supported-formats-matrix)
3. [Format-Specific Parsers (`MobiBook`, `Fb2Book`, `LitBook`, `CbzBook`)](#3-format-specific-parsers)
4. [EPUB 3 Accessibility Metadata (`AccessibilityMetadata`)](#4-epub-3-accessibility-metadata-accessibilitymetadata)
5. [EPUB 3 Media Overlays (SMIL Audio Sync) (`MediaOverlayPackage`)](#5-epub-3-media-overlays-smil-audio-sync-mediaoverlaypackage)
6. [Section Module (`ebook_rs::Section`)](#6-section-module-ebook_rssection)
7. [EPUB CFI Engine (`ebook_rs::Cfi`)](#7-epub-cfi-engine-ebook_rscfi)
8. [Locations Progress Engine (`ebook_rs::Locations`)](#8-locations-progress-engine-ebook_rslocations)
9. [Annotations Manager (`ebook_rs::AnnotationManager`)](#9-annotations-manager-ebook_rsannotationmanager)
10. [Full-Text Search Engine (`ebook_rs::SearchEngine`)](#10-full-text-search-engine-ebook_rssearchengine)
11. [Readium Webpub Manifest Export (`ebook_rs::webpub`)](#11-readium-webpub-manifest-export-ebook_rswebpub)
12. [OPDS Catalog Client & Feed Generator (`ebook_rs::opds`)](#12-opds-catalog-client--feed-generator-ebook_rsopds)
13. [HTTP Reader Server & Web UI (`ebook_rs::server`)](#13-http-reader-server--web-ui-ebook_rsserver)

---

## 1. Multi-Format Book Core API (`ebook_rs::Book`)

`Book` is the unified entry point struct for opening, inspecting, searching, and rendering eBooks across all formats.

### Opening eBooks (EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ)

```rust
use ebook_rs::Book;

// Auto-detects and loads EPUB, MOBI, AZW3, FB2, KEPUB, LIT, or CBZ from file path
let mut book = Book::from_file("sample.cbz")?;

// Load from in-memory byte slice (e.g. HTTP download or WASM Uint8Array)
let bytes: Vec<u8> = std::fs::read("sample.epub")?;
let mut book = Book::from_bytes(&bytes)?;
```

---

## 2. Supported Formats Matrix

| Extension | Format Name | Auto-Detected Header Magic | Inlining & Media |
|---|---|---|---|
| `.epub` | EPUB 2 & EPUB 3 | `PK\x03\x04` | Full Base64 Data URIs / Resource Stream |
| `.kepub` | Kobo EPUB | `PK\x03\x04` | KoboSpan DOM Aware |
| `.mobi` | Mobipocket / PalmDOC | `BOOKMOBI` / `TEXtREDR` | PalmDOC LZ77 Decompressor |
| `.azw3` | Kindle Format 8 (KF8) | `BOOKMOBI` PDB Header | HTML5 / CSS3 Extracted |
| `.fb2` | FictionBook 2 XML | `<FictionBook>` XML | Embedded Base64 Images |
| `.lit` | Microsoft Reader | `ITOLITLS` Header | Multi-section HTML Conversion |
| `.cbz` | Comic Book ZIP | `PK\x03\x04` Archive | Sequential Image Page HTML Reader |

---

## 3. Format-Specific Parsers

In addition to `Book::from_file()`, format-specific parsers are available:
- `ebook_rs::CbzBook::parse(bytes, title_fallback)` — Parses CBZ comic archives.
- `ebook_rs::MobiBook::parse(bytes)` — Parses MOBI / AZW3 PalmDOC PDB containers.
- `ebook_rs::Fb2Book::parse(bytes)` — Parses FictionBook 2 XML documents.
- `ebook_rs::LitBook::parse(bytes)` — Parses Microsoft Reader LIT files.

---

## 4. EPUB 3 Accessibility Metadata (`AccessibilityMetadata`)

`ebook-rs` parses W3C EPUB Accessibility 1.1 and Schema.org metadata into `book.metadata().accessibility`:

```rust
let a11y = &book.metadata().accessibility;

if a11y.is_accessible {
    println!("Access modes: {:?}", a11y.access_modes);
    println!("Features: {:?}", a11y.accessibility_features);
    println!("Summary: {:?}", a11y.accessibility_summary);
    println!("Certified by: {:?}", a11y.certified_by);

    assert!(a11y.has_alternative_text());
    assert!(a11y.has_structural_navigation());
    assert!(a11y.is_screen_reader_friendly());
}
```

---

## 5. EPUB 3 Media Overlays (SMIL Audio Sync) (`MediaOverlayPackage`)

`ebook-rs` provides full parsing and query support for EPUB 3 synchronized audio-text read-aloud overlays (`.smil`):

```rust
// Access parsed SMIL packages
for (smil_path, pkg) in &book.media_overlays {
    // Reverse lookup: Find text node active for a given audio timestamp (in seconds)
    if let Some(text_ref) = pkg.find_text_ref_by_timestamp("audio/ch1.mp3", 8.5) {
        println!("Active element ID: {:?}", text_ref.element_id);
    }

    // Forward lookup: Find audio clip start/end times for a clicked paragraph
    if let Some(audio_clip) = pkg.find_audio_clip_by_text_href("chapter1.xhtml#p2") {
        println!("Audio clip: {} -> {}s", audio_clip.clip_begin, audio_clip.clip_end);
    }

    // Export SMIL package as JSON
    let json_str = pkg.to_smil_json()?;
}
```
