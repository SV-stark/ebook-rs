use ebook_rs::{AnnotationManager, AnnotationType};

#[test]
fn test_annotations_crud_and_serialization() {
    let mut manager = AnnotationManager::new();

    // Create highlight
    let h = manager.create_highlight(
        "epubcfi(/6/2!/4/2:0)",
        "#ff0000",
        Some("Sample text"),
        Some("Note 1"),
    );
    assert_eq!(h.type_, AnnotationType::Highlight);
    assert_eq!(h.color, "#ff0000");

    // Create bookmark
    let b = manager.create_bookmark("epubcfi(/6/4!/2:0)", Some("Page 10"));
    assert_eq!(b.type_, AnnotationType::Bookmark);

    // List all
    let list = manager.list();
    assert_eq!(list.len(), 2);

    // Remove
    let removed = manager.remove(&h.id);
    assert!(removed);
    assert_eq!(manager.list().len(), 1);

    // JSON serialization
    let json = serde_json::to_string(&manager).expect("Should serialize annotations");
    let restored: AnnotationManager =
        serde_json::from_str(&json).expect("Should deserialize annotations");
    assert_eq!(restored.list().len(), 1);
}
