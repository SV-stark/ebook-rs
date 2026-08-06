# 📚 EBook-RS API Documentation & Wiki (v0.5.0)

Welcome to the **`ebook-rs`** API Documentation and Developer Wiki.

---

## 📑 Table of Contents

1. [Core Reader Engine (`Book`)](#1-core-reader-engine-book)
2. [Section & NLP Analytics API (`Section`, `ReadingAnalytics`)](#2-section--nlp-analytics-api-section-readinganalytics)
3. [Deterministic Reflow Paginator (`ReflowPaginator`)](#3-deterministic-reflow-paginator-reflowpaginator)
4. [Remote ZIP Central Directory Streamer (`ZipHeaderReader`)](#4-remote-zip-central-directory-streamer-zipheaderreader)
5. [Footnotes & OPDS Catalog Feed Client (`Footnote`, `OpdsFeed`)](#5-footnotes--opds-catalog-feed-client-footnote-opdsfeed)
6. [CFI Engine (`Cfi`)](#6-cfi-engine-cfi)
7. [WebAssembly (`wasm-bindgen`) & HTTP Reader Integration](#7-webassembly-wasm-bindgen--http-reader-integration)

---

## 1. Core Reader Engine (`Book`)

`Book` is the main entry point for loading, parsing, and searching eBook files.

```rust
use ebook_rs::Book;

// Load from a file path (auto-detects EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ)
let book = Book::from_file("book.epub")?;
```

---

## 2. Section & NLP Analytics API (`Section`, `ReadingAnalytics`)

```rust
let section = book.get_section(0)?;

// Compute NLP Reading Analytics
let analytics = section.analytics();
println!("Word Count: {}", analytics.word_count);
println!("Estimated Reading Time: {} mins", analytics.reading_time_minutes);
println!("Difficulty Score: {} / 10", analytics.difficulty_score);
println!("Top Keywords: {:?}", analytics.top_keywords);
```

---

## 3. Deterministic Reflow Paginator (`ReflowPaginator`)

Calculate line wraps and page break boundaries without browser layout reflow.

```rust
use ebook_rs::ReflowPaginator;

let paginator = ReflowPaginator::new(16, 1.6, 800, 600, 32);
let page_map = section.paginate(Some(&paginator));

println!("Total Virtual Pages: {}", page_map.total_pages);
for page in page_map.page_ranges {
    println!("Page {}: Chars {}..{}", page.page_number, page.start_char, page.end_char);
}
```

---

## 4. Remote ZIP Central Directory Streamer (`ZipHeaderReader`)

Locate EOCD `PK\x05\x06` signature from tail byte slices to generate HTTP `Range` headers for remote streaming.

```rust
use ebook_rs::{ZipHeaderReader, HttpRangeRequest};

let tail_bytes = fetch_http_tail_bytes("https://example.com/huge_comic.cbz", 65536)?;
let entries = ZipHeaderReader::parse_central_directory(&tail_bytes)?;

for entry in entries {
    let (header, val) = entry.to_http_range_header();
    println!("File: {}, Range: {}", entry.file_name, val);
}
```

---

## 5. Footnotes & OPDS Catalog Feed Client (`Footnote`, `OpdsFeed`)

```rust
// Extract footnotes for popup previewing
let footnotes = section.extract_footnotes();

// Parse OPDS 1.2 / 2.0 Catalog Feed
#[cfg(feature = "opds")]
let feed = ebook_rs::OpdsFeed::parse_atom_xml(&xml_string)?;
```

---

## 6. CFI Engine (`Cfi`)

```rust
use ebook_rs::Cfi;

let cfi = Cfi::parse("epubcfi(/6/4[chap01]!/4/2/1:0)")?;
println!("Spine Index: {}", cfi.spine_index());
```

---

## 7. WebAssembly (`wasm-bindgen`) & HTTP Reader Integration

```javascript
import { WasmBook } from "ebook-rs";

const book = new WasmBook(uint8Array);

// Get section reading analytics JSON
const analytics = JSON.parse(book.get_section_analytics_json(0));

// Get section reflow pagination JSON
const pageMap = JSON.parse(book.paginate_section_json(0));
```
