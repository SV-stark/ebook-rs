---
title: Supported Formats Matrix
description: Universal multi-format support overview in ebook-rs.
---

`ebook-rs` provides pure Rust parsing and conversion for 12+ industry formats with zero C/Python runtime dependencies.

| Format | Extension | Engine / Parser | Features Supported |
|---|---|---|---|
| **EPUB 2 & 3** | `.epub` | Native OPF/NCX/NAV | Full styling, Fixed-Layout (FXL), Media Overlays (SMIL), LCP DRM |
| **Kindle MOBI** | `.mobi` | PalmDOC LZ77 + EXTH | PalmDOC decompression, Huffman CDIC records, image asset mapping |
| **Kindle AZW3** | `.azw3` | KF8 / PalmDOC LZ77 | KF8 CSS3 styles, embedded fonts, TOC spine extraction |
| **Amazon KFX** | `.kfx` | Clean-Room Container | `b"CONT"` chunk parser, fragment indexing, Ion/CBOR symbol table |
| **FictionBook 2** | `.fb2` | Native XML Document | Base64 binary image extraction, nested poems/epigraphs, metadata |
| **Kobo EPUB** | `.kepub` | Kobo Span Engine | `koboSpan` CFI mapping, page progression direction |
| **Microsoft Reader**| `.lit` | ITOL/ITLS LZX Engine | MSCompressed LZX streaming, manifest tables, internal images |
| **Comic Book ZIP**| `.cbz`, `.cbr`| Native ZIP / Archive | Natural numerical sorting, double-page comic spread renderer |
| **PDF Document** | `.pdf` | `pdf_oxide` Spatial | 2-column academic reflow, font size clustering, heading detection |
| **Word Document** | `.docx` | Native WordML | OpenXML `document.xml`, styles, embedded PNG/JPG extraction |
| **Rich Text Format**| `.rtf` | Native Control Parser | Group stack parsing, `\pict` hex binary decoding, Unicode escapes |
| **Plain / Markdown**| `.txt`, `.md` | Markdown Reflow | Auto-heading detection, paragraph chunking, code formatting |\n