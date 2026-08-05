# 🏗️ Multi-Format Architecture

`ebook-rs` (v0.2.0) abstracts format-specific container structures into a unified `Book` model.

---

## 🏛️ Architecture Overview

```
                      +-----------------------------+
                      |       Book::from_bytes      |
                      +--------------+--------------+
                                     |
             +-----------------------+-----------------------+
             |                       |                       |
             v                       v                       v
     +---------------+       +---------------+       +---------------+
     |  EpubArchive  |       |   MobiBook    |       |    Fb2Book    |
     | EPUB 2 / 3    |       | MOBI / AZW3   |       | FictionBook 2 |
     +-------+-------+       +-------+-------+       +-------+-------+
             |                       |                       |
             +-----------------------+-----------------------+
                                     |
                                     v
                      +-----------------------------+
                      |    Unified Book Structure   |
                      |  Metadata, Spine, Sections  |
                      +-----------------------------+
```

### Supported Format Handlers
- **`EpubArchive`**: ZIP archive reader supporting EPUB 2 (`content.opf`, `toc.ncx`) and EPUB 3 (`nav.xhtml`, landmarks, page-list).
- **`MobiBook`**: Binary Palm Database (PDB) container parser with PalmDOC LZ77 decompression and EXTH metadata extraction.
- **`Fb2Book`**: FictionBook 2 XML parser with embedded Base64 binary image extraction.
- **`LitBook`**: Microsoft Reader ITOL/ITLS binary container extractor.
