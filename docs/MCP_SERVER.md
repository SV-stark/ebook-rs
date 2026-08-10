# Model Context Protocol (MCP) Server for `ebook-rs`

`ebook-rs` includes a built-in **Model Context Protocol (MCP)** server enabling AI assistants (Claude Desktop, Antigravity, Cursor, Zed, ChatGPT Local, etc.) to read, search, analyze, and convert eBooks seamlessly via standard JSON-RPC 2.0 over `stdio` or `HTTP/SSE`.

---

## 🚀 Running the MCP Server

You can launch the `ebook-rs` MCP server using the CLI:

```bash
cargo run --release -- mcp
# or using compiled binary:
ebook-rs mcp
```

The server listens on `stdio` for standard MCP JSON-RPC 2.0 requests (protocol version `2024-11-05`).

Alternatively, start the HTTP Reader server (`ebook-rs serve`) to query MCP over HTTP via POST `/api/mcp`.

---

## 🛠 Available Tools

| Tool Name | Description | Required Arguments | Optional Arguments |
| :--- | :--- | :--- | :--- |
| `get_metadata` | Extract full metadata (title, author, publisher, description, language, rights, total spine count) from an eBook file. | `path` | - |
| `get_toc` | Get complete Table of Contents navigation hierarchy and chapter tree. | `path` | - |
| `read_section` | Read a specific chapter/section by index or chapter title. Returns text or Markdown with heading structure and CFI anchor. | `path` | `section_index`, `chapter_title`, `format` (`"text"` / `"markdown"`) |
| `search_book` | Search for keyword/phrase across eBook contents, returning matched snippets, line numbers, section indices, and CFIs. | `path`, `query` | `max_results` (default `20`) |
| `chunk_book_for_rag` | Chunk an eBook into semantic passages optimized for RAG / vector embeddings with token estimates, heading trails, and Okapi BM25 relevance scoring. | `path` | `max_tokens` (default `512`), `overlap_tokens` (default `64`), `query_rank` |
| `convert_ebook` | Convert eBook between supported formats (`.epub`, `.kfx`, or RAG `.json`). | `input_path`, `output_path` | - |
| `validate_epub` | Validate EPUB file structural integrity against EPUB specifications. | `path` | - |

---

## 📦 MCP Resources & Prompts

### Resources (`resources/list`, `resources/read`)
- `ebook://info`: Returns engine capabilities, supported formats, and feature capabilities.
- `ebook://{path}/metadata`: Inspect book metadata.
- `ebook://{path}/toc`: Inspect Table of Contents tree.

### Prompts (`prompts/list`, `prompts/get`)
- `summarize_book`: Generate a structured chapter-by-chapter summary prompt.
- `extract_entities`: Prompt for cataloging key characters, locations, and terms.
- `generate_study_guide`: Prompt for generating executive summary & discussion questions.

---

## ⚙️ Configuration for AI Clients

### 1. Claude Desktop (`claude_desktop_config.json`)

Add `ebook-rs` to your `claude_desktop_config.json` (located at `%APPDATA%\Claude\claude_desktop_config.json` on Windows or `~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "ebook-rs": {
      "command": "E:/ebook-rs/target/release/ebook-rs.exe",
      "args": ["mcp"]
    }
  }
}
```

### 2. Antigravity / Cursor / VS Code (`mcp_config.json`)

In your workspace or global `.gemini/mcp_config.json` or Cursor settings:

```json
{
  "mcpServers": {
    "ebook-rs": {
      "command": "cargo",
      "args": ["run", "--manifest-path", "E:/ebook-rs/Cargo.toml", "--release", "--", "mcp"]
    }
  }
}
```

---

## 💡 Example Prompt Capabilities for LLMs

Once connected, your AI assistant can run prompts such as:
- *"Extract the metadata and Table of Contents for `books/dune.epub`."*
- *"Search for references to 'spice' in `books/dune.epub` and rank chapters using Okapi BM25 scoring."*
- *"Read Chapter 3 of `books/alice.epub` and summarize key events."*
- *"Chunk `books/handbook.epub` for vector embedding ingestion with 512 max tokens per chunk."*
- *"Validate `books/custom.epub` and report any structural or manifest errors."*
