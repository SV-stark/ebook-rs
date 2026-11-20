---
title: Model Context Protocol (MCP) Server
description: Native MCP JSON-RPC stdio & HTTP server for AI assistants.
---

`ebook-rs` includes a built-in implementation of Anthropic's **Model Context Protocol (MCP 2024-11-05)** specification.

### Available MCP Tools & Resources
- **`read_ebook_metadata`**: Inspects metadata, Table of Contents, and spine structure for any of the 30 supported input formats.
- **`search_ebook`**: Performs SIMD full-text search and returns highlighted snippet context with CFI locators.
- **`get_ebook_chapter`**: Fetches full section plain text or semantic HTML for a specific spine chapter.
- **`query_ebook_rag`**: Executes Okapi BM25 semantic chunk retrieval with exact CFI citation spans.
- **`convert_ebook`**: Converts any of the 30 supported input formats on disk to any of the **27 pure-Rust output formats** (`epub`, `kepub`, `kfx`, `mobi`, `azw3`, `fb2`, `pdf`, `docx`, `odt`, `rtf`, `pptx`, `html`, `htmlz`, `txt`, `txtz`, `md`, `oeb`, `zip`, `pdb`, `pmlz`, `rb`, `tcr`, `typ`/`typst`, `adoc`, `org`, `rst`, `tex`) or `json` (RAG chunks).
- **`export_ebook_text`**: Returns an eBook directly inline as clean text/markup (`md`, `txt`, `html`, `fb2`, `typst`, `adoc`, `org`, `rst`, `tex`) without writing intermediate files (with optional `max_chars` cap for LLM context windows).
- **`ebook://info`** *(Resource)*: Exposes engine version (`0.19.0`) and the full live list of supported input and output formats.

### Launching Server
```bash
# Stdio Mode (Default)
ebook-rs mcp

# Sandboxed to a specific library directory
ebook-rs mcp --sandbox-root /path/to/books
```
