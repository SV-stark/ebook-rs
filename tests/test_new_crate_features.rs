use ebook_rs::{Book, decode_bytes_with_encoding};

#[test]
fn test_legacy_encoding_rs_decoding() {
    // Windows-1252 byte sequence with smart quotes and em-dash
    let win1252_bytes = b"Hello \x93World\x94 \x97 EBook-RS";
    let decoded = decode_bytes_with_encoding(win1252_bytes, Some("windows-1252"));
    assert!(decoded.contains("World"));
    assert!(decoded.contains("EBook-RS"));

    // Shift-JIS byte sequence (Japanese "こんにちは")
    let shift_jis_bytes = &[0x82, 0xb1, 0x82, 0xf1, 0x82, 0xc9, 0x82, 0xbf, 0x82, 0xcd];
    let decoded_jp = decode_bytes_with_encoding(shift_jis_bytes, Some("shift_jis"));
    assert_eq!(decoded_jp, "こんにちは");
}

#[test]
fn test_whatlang_language_detection() {
    let book = Book::from_file("samples/Alice in Wonderland - Lewis Carroll EPUB3.epub")
        .expect("Sample EPUB should parse");

    let detected_lang = book.detect_language();
    assert!(detected_lang.is_some());
    let lang_code = detected_lang.unwrap();
    assert!(
        lang_code == "en" || lang_code == "eng",
        "Expected English language code, got: {}",
        lang_code
    );
}

#[test]
fn test_zstd_compressed_state_caching() {
    let book = Book::from_file("samples/Alice in Wonderland - Lewis Carroll EPUB3.epub")
        .expect("Sample EPUB should parse");

    let zstd_bytes = book
        .export_zstd_cache()
        .expect("Exporting zstd compressed state cache should succeed");

    assert!(!zstd_bytes.is_empty());

    let restored_book = Book::from_zstd_cache(&zstd_bytes)
        .expect("Restoring book from zstd compressed cache should succeed");

    assert_eq!(restored_book.opf.metadata.title, book.opf.metadata.title);
    assert_eq!(restored_book.sections.len(), book.sections.len());
}

#[test]
fn test_speech_synthesis_tts_word_synchronizer() {
    let book = Book::from_file("samples/Alice in Wonderland - Lewis Carroll EPUB3.epub")
        .expect("Sample EPUB should parse");

    let tts_tokens = book
        .get_tts_tokens(0)
        .expect("Should extract TTS tokens for section 0");
    assert!(
        !tts_tokens.is_empty(),
        "TTS tokens list should not be empty"
    );

    let first_token = &tts_tokens[0];
    assert_eq!(first_token.index, 0);
    assert!(!first_token.word.is_empty());
    assert!(first_token.char_end > first_token.char_start);

    let tts_html = book
        .get_tts_section_html(0)
        .expect("Should generate TTS annotated HTML");
    assert!(
        tts_html.contains("id=\"tts-w-0\""),
        "TTS annotated HTML must contain id=\"tts-w-0\""
    );
    assert!(
        tts_html.contains("class=\"tts-word\""),
        "TTS annotated HTML must contain class=\"tts-word\""
    );
}
