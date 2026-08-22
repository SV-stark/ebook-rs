---
title: IDPF & Readium CFI Engine
description: Canonical Fragment Identifier parser, formatter, comparator, and DOM resolver.
---

EPUB Canonical Fragment Identifiers (CFI) provide a standardized mechanism for referencing locations inside eBook content.

```rust
use ebook_rs::cfi::Cfi;

let cfi = Cfi::parse("epubcfi(/6/4[chap01]!/4/2/10:15)")?;
let dom_selector = cfi.resolve_dom_path(html_content)?;
```

### Capabilities
- Parsing, validation, formatting, and mathematical range ordering.
- DOM path resolver converting CFI coordinates into browser DOM element offsets.
- Full compliance with the Readium Unified Locator Model.\n