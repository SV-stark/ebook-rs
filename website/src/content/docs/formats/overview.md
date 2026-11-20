---
title: Supported Formats Matrix
description: Universal multi-format support overview in ebook-rs.
---

`ebook-rs` v0.19.0 provides pure-Rust **30 input** and **27 output** format codecs with zero C/Python runtime dependencies.

### E-Book Containers & Kindle / Palm Formats

| Format | Extensions | Input | Output | Engine / Codec |
|---|---|:---:|:---:|---|
| **EPUB 2 & 3** | `.epub` | ✅ | ✅ | Native OPF/NCX/NAV, FXL, SMIL, LCP DRM, `UniversalEpub3Exporter` |
| **Kobo EPUB** | `.kepub`, `.kepub.epub` | ✅ | ✅ | `koboSpan` sentence/segment wrapping & externalized OEB assets |
| **Amazon KFX** | `.kfx` | ✅ | ✅ | Clean-room Ion 1.0 `CONT` v2 / `ENTY` v1 parser & `UniversalKfxExporter` |
| **Kindle MOBI** | `.mobi` | ✅ | ✅ | PalmDOC LZ77 + EXTH + Huff/CDIC reader & PalmDB/MOBI writer |
| **Kindle AZW3** | `.azw3` | ✅ | ✅ | KF8/PalmDOC reader & Calibre-compatible PalmDB+EXTH+images writer |
| **FictionBook 2** | `.fb2` | ✅ | ✅ | Native XML + `<binary>` base64 images, poems/epigraphs |
| **Microsoft Reader** | `.lit` | ✅ | — | ITOL/ITLS MSCompressed LZX stream reader |
| **PalmDOC / zTXT** | `.pdb` | ✅ | ✅ | `TEXtREAd` (LZ77) & `zTXTGPlm` (zlib) PalmDB codec |
| **Palm Markup Archive** | `.pmlz` | ✅ | ✅ | PML `\c`/`\i`/`\b`/`\X` markup & image ZIP bundle |
| **Rocket eBook** | `.rb` | ✅ | ✅ | `NUVO` binary container with chunked zlib pages & TOC |
| **Psion TCR** | `.tcr` | ✅ | ✅ | `!!8-Bit!!` 256-entry dictionary compression codec |
| **Comic Book Archives** | `.cbz`, `.cbr` | ✅ | — | Natural-sorted image archives & double-page spread reader |
| **Sony BBeB** | `.lrf` | ⏳ | — | Legacy binary format (read-only roadmap) |

### Web, Package & Office Formats

| Format | Extensions | Input | Output | Engine / Codec |
|---|---|:---:|:---:|---|
| **HTML / XHTML** | `.html`, `.htm`, `.xhtml` | ✅ | ✅ | Self-contained HTML5 with CSS & data-URI images |
| **HTMLZ Archive** | `.htmlz` | ✅ | ✅ | Calibre-compatible `index.html` + `metadata.opf` + `images/` ZIP |
| **OEB Package** | `.oeb`, `.opf`, directory | ✅ | ✅ | Open eBook `content.opf` + XHTML spine + externalized `images/` |
| **Web ZIP** | `.zip` | ✅ | ✅ | Calibre-compatible HTML + `images/` ZIP container |
| **TXT / TXTZ** | `.txt`, `.txtz` | ✅ | ✅ | UTF-8 plain text & Calibre `index.txt` + `metadata.opf` archive |
| **Microsoft Word** | `.docx` | ✅ | ✅ | Native OOXML `word/document.xml`, styles, embedded media |
| **OpenDocument Text** | `.odt` | ✅ | ✅ | Native ODF `content.xml` / `meta.xml` / `Pictures/` |
| **Rich Text Format** | `.rtf` | ✅ | ✅ | Native control-word stack, `\pict` binary images, Unicode escapes |
| **PDF Document** | `.pdf` | ✅ *(feature `pdf`)* | ✅ | `unpdf` 2-column spatial reflow reader & zero-dep PDF 1.4 writer |
| **PowerPoint** | `.pptx` | ✅ | ✅ | OOXML slide deck (`ppt/slides/slideN.xml`) reader & writer |

### Technical & Academic Markup Formats

| Format | Extensions | Input | Output | Engine / Codec |
|---|---|:---:|:---:|---|
| **Markdown** | `.md`, `.markdown` | ✅ | ✅ | Headings, lists, blockquotes, fenced code, inline formatting |
| **Typst** | `.typ`, `.typst` | ✅ | ✅ | Native Typst markup (`=`, `*`, `_`, `#quote`, `#image`, raw blocks) |
| **AsciiDoc** | `.adoc`, `.asciidoc` | ✅ | ✅ | Native AsciiDoc sections, attributes, lists, source blocks |
| **Org-mode** | `.org` | ✅ | ✅ | Emacs Org-mode headings, `#+TITLE:`/`#+AUTHOR:`, `#+BEGIN_SRC` |
| **reStructuredText** | `.rst` | ✅ | ✅ | Sphinx/docutils underlined headings, directives, code blocks |
| **LaTeX** | `.tex`, `.latex` | ✅ | ✅ | `\chapter`/`\section`, `\textbf`/`\emph`, `itemize`/`quote`/`verbatim` |
