---
title: Amazon KFX Exporter
description: Compile standard eBooks into Amazon KFX container format.
---

`UniversalKfxExporter` compiles eBooks into native Amazon KFX `b"CONT"` containers with fragment indexes and CBOR/Ion binary symbol tables.

```rust
use ebook_rs::{Book, UniversalKfxExporter};

let book = Book::from_file("input.epub")?;
let kfx_bytes = UniversalKfxExporter::export(&book)?;
std::fs::write("output.kfx", kfx_bytes)?;
```\n