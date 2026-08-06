use ebook_rs::Book;

#[test]
fn test_regex_full_text_search() {
    let epub_path = "samples/Alice in Wonderland - Lewis Carroll EPUB3.epub";
    let book = Book::from_file(epub_path).expect("Should parse EPUB3");

    // Search regex for case-insensitive 'alice' or 'rabbit'
    let matches = book
        .search_regex("(?i)alice|rabbit")
        .expect("Regex search should succeed");
    assert!(
        !matches.is_empty(),
        "Regex search for 'alice|rabbit' should return matches"
    );

    let first = &matches[0];
    assert!(
        first.snippet.contains("<mark>"),
        "Snippet should contain <mark> highlights"
    );
    assert!(!first.cfi.is_empty(), "CFI should be populated");
}

#[test]
fn test_epub_structural_validator() {
    let epub_path = "samples/Alice in Wonderland - Lewis Carroll EPUB3.epub";
    let book = Book::from_file(epub_path).expect("Should parse EPUB3");

    let report = book.validate();
    assert!(
        report.is_valid,
        "Sample EPUB3 should pass validation with 0 errors"
    );
    assert_eq!(report.errors_count, 0);
}

#[test]
fn test_book_fingerprint_and_deduplication() {
    let epub2_path = "samples/Alice in Wonderland - Lewis Carroll EPUB2.epub";
    let epub3_path = "samples/Alice in Wonderland - Lewis Carroll EPUB3.epub";

    let book2 = Book::from_file(epub2_path).expect("Should parse EPUB2");
    let book3 = Book::from_file(epub3_path).expect("Should parse EPUB3");

    let fp2 = book2.fingerprint();
    let fp3 = book3.fingerprint();

    assert!(!fp2.content_hash.is_empty());
    assert!(!fp3.content_hash.is_empty());

    let match_score = fp2.match_score(&fp3);
    println!(
        "Fingerprint match score between EPUB2 and EPUB3: {}",
        match_score
    );
    assert!(
        match_score > 0.5,
        "EPUB2 and EPUB3 of same book should have high similarity"
    );
}

#[test]
fn test_citation_exporter() {
    let epub_path = "samples/Alice in Wonderland - Lewis Carroll EPUB3.epub";
    let book = Book::from_file(epub_path).expect("Should parse EPUB3");

    let bibtex = book.to_bibtex();
    println!("BibTeX:\n{}", bibtex);
    assert!(bibtex.contains("@book{"));
    assert!(bibtex.contains("title"));

    let apa = book.to_apa();
    println!("APA: {}", apa);
    assert!(!apa.is_empty());

    let mla = book.to_mla();
    println!("MLA: {}", mla);
    assert!(!mla.is_empty());

    let chicago = book.to_chicago();
    println!("Chicago: {}", chicago);
    assert!(!chicago.is_empty());
}
