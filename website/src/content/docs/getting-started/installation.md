---
title: Installation
description: Install ebook-rs via Cargo, Pip, or WASM NPM package.
---

### 🦀 Rust Crate

Add `ebook-rs` to your `Cargo.toml`:

```toml
[dependencies]
ebook-rs = "0.16.4"
```

Or via `cargo add`:

```bash
cargo add ebook-rs
```

#### Available Cargo Features

| Feature | Description | Default |
|---|---|:---:|
| `server` | Built-in HTTP web reader server (`tiny_http`) | ✅ Yes |
| `mcp` | Native Model Context Protocol (MCP) JSON-RPC server | ✅ Yes |
| `parallel` | Multi-threaded rayon parsing & search | ✅ Yes |
| `mmap` | Memory-mapped I/O reader for giant files | ✅ Yes |
| `zstd` | High-compression state caching (`zstd`) | ✅ Yes |
| `opds` | OPDS 1.2 / 2.0 catalog feeds | ✅ Yes |
| `python` | PyO3 Python bindings | ❌ No |
| `wasm` | WebAssembly (`wasm-bindgen`) bindings | ❌ No |
| `pdf` | Native PDF spatial reflow engine (`pdf_oxide`) | ❌ No |

---

### 💻 CLI Executable

Install the standalone binary directly on your system:

```bash
cargo install ebook-rs
```

Verify installation:
```bash
ebook-rs --help
```

---

### 🐍 Python Package

Install pre-compiled binary wheels for Linux, macOS, and Windows:

```bash
pip install ebook-rs
```

---

### 📦 WebAssembly / NPM

```bash
npm install @sv-stark/ebook-rs
```\n