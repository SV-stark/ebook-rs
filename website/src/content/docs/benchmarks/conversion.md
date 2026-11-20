---
title: Conversion Benchmarks (vs Calibre)
description: Empirical speed and memory benchmarks comparing ebook-rs with Calibre.
---

Empirically measured **v0.19.0 (`--release`, 3-run median)** conversion benchmarks vs **Calibre 9.18** on *Alice in Wonderland* (Windows 11 / AMD Ryzen 9 / PCIe 4.0 NVMe):

### Input Format → EPUB 3

| Input Format | 🚀 `ebook-rs` (v0.19.0) | 🐍 Calibre 9.18 | Speedup | Notes |
|---|:---:|:---:|:---:|---|
| **DOCX → EPUB** | **0.16s** ⚡ | 2.95s | **18.1× Faster** | 37 / 37 images preserved |
| **TXT → EPUB** | **0.11s** ⚡ | 1.98s | **17.8× Faster** | Auto-chapter & paragraph reflow |
| **CBZ → EPUB (21 MB)** | **0.83s** ⚡ | 12.87s | **15.5× Faster** | Natural-sorted comic archive |
| **TCR → EPUB** | **0.14s** ⚡ | 2.08s | **15.0× Faster** | Psion `!!8-Bit!!` dictionary codec |
| **AZW3 (KF8) → EPUB** | **0.15s** ⚡ | 2.19s | **14.7× Faster** | 38 / 38 images preserved |
| **PDB → EPUB** | **0.15s** ⚡ | 2.15s | **14.7× Faster** | PalmDOC `TEXtREAd` & `zTXTGPlm` |
| **FB2 → EPUB** | **0.15s** ⚡ | 1.77s | **11.9× Faster** | 37 / 37 images preserved |
| **PMLZ → EPUB** | **0.16s** ⚡ | 1.87s | **11.6× Faster** | Palm Markup + images ZIP |
| **HTMLZ → EPUB** | **0.19s** ⚡ | 1.97s | **10.4× Faster** | Calibre `index.html` + `metadata.opf` |
| **RB → EPUB** | **0.21s** ⚡ | 2.02s | **9.8× Faster** | Rocket eBook `NUVO` chunked zlib |
| **TXTZ → EPUB** | **0.14s** ⚡ | 1.25s | **9.1× Faster** | Calibre `index.txt` + `metadata.opf` |
| **LIT → EPUB** | **0.24s** ⚡ | 1.99s | **8.4× Faster** | MSCompressed LZX stream |
| **MOBI → EPUB** | **0.27s** ⚡ | 2.19s | **8.0× Faster** | 38 / 38 images preserved |
| **KFX → EPUB** | **0.40s** ⚡ | 3.05s *(Plugin Req.)* | **7.6× Faster** | **17.1× Faster** on `ebook-rs` KFX |
| **RTF → EPUB** | **0.58s** ⚡ | 2.86s | **4.9× Faster** | All 36 images preserved (vs Calibre's 1) |

### EPUB 3 → Multi-Format Exporters

| Target Output Format | 🚀 `ebook-rs` (v0.19.0) | 🐍 Calibre 9.18 | Speedup |
|---|:---:|:---:|:---:|
| **EPUB → KFX** | **0.31s** ⚡ | 60.99s *(KFX Output)* | **~195× Faster** |
| **EPUB → PDF** | **0.26s** ⚡ | 15.62s | **60.3× Faster** |
| **EPUB → TXTZ** | **0.16s** ⚡ | 1.92s | **11.7× Faster** |
| **EPUB → RB** | **0.23s** ⚡ | 2.65s | **11.5× Faster** |
| **EPUB → RTF** | **0.25s** ⚡ | 2.41s | **9.8× Faster** |
| **EPUB → PMLZ** | **0.26s** ⚡ | 2.39s | **9.1× Faster** |
| **EPUB → TCR** | **0.36s** ⚡ | 3.16s | **8.8× Faster** |
| **EPUB → FB2** | **0.22s** ⚡ | 1.90s | **8.5× Faster** |
| **EPUB → ZIP** | **0.25s** ⚡ | 1.98s | **7.8× Faster** |
| **EPUB → MOBI** | **0.36s** ⚡ | 2.64s | **7.3× Faster** |
| **EPUB → HTMLZ** | **0.26s** ⚡ | 1.87s | **7.2× Faster** |
| **EPUB → AZW3** | **0.33s** ⚡ | 1.91s | **5.8× Faster** |
| **EPUB → DOCX** | **0.69s** ⚡ | 3.85s | **5.6× Faster** |
| **EPUB → PDB** | **0.35s** ⚡ | 1.70s | **4.9× Faster** |
| **EPUB → OEB** | **0.35s** ⚡ | 1.71s | **4.8× Faster** |
| **EPUB → KEPUB / ODT / PPTX / HTML / MD / Typst / ADOC / Org / RST / TeX** | **0.24s–0.27s** ⚡ | — *(Pure-Rust Native)* | **Sub-300ms** |

### Analysis
- **KFX & PDF Export**: Clean-room Ion 1.0 `CONT` v2 compilation (**~195×**) and zero-dependency PDF 1.4 generation (**60.3×**) eliminate external Kindle Previewer / Chromium WebEngine bottlenecks.
- **Bidirectional Interoperability**: Every Calibre-supported format was verified end-to-end against Calibre 9.18 `ebook-convert` in both directions.
- **Memory Footprint**: `ebook-rs` maintains low peak memory during conversion with zero C/Python runtime overhead.
