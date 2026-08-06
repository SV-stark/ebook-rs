use ebook_rs::{Book, TxtBook};

#[test]
fn test_plain_text_parser() {
    let text_content = "Paragraph 1: Welcome to plain text reading in ebook-rs.\n\nParagraph 2: Full-text search and CFI mapping works seamlessly.";
    let book = TxtBook::parse(text_content.as_bytes(), "Sample Text Document", false)
        .expect("Plain text should parse");

    assert_eq!(book.metadata().title, "Sample Text Document");
    assert_eq!(book.spine().len(), 1);
    assert!(
        book.sections[0]
            .plain_text
            .contains("Welcome to plain text")
    );

    let search_res = book.search("ebook-rs");
    assert_eq!(search_res.len(), 1);
}

#[test]
fn test_markdown_parser() {
    let md_content = "# Quantum Computing Overview\n\nQuantum computing leverages superposition and entanglement.\n\n## Hardware Architectures\n\nSuperconducting qubits and trapped ions are primary platforms.";
    let book = TxtBook::parse(md_content.as_bytes(), "Markdown Book", true)
        .expect("Markdown should parse");

    assert_eq!(book.metadata().title, "Quantum Computing Overview");
    assert!(
        book.spine().len() >= 2,
        "Markdown sections split by headings"
    );
    assert_eq!(book.toc.len(), 2, "TOC extracted from Markdown headings");

    let matches = book.search("qubits");
    assert_eq!(matches.len(), 1);
}

#[test]
fn test_odt_archive_parser() {
    // Generate minimal valid ODT ZIP in memory for testing
    let mut zip_buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));
        let options = zip::write::SimpleFileOptions::default();

        zip.start_file("mimetype", options).unwrap();
        use std::io::Write;
        zip.write_all(b"application/vnd.oasis.opendocument.text")
            .unwrap();

        zip.start_file("meta.xml", options).unwrap();
        zip.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-meta xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><office:meta><dc:title>OpenDocument Test</dc:title><meta:initial-creator>Rust Author</meta:initial-creator></office:meta></office:document-meta>").unwrap();

        zip.start_file("content.xml", options).unwrap();
        zip.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\"><office:body><office:text><text:h text:outline-level=\"1\">Chapter 1: ODT Introduction</text:h><text:p>This is a test paragraph inside OpenDocument Text.</text:p></office:text></office:body></office:document-content>").unwrap();

        zip.finish().unwrap();
    }

    let book = Book::from_bytes(&zip_buf).expect("ODT buffer should auto-detect and parse");
    assert_eq!(book.metadata().title, "OpenDocument Test");
    assert_eq!(book.metadata().creator(), "Rust Author");
    assert!(book.sections[0].plain_text.contains("OpenDocument Text"));
}
