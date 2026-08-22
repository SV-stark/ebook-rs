---
title: FictionBook 2 (FB2)
description: XML-based FictionBook 2 parsing with Base64 asset extraction.
---

FB2 documents store metadata, structured book chapters, and Base64-encoded binary images in a single XML payload.

### Features
- **Fast XML Processing**: Streamlined parsing via `roxmltree`.
- **Image Extraction**: Base64 binary decoding directly into internal archive for zero-copy rendering.
- **Poetry & Epigraphs**: Preserves rich styling including stanzas, verses, epigraphs, and footnotes.\n