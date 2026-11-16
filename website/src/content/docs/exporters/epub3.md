---
title: Universal EPUB 3 Exporter
description: High-performance multi-format conversion to standardized EPUB 3.
---

`UniversalEpub3Exporter` compiles any parsed `Book` (from MOBI, AZW3, KFX, FB2, LIT, DOCX, RTF, PDF, TXT, or Markdown) into a compliant **W3C EPUB 3.3** package.

```rust
use ebook_rs::{Book, UniversalEpub3Exporter};

let book = Book::from_file("input.mobi")?;
let epub_bytes = UniversalEpub3Exporter::export(&book)?;
std::fs::write("output.epub", epub_bytes)?;
```

### Structure & Conformance

1. Uncompressed `mimetype` header (`application/epub+zip`) per the EPUB OCF specification.
2. `META-INF/container.xml` pointing to `OEBPS/content.opf`.
3. `OEBPS/content.opf` manifest and spine with asset deduplication, required `<meta property="dcterms:modified">` UTC timestamp, and preserved `page-progression-direction="rtl"`.
4. `OEBPS/nav.xhtml` EPUB 3 navigation document (with collision guards for root `nav.xhtml` inputs).
5. Relocated XHTML spine documents (`OEBPS/sec_*.xhtml`) with rewritten relative `href`, `src`, `xlink:href`, and `poster` URLs plus embedded images and fonts.
