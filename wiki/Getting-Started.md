# 🚀 Getting Started with `ebook-rs`

`ebook-rs` (**v0.19.0**) provides a clean, unified Rust API and CLI for loading, inspecting, searching, rendering, and converting eBooks across **30 input formats and 27 output formats**.

---

## 1. Installation

Add `ebook-rs` to your `Cargo.toml`:

```toml
[dependencies]
ebook-rs = "0.19.0"
```

Or install the CLI binary:

```bash
cargo install ebook-rs
```

To enable PDF input parsing (`unpdf`), opt into the `pdf` feature:

```toml
[dependencies]
ebook-rs = { version = "0.19.0", features = ["pdf"] }
```

---

## 2. Basic Usage

### Opening & Converting Any eBook

```rust
use ebook_rs::{Book, EbookError};

fn main() -> Result<(), EbookError> {
    // Auto-detects 30 formats from magic bytes, containers, or OEB directories
    let mut book = Book::from_file("book.epub")?;

    println!("Title: {}", book.metadata().title);
    println!("Author: {:?}", book.metadata().creators);
    println!("Total Chapters: {}", book.spine().len());

    // Export to any of 27 pure-Rust output formats
    let kfx_bytes = book.export_kfx_bytes()?;
    let docx_bytes = book.export_format_bytes("docx")?;
    let pdf_bytes = book.export_pdf_bytes()?;
    println!("KFX: {}B, DOCX: {}B, PDF: {}B", kfx_bytes.len(), docx_bytes.len(), pdf_bytes.len());

    Ok(())
}
```

### CLI Conversion (27+ Output Formats)

```bash
ebook-rs convert alice.epub alice.kfx
ebook-rs convert alice.epub alice.kepub.epub
ebook-rs convert alice.epub alice.docx
ebook-rs convert alice.epub alice.pdf
ebook-rs convert alice.mobi alice.epub
ebook-rs convert alice.epub alice.typ
```

### Full-Text Searching

```rust
let matches = book.search("Wonderland");
for m in matches {
    println!("Spine Index #{}: {}", m.spine_index, m.snippet);
}
```

### Locations & Reading Progress

```rust
// Generate discrete location chunks (1000 characters per location)
book.generate_locations(1000);

let total_locs = book.locations().total_locations;
let progress = book.locations().percentage_from_location(5);
println!("Total Locations: {}, Progress at loc 5: {:.2}%", total_locs, progress * 100.0);
```
