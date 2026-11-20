# 🌐 WebAssembly, HTTP Reader Server & MCP Integration

`ebook-rs` (**v0.19.0**) runs in browser WebAssembly environments, native desktop/mobile apps, HTTP reader servers, and AI agent workflows via the Model Context Protocol (`MCP 2024-11-05`).

---

## 1. WebAssembly Browser Bindings (`wasm-bindgen`)

Compile `ebook-rs` to WebAssembly for client-side web applications:

```bash
wasm-pack build --target web --release --no-default-features --features wasm
```

### JavaScript / WASM Interface (`WasmBook`)

```javascript
import init, { WasmBook } from './pkg/ebook_rs.js';

async function run() {
    await init();

    const response = await fetch('alice.epub');
    const bytes = new Uint8Array(await response.arrayBuffer());

    const book = WasmBook.from_bytes(bytes);
    console.log("Title:", book.get_title());
    console.log("Sections:", book.get_spine_count());

    const resultsJson = book.search("Rabbit");
    console.log("Search Results:", JSON.parse(resultsJson));
}
```

---

## 2. Built-in Multithreaded HTTP Reader Server

Serve books over HTTP (`127.0.0.1` loopback by default, with optional `--auth-token` / `EBOOK_RS_AUTH_TOKEN`):

```rust
use ebook_rs::Book;
use ebook_rs::server::ReaderServer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let book = Book::from_file("alice.epub")?;
    let server = ReaderServer::new(book, 8080);

    println!("Starting HTTP Reader Server on http://127.0.0.1:8080");
    server.start()?;
    Ok(())
}
```

### Server API Endpoints
- `GET /`: Serves the interactive double-spread / continuous scroll Web Reader UI.
- `GET /api/metadata`: Returns JSON book metadata.
- `GET /api/toc`: Returns JSON table of contents.
- `GET /api/section/:index`: Returns rendered chapter section HTML.
- `GET /api/search?q=query`: Executes full-text search and returns JSON matches.

---

## 3. Built-in Model Context Protocol (`MCP 2024-11-05`) Server

Run `ebook-rs` as a sandboxed stdio MCP server for AI assistants:

```bash
ebook-rs mcp --sandbox-root /path/to/your/ebooks
```

### Exposed MCP Tools
- `get_metadata` / `get_toc` / `read_section` / `search_book` / `chunk_book_for_rag` / `validate_epub`
- `convert_ebook`: Convert any supported input book to any of **27+ output formats** (`.epub`, `.kepub`, `.kfx`, `.mobi`, `.azw3`, `.pdb`, `.pdf`, `.docx`, `.odt`, `.rtf`, `.pptx`, `.fb2`, `.html`, `.htmlz`, `.txt`, `.txtz`, `.md`, `.oeb`, `.zip`, `.tcr`, `.pmlz`, `.rb`, `.typ`, `.adoc`, `.org`, `.rst`, `.tex`, or RAG `.json`).
- `export_ebook_text`: Return inline UTF-8 text/markup (`md`, `txt`, `html`, `fb2`, `typst`, `adoc`, `org`, `rst`, `tex`) directly to the LLM without writing files.
- `ebook://info` resource: Reports version, `supported_input_formats`, `supported_output_formats`, and engine capabilities.
