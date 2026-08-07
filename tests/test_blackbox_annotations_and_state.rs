use ebook_rs::TxtBook;
use ebook_rs::annotations::{Annotation, AnnotationManager, AnnotationType};

#[test]
fn test_blackbox_annotations_crud() {
    let mut manager = AnnotationManager::default();

    let ann = Annotation {
        id: "ann-1".to_string(),
        cfi_range: "epubcfi(/6/2!/4/2:10)".to_string(),
        selected_text: Some("sample selection".to_string()),
        note: Some("Note on chapter 1".to_string()),
        color: "#ff0000".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        type_: AnnotationType::Highlight,
    };

    manager.add(ann);
    assert_eq!(manager.list().len(), 1);
    assert_eq!(
        manager.get("ann-1").unwrap().selected_text,
        Some("sample selection".to_string())
    );

    // W3C Web Annotation JSON Export
    let w3c_json = manager.to_w3c_json().unwrap();
    assert!(w3c_json.contains("\"@context\":\"http://www.w3.org/ns/anno.jsonld\""));

    // Delete
    assert!(manager.remove("ann-1"));
    assert_eq!(manager.list().len(), 0);
}

#[test]
fn test_blackbox_zstd_state_caching() {
    let md = "# Cache Test\nSection 1 text.\n\n# Chapter 2\nSection 2 text.";
    let book = TxtBook::parse(md.as_bytes(), "Cache Test", true).unwrap();

    let compressed_zstd = book.export_zstd_cache().expect("Zstd compression error");
    assert!(!compressed_zstd.is_empty());

    let restored =
        ebook_rs::Book::from_zstd_cache(&compressed_zstd).expect("Zstd decompression error");
    assert!(!restored.metadata().title.is_empty());
    assert_eq!(restored.sections.len(), 2);
    assert_eq!(restored.sections[0].plain_text, book.sections[0].plain_text);
}

#[test]
fn test_blackbox_book_fingerprint() {
    let md = "# Fingerprint Test\nUnique content for SHA-256 fingerprint hash verification.";
    let book1 = TxtBook::parse(md.as_bytes(), "Book A", true).unwrap();
    let book2 = TxtBook::parse(md.as_bytes(), "Book B", true).unwrap();

    let fp1 = book1.fingerprint();
    let fp2 = book2.fingerprint();

    assert!(!fp1.content_hash.is_empty());
    assert!(!fp2.content_hash.is_empty());
}
