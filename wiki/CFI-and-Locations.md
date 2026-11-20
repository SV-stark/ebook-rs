# 📍 CFI Engine and Locations Progress Indexing

`ebook-rs` (**v0.19.0**) implements a complete EPUB Canonical Fragment Identifier (CFI) parser, DOM resolver, and discrete location chunk generator.

---

## 1. Canonical Fragment Identifiers (CFI)

CFI is the IDPF standard string format for referencing specific DOM nodes, character offsets, and text ranges inside EPUB documents.

### Parsing, Comparing & Formatting CFIs

```rust
use ebook_rs::Cfi;
use std::cmp::Ordering;

let cfi1 = Cfi::parse("epubcfi(/6/2!/4/2)").expect("Valid CFI");
let cfi2 = Cfi::parse("epubcfi(/6/4[chap01]!/4/2/10:12)").expect("Valid CFI");

println!("Spine Item Index: {:?}", cfi2.spine_index());
println!("Element ID Assertion: {:?}", cfi2.element_id());
println!("Formatted: {}", cfi2);
assert_eq!(cfi1.compare(&cfi2), Ordering::Less);
```

### Exact In-Section Offset & DOM Resolution

```rust
use ebook_rs::Book;

let book = Book::from_file("alice.epub")?;

// Resolve section + in-section character offset
let (section, offset) = book.get_section_and_offset_by_cfi("epubcfi(/6/4!/4/2/1:12)")?;

// Full DOM resolution returning a CSS selector + character offset
let (_sec, _off, dom_target) =
    book.resolve_cfi("epubcfi(/6/4!/4/2/1:12)", &section.processed_html)?;
println!("CSS selector: {}, offset: {}", dom_target.css_selector, dom_target.char_offset);
```

---

## 2. Locations Progress Engine

`ebook-rs` divides book content into uniform character chunks (default 1000 characters per location) for reading progress and Readium locators:

```rust
book.generate_locations(1000);

let total_locations = book.locations().total_locations;
let progress = book.locations().percentage_from_location(12);

println!("Total Locations: {}", total_locations);
println!("Progress at Location 12: {:.2}%", progress * 100.0);
```
