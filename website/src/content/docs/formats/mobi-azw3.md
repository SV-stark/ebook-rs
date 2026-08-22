---
title: Amazon MOBI & AZW3 (KF8)
description: Native PalmDOC LZ77 decompression and KF8 container parsing.
---

`ebook-rs` decodes Amazon Kindle legacy MOBI and modern KF8 AZW3 formats natively in pure Rust.

### Architecture
- **PalmDOC LZ77 Decompressor**: High-speed byte streaming decompression without external C libraries.
- **EXTH Metadata Parser**: Extracts ASIN, cover offset, author, publisher, and DRM status.
- **KF8 Container & CSS**: Parses modern KF8 boundary records, extracting HTML5, CSS3, and embedded fonts.\n