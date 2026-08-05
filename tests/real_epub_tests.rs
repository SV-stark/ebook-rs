use ebook_rs::{AnnotationType, Book, Cfi, Theme};
use std::fs;

fn find_sample_epub() -> Option<String> {
    let entries = fs::read_dir("samples").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "epub") {
            return Some(path.to_string_lossy().to_string());
        }
    }
    None
}

#[test]
fn test_real_epub_sample_features() {
    let sample_path =
        find_sample_epub().expect("Sample EPUB file must exist in samples/ directory");
    println!("🧪 Testing library against real EPUB: {}", sample_path);

    // 1. Load real EPUB file
    let mut book = Book::from_file(&sample_path).expect("Should parse real sample EPUB");

    // 2. Test Metadata Extraction
    let meta = book.metadata();
    assert!(!meta.title.is_empty(), "Book title should not be empty");
    assert!(
        !meta.creators.is_empty(),
        "Book creators should not be empty"
    );

    // 3. Test Manifest & Spine Structure
    assert!(!book.spine().is_empty(), "Spine items should not be empty");

    // 4. Test Cover Image Extraction
    if let Some((cover_bytes, cover_mime)) = book.cover_image() {
        assert!(!cover_bytes.is_empty());
        assert!(cover_mime.starts_with("image/") || cover_mime == "application/xhtml+xml");
    }

    // 5. Test Section Loading & Resource Inlining (Base64)
    let sec0 = book.get_section(0).expect("Section 0 should exist");
    assert!(!sec0.processed_html.is_empty());

    // 6. Test CFI Engine on Real EPUB
    let cfi_sec0 = Cfi::from_spine_index(0, None, 0).to_string();
    let parsed0 = Cfi::parse(&cfi_sec0).expect("Should parse CFI sec0");
    assert_eq!(parsed0.spine_index(), 0);

    let sec_from_cfi = book
        .get_section_by_cfi(&cfi_sec0)
        .expect("Section from CFI");
    assert_eq!(sec_from_cfi.index, 0);

    // 7. Test Locations & Progress Mapping
    assert!(book.locations.total_locations > 0);
    let loc1_cfi = book.locations.cfi_from_location(1).expect("Location 1 CFI");
    let loc1_entry = book
        .locations
        .location_from_cfi(&loc1_cfi)
        .expect("Location entry");
    assert_eq!(loc1_entry.spine_index, 0);

    let pct = book.locations.percentage_from_cfi(&loc1_cfi);
    assert!((0.0..=1.0).contains(&pct));

    // 8. Test Annotations Engine
    let ann = book.annotations.create_highlight(
        &cfi_sec0,
        "#fde047",
        Some("Selected Text"),
        Some("Real book note"),
    );
    assert_eq!(ann.type_, AnnotationType::Highlight);
    assert_eq!(book.annotations.list().len(), 1);

    // 9. Test Rendition Layout Theme Generation
    book.layout.theme = Theme::Dark;
    let css = book.layout.to_css_override();
    assert!(css.contains("--reader-bg: #0f172a"));
}
