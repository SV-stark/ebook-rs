use ebook_rs::{Book, generate_sample_epub};

#[test]
fn test_full_text_search() {
    let bytes = generate_sample_epub().unwrap();
    let book = Book::from_bytes(&bytes).unwrap();

    // 1. Search existing term
    let results = book.search("Canonical");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].spine_index, 1);
    assert!(results[0].snippet.contains("Canonical"));
    assert!(results[0].cfi.starts_with("epubcfi(/6/4!"));

    // 2. Case-insensitive search
    let results_lower = book.search("canonical");
    assert_eq!(results_lower.len(), 1);

    // 3. Search non-existent term
    let no_results = book.search("nonexistentword123");
    assert!(no_results.is_empty());

    // 4. Empty query
    let empty_results = book.search("");
    assert!(empty_results.is_empty());
}
