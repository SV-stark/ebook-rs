#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz `Cfi::parse` with arbitrary CFI strings.
/// Covers all CFI step types, malformed IDPF paths, non-ASCII characters,
/// extremely deep nesting, and empty/whitespace-only strings.
///
/// Run with: cargo +nightly fuzz run fuzz_cfi_parse
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = ebook_rs::Cfi::parse(s);
    }
});
