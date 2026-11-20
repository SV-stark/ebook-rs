---
title: Feature Parity Matrix
description: Detailed comparison of ebook-rs against foliate-js, epub.js, and rbook.
---

### 📂 Format Support (30 Input / 27 Output Pure-Rust Formats)
| Feature | 🚀 `ebook-rs` (v0.19.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **EPUB 2 & 3 (In + Out)** | ✅ Full OPF + NCX/NAV + Exporter | ✅ Read | ✅ Read | ✅ Read |
| **EPUB 3 Fixed-Layout (FXL)** | ✅ 2-page spread renderer | ✅ Yes | ✅ Yes | ❌ No |
| **Amazon KFX (KF10, In + Out)** | ✅ Clean-room `CONT` v2 + Exporter | ❌ No | ✅ Read | ❌ No |
| **MOBI & AZW3 (KF8, In + Out)** | ✅ Native PalmDOC LZ77 + Exporters | ❌ No | ✅ Read | ❌ No |
| **FB2 & KEPUB (In + Out)** | ✅ Native XML / `koboSpan` + Exporters | ❌ No | ✅ Read | ❌ No |
| **LIT & CBZ/CBR (In)** | ✅ Native LZX / Archive Readers | ❌ No | ✅ Read | ❌ No |
| **PDF (In + Out)** | ✅ `unpdf` reflow + zero-dep PDF 1.4 | ❌ No | ❌ No | ❌ No |
| **Office: DOCX, ODT, RTF, PPTX (In + Out)** | ✅ Native Pure-Rust Codecs | ❌ No | ❌ No | ❌ No |
| **Legacy & Packages: PDB, PMLZ, RB, TCR, HTMLZ, TXTZ, OEB, ZIP** | ✅ Full Bidirectional Codecs | ❌ No | ❌ No | ❌ No |
| **Markup: MD, HTML, Typst, AsciiDoc, Org, reST, LaTeX** | ✅ Full Bidirectional Codecs | ❌ No | ❌ No | ❌ No |

### 🧭 Navigation & AI Features
| Feature | 🚀 `ebook-rs` (v0.19.0) | 📦 `epub.js` | 📖 `foliate-js` | 🦀 `rbook` |
|---|:---:|:---:|:---:|:---:|
| **Model Context Protocol (MCP)** | ✅ Stdio & HTTP + 27-Format `convert_ebook` + `export_ebook_text` | ❌ No | ❌ No | ❌ No |
| **Okapi BM25 RAG Chunking** | ✅ `rank_chunks_bm25()` | ❌ No | ❌ No | ❌ No |
| **SIMD Full-Text Search** | ✅ SIMD memchr scanning | ❌ No | ✅ Basic | ❌ No |
| **IDPF CFI Engine** | ✅ Parse / format / compare / range | ✅ Yes | ✅ Yes | ✅ Yes |
| **TTS Word Synchronizer** | ✅ Word token span offsets | ❌ No | ❌ No | ❌ No |
| **Readium LCP DRM** | ✅ Full License Parser | ❌ No | ✅ Yes | ❌ No |
