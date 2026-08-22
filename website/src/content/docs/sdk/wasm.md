---
title: WebAssembly SDK (WASM)
description: Run ebook-rs directly in browsers and Node.js with WebAssembly.
---

### Installation
```bash
npm install @sv-stark/ebook-rs
```

### Web Browser Integration
```typescript
import init, { WasmBook } from '@sv-stark/ebook-rs';

async function run() {
  await init();
  
  const res = await fetch('/books/alice.epub');
  const buffer = new Uint8Array(await res.arrayBuffer());
  
  const book = WasmBook.from_bytes(buffer);
  console.log('Book Title:', book.title);
  
  const sectionHtml = book.get_section_html(0);
  document.getElementById('content').innerHTML = sectionHtml;
}

run();
```\n