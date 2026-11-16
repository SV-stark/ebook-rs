---
title: Amazon KFX (Clean-Room Parser)
description: Clean-room Amazon Ion 1.0 / KF10 KFX container parser and exporter.
---

Amazon KFX (Kindle Format 10) is a compiled binary eBook format built on Amazon Ion 1.0. `ebook-rs` includes a clean-room Ion 1.0 binary codec (`src/kfx/ion.rs`), `CONT` v2 / `ENTY` v1 container parser (`src/kfx/container.rs`), `KfxBook` reader (`src/kfx/reader.rs`), and `UniversalKfxExporter` (`src/kfx/writer.rs`).

### Technical Highlights

- **`b"CONT"` v2 & Ion 1.0 Binary Codec**: Parses `container_info`, `format_capabilities`, `YJ_symbols` v10 local symbol tables, and `ENTY` v1 framed entities (including `0xD1` sorted structs and `Decimal` values).
- **Full-Fidelity Reconstruction**: Restores book metadata (`$490`/`$258`), hierarchical TOC (`$389`), storylines & tables (`$259`/`$278`/`$279`), internal/external anchors (`$266`), embedded images & covers (`$164`/`$417`), embedded fonts (`$262`/`$418`), and RTL/bidi direction (`$538`/`$157`/`$142`).
- **KFX Exporter**: Direct compilation from any parsed `Book` into interoperable `.kfx` containers.
