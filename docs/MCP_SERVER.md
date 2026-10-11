# Model Context Protocol (MCP) Server for `ebook-rs`

`ebook-rs` includes a built-in **Model Context Protocol (MCP)** server enabling AI assistants (Claude Desktop, Antigravity, Cursor, Zed, ChatGPT Local, etc.) to read, search, analyze, and convert eBooks seamlessly via standard JSON-RPC 2.0 over `stdio` or HTTP POST (`/api/mcp`).

---

## 🚀 Running the MCP Server

You can launch the `ebook-rs` MCP server using the CLI:

```bash
cargo run --release -- mcp
# or using compiled binary:
ebook-rs mcp
```

The server listens on `stdio` for standard MCP JSON-RPC 2.0 requests (protocol version `2024-11-05`).

Alternatively, start the HTTP Reader server (`ebook-rs serve`) to query MCP over HTTP via POST `/api/mcp`. There is **no SSE transport**; the endpoint accepts a single JSON-RPC request and returns a single JSON response.

### ⚠️ Filesystem Access Is Sandboxed

MCP tools that touch the filesystem (`get_metadata`, `get_toc`, `read_section`, `search_book`, `chunk_book_for_rag`, `validate_epub`, `convert_ebook`, and the `ebook://{path}/…` resources) are **refused unless a sandbox root is configured**. When one is set, every path argument — and `convert_ebook`'s `output_path` — is resolved against it and rejected if it escapes (including via symlinks).

```bash
# stdio: sandbox defaults to the current working directory
ebook-rs mcp
ebook-rs mcp --sandbox-root /path/to/ebooks

# HTTP: same flag applies to /api/mcp
ebook-rs serve sample.epub --sandbox-root /path/to/ebooks
```

`EBOOK_RS_SANDBOX_ROOT` sets the same thing via the environment.

### 🔒 HTTP Authentication

The HTTP `/api/mcp` endpoint (and the other reader API routes) are **unauthenticated unless you
configure a token**, because any local process can reach `127.0.0.1:8080`. A loud warning is
printed at startup when no token is set.

```bash
ebook-rs serve sample.epub --auth-token "$SECRET"   # or: EBOOK_RS_AUTH_TOKEN
curl -H "Authorization: Bearer $SECRET" -H 'Content-Type: application/json' \
     -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' http://localhost:8080/api/mcp
```

An empty token is rejected outright — `with_auth_token("")` would otherwise authenticate every
request. Token comparison is constant-time. See [`SECURITY.md`](../SECURITY.md).

---

## 🛠 Available Tools

| Tool Name | Description | Required Arguments | Optional Arguments |
| :--- | :--- | :--- | :--- |
| `get_metadata` | Extract full metadata (title, author, publisher, description, language, rights, total spine count) from an eBook file. | `path` | - |
| `get_toc` | Get complete Table of Contents navigation hierarchy and chapter tree. | `path` | - |
| `read_section` | Read a specific chapter/section by index or chapter title. Returns text or Markdown with heading structure and CFI anchor. | `path` | `section_index`, `chapter_title`, `format` (`"text"` / `"markdown"`) |
| `search_book` | Search for keyword/phrase across eBook contents, returning matched snippets, line numbers, section indices, and CFIs. | `path`, `query` | `max_results` (default `20`) |
| `chunk_book_for_rag` | Chunk an eBook into semantic passages optimized for RAG / vector embeddings with token estimates, heading trails, and Okapi BM25 relevance scoring. | `path` | `max_tokens` (default `512`), `overlap_tokens` (default `64`), `query_rank` |
| `convert_ebook` | Convert eBook between supported formats (`.epub`, `.kfx`, or RAG `.json`). Output must resolve inside the sandbox root. | `input_path`, `output_path` | - |
| `validate_epub` | Validate EPUB file structural integrity against EPUB specifications. | `path` | - |

All seven tools take a filesystem path, so all seven require a configured sandbox root.

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
