# 📍 CFI Engine and Locations Progress Indexing

`ebook-rs` implements a complete EPUB Canonical Fragment Identifier (CFI) parser and discrete location chunk generator.

---

## 1. Canonical Fragment Identifiers (CFI)

CFI is the IDPF standard string format for referencing specific DOM nodes, character offsets, and text ranges inside EPUB documents.

### Parsing and Formatting CFIs

```rust
use ebook_rs::Cfi;

// Parse a standard EPUB CFI string
let cfi = Cfi::parse("epubcfi(/6/4[chap01]!/4/2/10)").expect("Valid CFI");

println!("Spine Item Index: {:?}", cfi.spine_index());
println!("Element ID Assertion: {:?}", cfi.element_id());

// Format back to string
println!("Re-formatted CFI: {}", cfi);
```

### Comparing CFI Locations

`Cfi::compare()` orders CFIs to determine reading progression order:

```rust
use std::cmp::Ordering;

let cfi1 = Cfi::parse("epubcfi(/6/2!/4/2)").unwrap();
let cfi2 = Cfi::parse("epubcfi(/6/4!/4/2)").unwrap();

assert_eq!(cfi1.compare(&cfi2), Ordering::Less);
```

---

## 2. Locations Progress Engine

`ebook-rs` divides full book content into uniform character chunks (default 1000 characters per location) to calculate precise reading progress.

```rust
// Generate locations (1000 chars per location)
book.generate_locations(1000);

let total_locations = book.locations.total_locations;
let progress = book.locations.percentage_from_location(12);

println!("Total Locations: {}", total_locations);
println!("Progress at Location 12: {:.2}%", progress * 100.0);
```
