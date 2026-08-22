---
title: Zero-Allocation SIMD Search
description: Sub-millisecond full-text and regex search engine.
---

`ebook-rs` features an ultra-fast search engine built with SIMD-vectorized character scanning.

```rust
let results = book.search("Sherlock Holmes");
for r in results {
    println!("Spine: {} | CFI: {} | Match: {}", r.spine_index, r.cfi, r.snippet);
}
```

### Performance
- **Zero Allocations**: Match scanning operates on zero-copy string slices.
- **Vector Acceleration**: Uses `memchr` SIMD byte search.
- **XSS-Safe Snippets**: Search highlights wrap matched words in `<mark>` tags while sanitizing surrounding HTML.\n