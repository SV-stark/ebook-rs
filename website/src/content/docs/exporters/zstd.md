---
title: Zstandard State Caching
description: Instant resume with sub-millisecond Zstd compressed book states.
---

For server and reader applications, re-parsing large books on every request creates unnecessary CPU overhead. `ebook-rs` supports zero-overhead snapshot caching using Zstandard.

```rust
// Save compressed book state to disk / Redis
let cache_bytes = book.export_zstd_cache()?;

// Instantly restore in <1ms without re-parsing
let restored_book = Book::from_zstd_cache(&cache_bytes)?;
```\n