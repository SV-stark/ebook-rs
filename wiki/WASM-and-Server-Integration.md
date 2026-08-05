# 🌐 WebAssembly & HTTP Reader Server Integration

`ebook-rs` (v0.2.0) is designed for both browser WebAssembly environments and native multithreaded desktop/server applications.

---

## 1. WebAssembly Browser Bindings (`wasm-bindgen`)

Compile `ebook-rs` to WebAssembly for client-side web applications:

```bash
wasm-pack build --target web --release
```

### JavaScript / WASM Interface (`WasmBook`)

```javascript
import init, { WasmBook } from './pkg/ebook_rs.js';

async function run() {
    await init();

    const response = await fetch('alice.epub');
    const bytes = new Uint8Array(await response.arrayBuffer());

    // Load book in WASM
    const book = WasmBook.from_bytes(bytes);

    console.log("Title:", book.get_title());
    console.log("Sections:", book.get_spine_count());

    // Search full text in WebAssembly
    const resultsJson = book.search("Rabbit");
    console.log("Search Results:", JSON.parse(resultsJson));
}
```

---

## 2. Built-in Multithreaded HTTP Reader Server

Serve books over HTTP with live REST API endpoints and built-in web reader interface:

```rust
use ebook_rs::Book;
use ebook_rs::server::ReaderServer;

fn main() -> Result<(), String> {
    let book = Book::from_file("alice.epub")?;
    let server = ReaderServer::new(book, 8080);

    println!("Starting HTTP Reader Server on http://localhost:8080");
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
