---
title: Model Context Protocol (MCP) Server
description: Native MCP JSON-RPC stdio & HTTP server for AI assistants.
---

`ebook-rs` includes a built-in implementation of Anthropic's **Model Context Protocol (MCP 2024-11-05)** specification.

### Available MCP Tools & Resources
- **`open_book`**: Mounts an eBook file into the assistant context.
- **`search_book`**: Performs SIMD full-text search and returns highlighted snippet context.
- **`read_section`**: Fetches full section text or semantic HTML for a specific spine chapter.
- **`query_rag`**: Executes BM25 semantic chunk retrieval with exact CFI citation spans.
- **`export_epub`**: Converts books to standard EPUB 3 on the fly.

### Launching Server
```bash
# Stdio Mode (Default)
ebook-rs mcp
```\n