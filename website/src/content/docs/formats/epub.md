---
title: EPUB 2 & 3 (Fixed-Layout & Reflowable)
description: Deep dive into EPUB specification support in ebook-rs.
---

`ebook-rs` provides complete compliance with **W3C EPUB 3.3** and **IDPF EPUB 2.0.1** specifications.

### Key Capabilities
- **Package Parsing**: Reads `META-INF/container.xml` and resolves root OPF package documents.
- **Navigation**: Dual-engine parser supporting both EPUB 2 NCX (`toc.ncx`) and EPUB 3 Navigation Document (`nav.xhtml`).
- **Fixed-Layout (FXL)**: Detects `rendition:layout="pre-paginated"`, orientation spreads (`rendition:spread`), and zero-allocation viewport metadata extraction.
- **RTL & CJK Vertical Text**: Full support for right-to-left page progression direction and vertical-rl writing modes.
- **Lazy Archive Loading**: Uncompresses files on demand, with automatic streaming mode for oversized EPUB files (>500MB).\n