use ebook_rs::{Book, TxtBook};

#[test]
fn test_blackbox_universal_epub3_exporter() {
    // TxtBook uses the first H1 heading as the title when markdown=true
    let md = "# Export Test Book\n\nSection content for universal EPUB3 exporter verification.";
    let book = TxtBook::parse(md.as_bytes(), "Ignored Title", true).unwrap();

    let epub_bytes = book
        .export_epub3_bytes()
        .expect("Should export clean EPUB 3 bytes");
    assert!(!epub_bytes.is_empty());

    // Re-parse exported EPUB bytes to verify roundtrip fidelity
    let reloaded = Book::from_bytes(&epub_bytes).expect("Exported EPUB should re-parse cleanly");
    // Title is derived from the markdown H1 heading
    assert!(
        reloaded.metadata().title.contains("Export Test Book"),
        "Title was: {}",
        reloaded.metadata().title
    );
    assert_eq!(reloaded.sections.len(), 1);
    assert!(
        reloaded.sections[0]
            .plain_text
            .contains("universal EPUB3 exporter")
    );
}
