# 📚 EBook-RS API Reference & Complete Documentation (v0.12.0)

`ebook-rs` (v0.12.0) is a multi-format pure Rust eBook engine supporting **EPUB 2**, **EPUB 3**, **MOBI**, **AZW3 (KF8)**, **FB2 (FictionBook 2)**, **KEPUB (Kobo EPUB)**, **LIT (Microsoft Reader)**, **CBZ (Comic Book ZIP)**, **PDF**, **ODT (OpenDocument Text)**, **Plain Text (.txt)**, and **Markdown (.md)** formats with **Native AI/RAG Document Chunking**, **C / Multi-Language FFI Bindings**, **Zero-Dependency Web Component Generator**, **CJK Vertical & RTL Layout Engine**, **SpeechSynthesis TTS Word Synchronizer**, **Legacy Non-UTF-8 Charset Decoding**, **Automatic Language Detection**, **Zstd Compressed State Caching**, **Universal EPUB 3 Exporter**, **Zero-Copy mmap**, **Lightweight DOM AST Tree**, **Fuzzy XML Recovery**, **CFI**, and **Readium LCP/Locator** support.

---

## 📋 Table of Contents
1. [Multi-Format Book Core API (`ebook_rs::Book`)](#1-multi-format-book-core-api-ebook_rsbook)
2. [Native AI & RAG Document Chunking Engine (`RagChunk`, `RagChunkConfig`)](#2-native-ai--rag-document-chunking-engine)
3. [C / Multi-Language FFI API (`ebook_rs::ffi`)](#3-c--multi-language-ffi-api)
4. [Zero-Dependency Web Component (`<ebook-reader>`)](#4-zero-dependency-web-component)
5. [CJK Vertical Text & RTL Reading Modes (`WritingMode`)](#5-cjk-vertical-text--rtl-reading-modes)
6. [SpeechSynthesis TTS Word Synchronizer (`TtsWordToken`, `get_tts_tokens`)](#6-speechsynthesis-tts-word-synchronizer)

---

## 2. Native AI & RAG Document Chunking Engine

Split eBooks into AI-ready semantic chunks with Markdown heading hierarchy, token estimations, and exact `epubcfi` citation anchors for Vector DBs and LLM prompt ingestion:

```rust
use ebook_rs::{Book, RagChunkConfig};

let book = Book::from_file("book.epub")?;
let config = RagChunkConfig {
    max_tokens: 512,
    overlap_tokens: 64,
    preserve_headings: true,
    include_cfi: true,
    min_chunk_size: 50,
};

let chunks = book.to_rag_chunks(&config);
for chunk in chunks {
    println!("Chunk ID: {}", chunk.id);
    println!("CFI Anchor: {}", chunk.cfi);
    println!("Text: {}", chunk.text);
    println!("Markdown Context: \n{}", chunk.markdown);
}
```

---

## 3. C / Multi-Language FFI API (`ebook_rs::ffi`)

C-compatible ABI (`#[unsafe(no_mangle)] extern "C"`) functions for zero-copy integration across Python (`ctypes`/`pyo3`), Node.js (`ffi-napi`), C/C++, Swift (iOS), Kotlin (Android), and Go:

```c
#include "ebook_rs.h"

int main() {
    uint8_t* buffer = load_file("book.epub", &len);
    CBookHandle book = ebook_rs_book_from_bytes(buffer, len);

    char* json_meta = ebook_rs_get_metadata_json(book);
    printf("Metadata: %s\n", json_meta);
    ebook_rs_string_free(json_meta);

    char* json_rag = ebook_rs_to_rag_chunks_json(book, 512);
    printf("RAG Chunks: %s\n", json_rag);
    ebook_rs_string_free(json_rag);

    ebook_rs_book_free(book);
    return 0;
}
```

---

## 4. Zero-Dependency Web Component

Generate a standalone `<ebook-reader>` custom HTML element JS definition string for WebAssembly applications:

```javascript
import init, { WasmBook } from './pkg/ebook_rs.js';

await init();
const customElementJs = WasmBook.get_custom_element_js();
eval(customElementJs); // Defines <ebook-reader> custom element
```

---

## 5. CJK Vertical Text & RTL Reading Modes (`WritingMode`)

Configure reading direction (`HorizontalLtr`, `HorizontalRtl`, `VerticalRl`, `VerticalLr`):

```rust
use ebook_rs::{RenditionLayout, WritingMode};

let mut layout = RenditionLayout::default();
layout.writing_mode = WritingMode::VerticalRl; // CJK Vertical Writing
let css = layout.to_css_override(); // Injects writing-mode: vertical-rl;
```
