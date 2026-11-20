# 🏗️ Multi-Format Architecture

`ebook-rs` (**v0.19.0**) abstracts 30 input formats and 27 output formats into a single unified `Book` model in 100% pure Rust.

---

## 🏛️ Architecture Overview

```
                     +----------------------------------+
                     | Book::from_file / Book::from_bytes|
                     +----------------+-----------------+
                                      |
      +---------------+---------------+---------------+---------------+
      |               |               |               |               |
      v               v               v               v               v
+-----------+   +-----------+   +-----------+   +-----------+   +-----------+
| EPUB 2/3  |   | KFX Ion   |   | MOBI/AZW3 |   | Office &  |   | Markup &  |
| KEPUB/OEB |   | CONT v2   |   | PDB/LIT   |   | Archives  |   | Text      |
+-----+-----+   +-----+-----+   +-----+-----+   +-----+-----+   +-----+-----+
      |               |               |               |               |
      +---------------+---------------+---------------+---------------+
                                      |
                                      v
                     +----------------------------------+
                     |     Unified `Book` Structure     |
                     | Metadata, Spine, TOC, Sections   |
                     +----------------+-----------------+
                                      |
                                      v
                     +----------------------------------+
                     |  `Book::export_format_bytes()`   |
                     |  27 Pure-Rust Output Exporters   |
                     +----------------------------------+
```

### Core Format Modules
- **`src/archive.rs` & `src/opf.rs`**: EPUB 2/3 & KEPUB ZIP archive reader with OPF, NCX, NAV, landmarks, page-list, and lazy on-demand hydration.
- **`src/kfx/`**: Clean-room Amazon Ion 1.0 `CONT` v2 / `YJ_symbols` reader & writer preserving embedded publisher fonts (`$262`/`$418`), CSS `font-family` cascade (`$11` in `$157`), bidi/RTL direction (`$682`/`$674`), tables, internal/external anchors (`$266`), hierarchical TOC (`$389`), and cover/non-spine images (`$164`/`$417`).
- **`src/mobi.rs` & `src/formats/kindle_and_palm.rs`**: PalmDB reader/writer for MOBI, AZW3 (KF8), and PalmDOC/zTXT PDB with PalmDOC LZ77 compression, EXTH metadata, and embedded image records.
- **`src/formats/web_and_archives.rs`**: Pure-Rust Input & Output for KEPUB (`koboSpan`), FB2 (`<binary>` images), standalone HTML5 (inlined `data:` images), HTMLZ, TXTZ, OEB (dir/OPF/ZIP), generic HTML/TXT ZIP, Markdown, Plain Text, and Psion TCR (`!!8-Bit!!` 256-entry dictionary).
- **`src/formats/office_and_markup.rs`**: Pure-Rust DOCX, ODT, RTF exporters; PMLZ & Rocket eBook (`rb`, `NUVO` + chunked zlib) Input & Output; and Typst (`.typ`), AsciiDoc (`.adoc`), Org-mode (`.org`), and reStructuredText (`.rst`) Input & Output.
- **`src/formats/pdf_tex_pptx.rs`**: Zero-dependency pure-Rust PDF 1.4 paginated writer, LaTeX (`.tex`) Input & Output, and PowerPoint OOXML (`.pptx`) Input & Output.
- **`src/docx.rs`, `src/odt.rs`, `src/rtf.rs`, `src/fb2.rs`, `src/lit.rs`, `src/cbz.rs`, `src/pdf.rs`**: Dedicated native readers for Word, OpenDocument, RTF, FictionBook 2, Microsoft Reader LIT, CBZ comics, and PDF (`pdf` feature).
