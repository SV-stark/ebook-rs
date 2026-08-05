use ebook_rs::Book;

#[test]
fn test_readium_webpub_manifest_export() {
    let path = "samples/Alice in Wonderland - Lewis Carroll EPUB2.epub";
    let book = Book::from_file(path).expect("Should load sample EPUB");

    let manifest = book.to_webpub_manifest();
    assert_eq!(manifest.metadata.title, "Alice in Wonderland");
    assert_eq!(
        manifest.context,
        "https://readium.org/webpub-manifest/context.jsonld"
    );
    assert!(
        !manifest.reading_order.is_empty(),
        "Reading order should contain chapters"
    );
    assert!(
        !manifest.toc.is_empty(),
        "TOC should contain navigation items"
    );

    let json = book.to_webpub_json().expect("Should serialize to JSON");
    assert!(json.contains("application/webpub+json"));
    assert!(json.contains("http://schema.org/Book"));
    assert!(json.contains("readingOrder"));
}
