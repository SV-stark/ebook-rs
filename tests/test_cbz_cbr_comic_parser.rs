use ebook_rs::Book;

#[test]
fn test_real_cbz_sample_file() {
    let cbz_path = "samples/Jumbo Comics 099.cbz";
    let book = Book::from_file(cbz_path).expect("CBZ sample file should parse successfully");

    assert_eq!(book.metadata().title, "Jumbo Comics 099");
    assert_eq!(
        book.sections.len(),
        52,
        "CBZ sample should contain 52 comic pages"
    );
    assert_eq!(book.spine().len(), 52, "Spine items should match 52 pages");

    let page1 = book.get_section(0).expect("Page 1 section should exist");
    assert!(
        page1.processed_html.contains("data:image/jpeg;base64,"),
        "Page 1 HTML should contain embedded base64 image data URI"
    );
}

#[test]
fn test_real_cbr_sample_file_detection() {
    let cbr_path = "samples/Jumbo Comics 082.cbr";
    match Book::from_file(cbr_path) {
        Ok(_) => panic!("Expected CBR file to fail in pure-Rust mode"),
        Err(err) => {
            assert!(
                err.contains("CBR (RAR format) is not supported in pure-Rust mode"),
                "CBR should return clear Option A error message directing user to CBZ format: {}",
                err
            );
        }
    }
}
