---
title: Quick Start Guide
description: Get up and running with ebook-rs in Rust, Python, CLI, and WASM.
---

import { Tabs, TabItem } from '@astrojs/starlight/components';

Learn how to open, parse, search, convert, and query eBook files in seconds.

<Tabs>
  <TabItem label="Rust">
```rust
use ebook_rs::{Book, SearchEngine, RagChunkConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open any eBook (EPUB, MOBI, AZW3, KFX, FB2, LIT, DOCX, RTF, PDF)
    let book = Book::from_file("books/alice.epub")?;

    println!("Title: {}", book.metadata().title);
    println!("Author: {}", book.metadata().creators.join(", "));
    println!("Total Spine Sections: {}", book.sections.len());

    // 2. Extract section text & raw HTML
    let section = book.get_section(0)?;
    println!("Chapter 1 Preview:
{}", &section.plain_text[..200]);

    // 3. Zero-allocation SIMD full-text search
    let results = book.search("Rabbit");
    for hit in results.iter().take(3) {
        println!("[Section {}] CFI: {} | Snippet: {}", hit.spine_index, hit.cfi, hit.snippet);
    }

    // 4. Generate AI RAG document chunks with BM25 ranking
    let chunks = book.to_rag_chunks(&RagChunkConfig::default());
    let ranked = ebook_rs::rag::RagChunker::rank_chunks_bm25(&chunks, "White Rabbit", 3);
    println!("Top RAG Score: {}", ranked[0].bm25_score);

    Ok(())
}
```
  </TabItem>

  <TabItem label="Python">
```python
from ebook_rs import PyBook

# 1. Open eBook
book = PyBook.open("books/alice.epub")

print(f"Title: {book.title}")
print(f"Authors: {book.authors}")
print(f"Sections: {book.section_count}")

# 2. Get chapter section HTML & Plain Text
html = book.get_section_html(0)
text = book.get_section_text(0)

# 3. Perform Full-Text Search
matches = book.search("Rabbit")
for m in matches[:3]:
    print(f"CFI: {m.cfi} | Snippet: {m.snippet}")

# 4. Export RAG Chunks JSON for LLMs
rag_json = book.to_rag_chunks_json(max_tokens=512)
```
  </TabItem>

  <TabItem label="CLI">
```bash
# Parse metadata and Table of Contents
ebook-rs parse sample.epub

# Perform zero-copy SIMD search
ebook-rs search sample.epub "Rabbit"

# Convert formats (MOBI/AZW3/DOCX/RTF -> EPUB 3 or KFX)
ebook-rs convert sample.mobi output.epub

# Run Model Context Protocol (MCP) server for Claude / Cursor
ebook-rs mcp

# Launch local HTTP Reader Web Server on Port 8080
ebook-rs serve sample.epub
```
  </TabItem>

  <TabItem label="WebAssembly (WASM)">
```typescript
import { WasmBook } from '@sv-stark/ebook-rs';

const response = await fetch('/books/sample.epub');
const arrayBuffer = await response.arrayBuffer();
const bytes = new Uint8Array(arrayBuffer);

// Open eBook from byte stream
const book = WasmBook.from_bytes(bytes);

console.log('Title:', book.title);
console.log('Spine Count:', book.section_count);

// Extract section HTML
const chapterHtml = book.get_section_html(0);
document.getElementById('viewer').innerHTML = chapterHtml;
```
  </TabItem>
</Tabs>\n