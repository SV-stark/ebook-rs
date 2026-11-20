---
title: Amazon KFX Exporter
description: Compile standard eBooks into full-fidelity Amazon Ion 1.0 KFX containers.
---

`UniversalKfxExporter` compiles any parsed `Book` into an Amazon Ion 1.0 `CONT` v2 KFX container (`YJ_symbols` v10, `ENTY` v1 framed entities, and `kfxgen` SHA-1 payload trailer) compatible with both `KfxBook` and Calibre's `KFX Input` plugin.

```rust
use ebook_rs::{Book, UniversalKfxExporter};

let book = Book::from_file("input.epub")?;
let kfx_bytes = UniversalKfxExporter::export(&book)?;
std::fs::write("output.kfx", kfx_bytes)?;
```

### Full-Fidelity Features

- **Deterministic Identifiers**: Computes unique SHA-1 publication digests for `ASIN`, `asset_id`, `book_id`, and `content_id`.
- **Covers & Images**: Preserves declared cover images (`cover_image` in `$490` `kindle_title_metadata`), non-spine manifest images, inline `<img>`, SVG `<image>`, and `data:` URIs as `$164` (`external_resource`) and `$417` (`bcRawMedia`) entities.
- **Embedded Publisher Fonts & `font-family` Cascade**: Serializes `.otf`, `.ttf`, `.woff`, and `.woff2` manifest fonts into `$262` (`font`) and `$418` (`bcRawFont`) entities using parsed `@font-face` CSS metadata, and propagates CSS `font-family` assignments (`body`, element, class, ID, inline `style`) into `$157` style entities (`$11` `font_family`) with inheritance down to paragraphs, headings, lists, table cells, and inline `$142` style events.
- **Hierarchical TOC & Hyperlinks**: Emits `$389` (`book_navigation`) with recursive `$393` (`nav_unit`) entries and `$266` (`anchor`) / `$142` (`style_events`) for internal and external links.
- **RTL & Bidi Direction**: Preserves spine `page-progression-direction="rtl"` (`$192` in `$538` `document_data`) plus document, block, and inline `<span dir>`, `<bdi>`, and `<bdo>` bidi overrides (`$682` `direction` / `$674` `unicode-bidi` in `$157` styles).
- **Structured Tables & Chunking**: Serializes `<table>` (`$278`/`$279`/`$269` with `colspan`/`rowspan`) and chunks `$145` content fragments below 8,192 bytes.
