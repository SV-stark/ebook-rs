use ebook_rs::dom::decode_bytes_with_encoding;

#[test]
fn test_blackbox_legacy_charset_decoding() {
    // Windows-1252 bytes for "Hello World"
    let win1252_bytes = b"Hello World";
    let decoded = decode_bytes_with_encoding(win1252_bytes, Some("windows-1252"));
    assert_eq!(decoded, "Hello World");

    // Fallback to UTF-8 on invalid encoding name
    let utf8_bytes = "UTF-8 Direct".as_bytes();
    let decoded_fallback = decode_bytes_with_encoding(utf8_bytes, Some("unknown-encoding-xyz"));
    assert_eq!(decoded_fallback, "UTF-8 Direct");
}
