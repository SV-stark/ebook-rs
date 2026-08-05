use ebook_rs::Book;

#[test]
fn test_epub2_compatibility() {
    let path = "samples/Alice in Wonderland - Lewis Carroll EPUB2.epub";
    let book = Book::from_file(path).expect("Should parse EPUB2 sample file");

    println!("\n📖 Testing EPUB 2 Specific Features:");
    println!("   - Title:       '{}'", book.metadata().title);
    println!("   - OPF Version: '{}'", book.opf.version);
    println!("   - Spine Items: {}", book.spine().len());
    println!("   - TOC Points:  {}", book.toc().len());

    assert_eq!(book.opf.version, "2.0");
    assert!(book.spine().len() > 0);
    assert!(!book.toc().is_empty(), "EPUB 2 NCX TOC should be parsed");

    let matches = book.search("Rabbit");
    assert!(
        matches.len() > 0,
        "Full-text search should find matches in EPUB 2"
    );
    println!(
        "   - EPUB 2 Search: {} matches found for 'Rabbit'\n",
        matches.len()
    );
}

#[test]
fn test_epub3_compatibility() {
    let path = "samples/Alice in Wonderland - Lewis Carroll EPUB3.epub";
    let book = Book::from_file(path).expect("Should parse EPUB3 sample file");

    println!("\n📖 Testing EPUB 3 Specific Features:");
    println!("   - Title:       '{}'", book.metadata().title);
    println!("   - OPF Version: '{}'", book.opf.version);
    println!("   - Spine Items: {}", book.spine().len());
    println!("   - TOC Points:  {}", book.toc().len());
    println!("   - Landmarks:   {}", book.landmarks().len());
    println!("   - Page List:   {}", book.page_list().len());

    assert!(book.opf.version.starts_with("3"));
    assert!(book.spine().len() > 0);
    assert!(
        !book.toc().is_empty(),
        "EPUB 3 NAV XHTML TOC should be parsed"
    );

    let matches = book.search("Rabbit");
    assert!(
        matches.len() > 0,
        "Full-text search should find matches in EPUB 3"
    );
    println!(
        "   - EPUB 3 Search: {} matches found for 'Rabbit'\n",
        matches.len()
    );
}
