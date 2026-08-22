---
title: Feature Parity Matrix
description: Detailed comparison of ebook-rs against foliate-js, epub.js, and rbook.
---

### 📂 Format Support
| Feature | 🚀 `ebook-rs` (v0.16.4) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 Support** | ✅ Full OPF + NCX/NAV | ✅ Yes | ✅ Yes | ✅ Yes |
| **EPUB 3 Fixed-Layout (FXL)** | ✅ 2-page spread renderer | ✅ Yes | ✅ Yes | ❌ No |
| **Amazon KFX (KF10)** | ✅ Clean-room `b"CONT"` container | ❌ No | ✅ Yes | ❌ No |
| **MOBI & AZW3 (KF8)** | ✅ Native PalmDOC LZ77 | ❌ No | ✅ Yes | ❌ No |
| **FB2 (FictionBook 2)** | ✅ Native XML + xlink:href | ❌ No | ✅ Yes | ❌ No |
| **KEPUB (Kobo EPUB)** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **LIT (Microsoft Reader)** | ✅ Native | ❌ No | ✅ Yes | ❌ No |
| **CBZ (Comic Book ZIP)** | ✅ Native ZIP Images | ❌ No | ✅ Yes | ❌ No |
| **PDF 2-Column Reflow** | ✅ `pdf_oxide` spatial reflow | ❌ No | ❌ No | ❌ No |
| **DOCX (Microsoft Word)** | ✅ Native WordML | ❌ No | ❌ No | ❌ No |
| **RTF (Rich Text Format)** | ✅ Native Control Words | ❌ No | ❌ No | ❌ No |

### 🧭 Navigation & AI Features
| Feature | 🚀 `ebook-rs` (v0.16.4) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **Model Context Protocol (MCP)** | ✅ Built-in Stdio & HTTP Server | ❌ No | ❌ No | ❌ No |
| **Okapi BM25 RAG Chunking** | ✅ `rank_chunks_bm25()` | ❌ No | ❌ No | ❌ No |
| **SIMD Full-Text Search** | ✅ SIMD memchr scanning | ❌ No | ✅ Basic | ❌ No |
| **IDPF CFI Engine** | ✅ Parse / format / compare / range | ✅ Yes | ✅ Yes | ✅ Yes |
| **TTS Word Synchronizer** | ✅ Word token span offsets | ❌ No | ❌ No | ❌ No |
| **Readium LCP DRM** | ✅ Full License Parser | ❌ No | ✅ Yes | ❌ No |\n