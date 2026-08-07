use ebook_rs::Book;

#[test]
fn test_blackbox_cbr_rar_detection_error_message() {
    let rar_header = b"Rar!\x1a\x07\x00\x00\x00\x00\x00\x00\x00\x00";

    match Book::from_bytes(rar_header) {
        Ok(_) => panic!("CBR file should return Err in pure-Rust mode"),
        Err(err) => {
            assert!(
                err.contains("CBR (RAR format) is not supported in pure-Rust mode"),
                "Should return clear Option A error message directing user to convert to CBZ: {}",
                err
            );
        }
    }
}
