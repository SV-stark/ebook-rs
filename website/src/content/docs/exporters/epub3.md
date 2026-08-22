---
title: Universal EPUB 3 Exporter
description: High-performance multi-format conversion to standardized EPUB 3.
---

`UniversalEpub3Exporter` allows compiling any parsed `Book` (from MOBI, AZW3, KFX, FB2, LIT, DOCX, RTF, PDF) into an official **W3C EPUB 3.3** package.

```rust
use ebook_rs::{Book, UniversalEpub3Exporter};

let book = Book::from_file("input.mobi")?;
let epub_bytes = UniversalEpub3Exporter::export(&book)?;
std::fs::write("output.epub", epub_bytes)?;
```

### Structure Created
1. Uncompressed `mimetype` header per EPUB specification.
2. `META-INF/container.xml` linking to package root.
3. `OEBPS/content.opf` manifest and spine with asset deduplication.
4. `OEBPS/nav.xhtml` EPUB 3 Table of Contents.
5. All XHTML spine chapter documents and embedded images.\n