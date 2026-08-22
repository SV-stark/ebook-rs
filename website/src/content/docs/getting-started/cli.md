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
Prints eBook metadata, Table of Contents (TOC), and spine structure in JSON format.

```bash
ebook-rs parse <path.epub|mobi|pdf|fb2|docx|rtf>
```

---

### 2. `search`
Runs high-speed SIMD accelerated full-text search across all book chapters and prints matched CFI locators and highlighted snippets.

```bash
ebook-rs search <path.epub> "search term"
```

---

### 3. `convert`
Converts any supported input format (MOBI, AZW3, KFX, FB2, LIT, DOCX, RTF, PDF, TXT) to **EPUB 3**, **Amazon KFX**, or **AI RAG JSON**.

```bash
# Convert MOBI to EPUB 3
ebook-rs convert book.mobi output.epub

# Convert EPUB to Amazon KFX
ebook-rs convert book.epub output.kfx

# Export AI RAG JSON Chunks
ebook-rs convert book.epub output.json
```

---

### 4. `mcp`
Launches the **Model Context Protocol (MCP 2024-11-05)** JSON-RPC server on `stdio` for AI assistant integration.

```bash
ebook-rs mcp
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
```\n