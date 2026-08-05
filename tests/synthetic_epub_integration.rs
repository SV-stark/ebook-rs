use ebook_rs::{AnnotationType, Book, Cfi, Theme, generate_sample_epub};

#[test]
fn test_full_epub_parsing_and_reader_parity() {
    // 1. Generate test EPUB 3 bytes
    let epub_bytes = generate_sample_epub().expect("Should generate sample EPUB");

    // 2. Open book with Book::from_bytes
    let mut book = Book::from_bytes(&epub_bytes).expect("Should parse sample EPUB");

    // 3. Verify Metadata Parity
    let meta = book.metadata();
    assert_eq!(meta.title, "The Rustonomicon & EBook-RS Guide");
    assert_eq!(meta.creators, vec!["Antigravity AI"]);
    assert_eq!(meta.languages, vec!["en"]);
    assert_eq!(meta.publishers, vec!["Rust Ebook Publishers"]);

    // 4. Verify Spine and TOC
    assert_eq!(book.spine().len(), 3);
    let toc = book.toc();
    assert_eq!(toc.len(), 3);
    assert_eq!(toc[0].label, "Chapter 1: Welcome to EBook-RS");
    assert_eq!(
        toc[1].label,
        "Chapter 2: Canonical Fragment Identifiers (CFI)"
    );
    assert_eq!(toc[2].label, "Chapter 3: Full-Text Search and Annotations");

    // 5. Verify Section Loading & Processing
    let section1 = book.get_section(0).expect("Should get section 0");
    assert!(section1.plain_text.contains("Welcome to ebook-rs"));
    assert!(section1.char_count > 0);

    let section2 = book
        .get_section_by_href("ch2.xhtml")
        .expect("Should find ch2");
    assert!(
        section2
            .plain_text
            .contains("Canonical Fragment Identifiers")
    );

    // 6. Verify EPUB CFI Engine
    let cfi_str = "epubcfi(/6/4[chap01ref]!/4/2/10/1:5)";
    let parsed_cfi = Cfi::parse(cfi_str).expect("Should parse CFI");
    assert_eq!(parsed_cfi.spine_index(), 1);
    assert_eq!(parsed_cfi.char_offset(), 5);
    assert_eq!(parsed_cfi.to_string(), cfi_str);

    // 7. Verify Locations & Progress Engine
    assert!(book.locations.total_locations > 0);
    let first_cfi = book
        .locations
        .cfi_from_location(1)
        .expect("Should have CFI for loc 1");
    let loc_entry = book
        .locations
        .location_from_cfi(&first_cfi)
        .expect("Should map back to location");
    assert_eq!(loc_entry.location, 1);

    let pct = book.locations.percentage_from_cfi(&first_cfi);
    assert!(pct >= 0.0 && pct <= 1.0);

    // 8. Verify Full-Text Search Engine
    let search_results = book.search("CFI");
    assert!(!search_results.is_empty());
    assert!(search_results[0].snippet.contains("CFI"));
    assert!(search_results[0].cfi.starts_with("epubcfi("));

    // 9. Verify Annotations Manager
    let ann = book.annotations.create_highlight(
        "epubcfi(/6/2!/4/2/1:0)",
        "#fef08a",
        Some("Highlighted text"),
        Some("Test note"),
    );
    assert_eq!(ann.type_, AnnotationType::Highlight);
    assert_eq!(ann.color, "#fef08a");
    assert_eq!(book.annotations.list().len(), 1);

    // 10. Verify Layout Theme CSS Generation
    book.layout.theme = Theme::Sepia;
    let css = book.layout.to_css_override();
    assert!(css.contains("--reader-bg: #fef3c7"));
}
