---
title: Microsoft Reader (LIT)
description: ITOL / ITLS container decoding and MSCompressed LZX streaming.
---

Microsoft Reader (.LIT) files use a proprietary compound file binary container and MSCompressed LZX stream compression.

### Capabilities
- **Directory Structure**: Parses ITOL/ITLS headers, chunk tables, and manifest streams.
- **LZX Decompression**: Multi-block sliding window decompression.
- **OEBPS Conversion**: Emits compliant HTML sections with embedded images.\n