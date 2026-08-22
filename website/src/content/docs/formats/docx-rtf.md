---
title: Microsoft Word (DOCX) & Rich Text Format (RTF)
description: WordML and RTF control sequence decoding with asset extraction.
---

### Microsoft Word (`.docx`)
- Parses `word/document.xml` and `word/_rels/document.xml.rels`.
- Extracts paragraph styles, bold/italic runs, tables, lists, and embedded image files.
- Converts to semantic HTML5 with 19.4× faster speed than Calibre.

### Rich Text Format (`.rtf`)
- State machine parser managing control words (`\b`, `\i`, `\par`, `\ul`).
- Extracts hexadecimal binary image payloads (`\pict`).
- Full Unicode escape sequences support (`\uN?` and code-page auto-detection).\n