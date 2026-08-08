# 🚀 Getting Started with `ebook-rs`

`ebook-rs` provides a clean, unified API for loading, inspecting, searching, and displaying eBooks in Rust applications.

---

## 1. Installation

Add `ebook-rs` to your `Cargo.toml`:

```toml
[dependencies]
ebook-rs = "0.12.0"
```

---

## 2. Basic Usage

### Opening an eBook (EPUB, MOBI, AZW3, FB2, KEPUB, LIT)

```rust
use ebook_rs::Book;

fn main() -> Result<(), String> {
    // Book::from_file auto-detects format from magic bytes
    let mut book = Book::from_file("book.epub")?;

    println!("Title: {}", book.metadata().title);
    println!("Author: {:?}", book.metadata().creators);
    println!("Total Chapters: {}", book.spine().len());

    Ok(())
}
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

let total_locs = book.locations.total_locations;
let progress = book.locations.percentage_from_location(5);
println!("Total Locations: {}, Progress at loc 5: {:.2}%", total_locs, progress * 100.0);
```
