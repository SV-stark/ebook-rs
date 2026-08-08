use ebook_rs::{Book, KFX_MAGIC_CONT, KfxBook, TxtBook, UniversalKfxExporter};

#[test]
fn test_blackbox_kfx_parsing_and_detection() {
    let raw_text = "title: \"Alice in KFX Wonderland\"\nauthor: \"Lewis Carroll\"\nlanguage: \"en\"\n\n# Chapter 1\nAlice was beginning to get very tired of sitting by her sister on the bank.";
    let txt_book = TxtBook::parse(raw_text.as_bytes(), "Alice in KFX Wonderland", true).unwrap();
    let sample_kfx =
        UniversalKfxExporter::export(&txt_book).expect("Exporting KFX container failed");

    // Verify KFX magic detection
    assert!(KfxBook::is_kfx(&sample_kfx));
    assert_eq!(&sample_kfx[0..4], KFX_MAGIC_CONT);

    // Parse into standard Book struct
    let book = Book::from_bytes(&sample_kfx).expect("KFX book parsing failed");
    assert_eq!(book.metadata().title, "Alice in KFX Wonderland");
    assert_eq!(book.metadata().creators[0], "Lewis Carroll");
    assert_eq!(book.metadata().language(), "en");
    assert!(!book.spine().is_empty());
}

#[test]
fn test_blackbox_kfx_conversion_and_export() {
    let raw_text = "# Kindle KFX Test\nThis is a clean-room test for KFX container export.";
    let txt_book = TxtBook::parse(raw_text.as_bytes(), "Kindle KFX Conversion Test", true).unwrap();
    let sample_kfx = UniversalKfxExporter::export(&txt_book).unwrap();

    let book = Book::from_bytes(&sample_kfx).unwrap();

    // Export any book to KFX container bytes
    let exported_kfx_bytes = UniversalKfxExporter::export(&book).expect("KFX export failed");
    assert!(KfxBook::is_kfx(&exported_kfx_bytes));

    // Export KFX book to EPUB 3 zip bytes
    let exported_epub3 = book.export_epub3_bytes().expect("EPUB3 export failed");
    assert!(exported_epub3.starts_with(b"PK\x03\x04"));
}
