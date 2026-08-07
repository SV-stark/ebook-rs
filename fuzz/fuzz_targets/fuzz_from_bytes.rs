#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz `Book::from_bytes` with arbitrary byte sequences.
/// Covers: ZIP parsing, EPUB parsing, MOBI PDB parsing, FB2 XML, PDF header detection,
/// LIT header parsing, ODT ZIP, TXT/MD fallback, and all format auto-detection paths.
///
/// Run with: cargo +nightly fuzz run fuzz_from_bytes
fuzz_target!(|data: &[u8]| {
    // Must not panic, must return Ok or Err — never undefined behavior.
    let _ = ebook_rs::Book::from_bytes(data);
});
