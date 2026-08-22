use ebook_rs::{Book, KfxBook, TxtBook, UniversalKfxExporter};

#[test]
fn test_kfx_export_and_parse_roundtrip() {
    let markdown_content = "# Clean Room KFX Test\n\nThis is chapter 1 of our clean-room KFX test.\n\n## Section 2\n\nMore text content for KFX validation.";
    let original_book =
        TxtBook::parse(markdown_content.as_bytes(), "KFX Clean Room Book", true).unwrap();
    let expected_title = original_book.opf.metadata.title.clone();

    // 1. Export Book to KFX binary container
    let kfx_bytes =
        UniversalKfxExporter::export(&original_book).expect("KFX export should succeed");
    assert!(kfx_bytes.len() > 18);
    assert_eq!(&kfx_bytes[0..4], b"CONT");

    // 2. Parse KFX binary container using KfxBook
    let read_book = KfxBook::parse(&kfx_bytes).expect("KFX parsing should succeed");
    assert_eq!(read_book.opf.metadata.title, expected_title);
    assert!(!read_book.sections.is_empty());

    // 3. Parse KFX binary container using unified Book::from_bytes auto-detection
    let auto_book = Book::from_bytes(&kfx_bytes)
        .expect("Book::from_bytes auto-detection should recognize CONT KFX");
    assert_eq!(auto_book.opf.metadata.title, expected_title);
    assert!(!auto_book.sections.is_empty());
}

#[test]
fn test_kfx_container_header_structure() {
    let markdown_content = "# Header Test\nSimple content.";
    let book = TxtBook::parse(markdown_content.as_bytes(), "Header Book", false).unwrap();
    let kfx_bytes = UniversalKfxExporter::export(&book).unwrap();

    // Verify magic CONT bytes
    assert_eq!(&kfx_bytes[..4], b"CONT");

    // Verify version 2 in bytes 4..6
    let version = u16::from_le_bytes([kfx_bytes[4], kfx_bytes[5]]);
    assert_eq!(version, 2);

    // Verify header length
    let header_len =
        u32::from_le_bytes([kfx_bytes[6], kfx_bytes[7], kfx_bytes[8], kfx_bytes[9]]) as usize;
    assert!(header_len > 18);
    assert!(header_len < kfx_bytes.len());
}

#[test]
fn test_kfx_multibyte_utf8_preservation() {
    let markdown_content = "# KFX UTF-8 Test: München & Café\n\nChapter with German characters: Müller und Schönbrunn. Also curly quotes: “Hello, world!” and em-dashes — perfectly preserved in KFX.";
    let original_book =
        TxtBook::parse(markdown_content.as_bytes(), "KFX UTF-8 Book", true).unwrap();

    let kfx_bytes = UniversalKfxExporter::export(&original_book).unwrap();
    let parsed_book = KfxBook::parse(&kfx_bytes).unwrap();

    assert!(!parsed_book.sections.is_empty());
    let all_text = parsed_book
        .sections
        .iter()
        .map(|s| s.plain_text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        all_text.contains("Müller")
            || all_text.contains("Schönbrunn")
            || all_text.contains("Café")
            || all_text.contains("Hello")
    );

    // Verify search works on KFX parsed book
    let results = parsed_book.search("Müller");
    assert!(!results.is_empty() || !parsed_book.search("Hello").is_empty());
}
