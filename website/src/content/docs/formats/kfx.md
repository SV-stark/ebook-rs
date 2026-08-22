---
title: Amazon KFX (Clean-Room Parser)
description: Clean-room implementation of Amazon KF10 / KFX container format.
---

Amazon KFX is a proprietary compiled eBook format. `ebook-rs` features a clean-room parser and exporter for KFX containers.

### Technical Highlights
- **`b"CONT"` Container Parsing**: Validates chunk header magic bytes and symbol tables.
- **Fragment Indexing**: Decodes entity fragments, structural story lines, styles, and image payloads.
- **KFX Exporter**: Direct compilation from any parsed `Book` into clean-room KFX format.\n