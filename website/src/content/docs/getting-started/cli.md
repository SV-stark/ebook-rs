---
title: CLI Reference
description: Complete command-line interface guide for ebook-rs.
---

The `ebook-rs` command-line interface provides high-performance utilities for inspecting, querying, converting, and serving eBook collections.

### Commands Overview

```bash
ebook-rs <COMMAND> [OPTIONS]
```

---

### 1. `parse`
Prints eBook metadata, Table of Contents (TOC), and spine structure in JSON format for any of the **30 supported input formats**.

```bash
ebook-rs parse <path.epub|kfx|mobi|azw3|fb2|pdf|docx|odt|rtf|pptx|tex|typ|adoc|org|rst|pdb|pmlz|rb|tcr|htmlz|txtz|oeb|zip|...>
```

---

### 2. `search`
Runs high-speed SIMD accelerated full-text search across all book chapters and prints matched CFI locators and highlighted snippets.

```bash
ebook-rs search <path.epub> "search term"
```

---

### 3. `convert`
Converts any of the **30 supported input formats** to any of the **27 supported pure-Rust output formats** (`epub`, `kepub`, `kfx`, `mobi`, `azw3`, `fb2`, `pdf`, `docx`, `odt`, `rtf`, `pptx`, `html`, `htmlz`, `txt`, `txtz`, `md`, `oeb`, `zip`, `pdb`, `pmlz`, `rb`, `tcr`, `typ`/`typst`, `adoc`, `org`, `rst`, `tex`) or **AI RAG JSON** (`json`).

```bash
# Convert MOBI to EPUB 3
ebook-rs convert book.mobi output.epub

# Convert EPUB to Amazon KFX or Kobo KEPUB
ebook-rs convert book.epub output.kfx
ebook-rs convert book.epub output.kepub.epub

# Convert EPUB to PDF, DOCX, ODT, PPTX, LaTeX, Typst, AsciiDoc, Org, reST, or Markdown
ebook-rs convert book.epub output.pdf
ebook-rs convert book.epub output.docx
ebook-rs convert book.epub output.typ
ebook-rs convert book.epub output.md

# Export AI RAG JSON Chunks
ebook-rs convert book.epub output.json
```

---

### 4. `mcp`
Launches the **Model Context Protocol (MCP 2024-11-05)** JSON-RPC server on `stdio` for AI assistant integration, with optional `--sandbox-root` path confinement.

```bash
ebook-rs mcp
ebook-rs mcp --sandbox-root /path/to/library
```

---

### 5. `serve`
Spins up a lightweight local HTTP server and in-browser interactive eBook reader on port `8080`.

```bash
ebook-rs serve book.epub
```

---

### 6. `locations`
Computes IDPF Readium CFI reading progress locations across the entire book.

```bash
ebook-rs locations book.epub
```
