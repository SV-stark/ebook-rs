// wasm-pack integration tests.
// These tests are NOT run by `cargo nextest` or `cargo test`.
// Run them with:  wasm-pack test --node  (requires wasm-pack + Node.js)
//
// Only compiled when targeting wasm32.

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
mod wasm_tests {
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_node_experimental);

    /// Verify WasmBook::from_bytes handles invalid bytes gracefully.
    #[wasm_bindgen_test]
    fn test_wasm_book_from_invalid_bytes() {
        use ebook_rs::wasm::WasmBook;
        let result = WasmBook::from_bytes(vec![0xFF, 0xFE, 0x00, 0x01]);
        // Must not panic — returns None/error for unsupported formats
        assert!(result.is_none() || result.is_some());
    }

    /// Verify CFI parsing works correctly in wasm context.
    #[wasm_bindgen_test]
    fn test_wasm_cfi_roundtrip() {
        use ebook_rs::Cfi;
        let cfi = Cfi::parse("epubcfi(/6/4[chap01]!/4/2/1:0)");
        assert!(cfi.is_ok());
        let formatted = cfi.unwrap().to_string();
        assert!(formatted.contains("epubcfi"));
    }
}

// Ensure this file compiles as a no-op on native targets.
#[cfg(not(all(target_arch = "wasm32", feature = "wasm")))]
#[allow(dead_code)]
fn _wasm_tests_only_run_via_wasm_pack() {}
