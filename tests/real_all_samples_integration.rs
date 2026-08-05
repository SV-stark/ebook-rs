use ebook_rs::Book;
use std::fs;

#[test]
fn test_all_sample_files_parsing_and_verification() {
    let entries = fs::read_dir("samples").expect("samples directory should exist");
    let mut tested_count = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        let path_str = path.to_string_lossy();

        if path.is_file() {
            println!("🧪 Testing Real Sample File: {}", path_str);
            let book = Book::from_file(&path_str).expect(&format!("Should parse {}", path_str));

            // Verify Metadata
            assert!(
                !book.metadata().title.is_empty(),
                "Title should not be empty for {}",
                path_str
            );

            // Verify Spine
            assert!(
                !book.spine().is_empty(),
                "Spine items count > 0 for {}",
                path_str
            );

            // Verify Section 0 Plain Text
            let sec0 = book.get_section(0).expect("Section 0 should exist");
            assert!(
                sec0.char_count > 0 || !sec0.raw_html.is_empty(),
                "Section 0 content for {}",
                path_str
            );

            // Verify Locations Progress
            assert!(
                book.locations.total_locations > 0,
                "Total locations > 0 for {}",
                path_str
            );

            // Verify Search Capability
            let matches = book.search("Alice");
            println!("   - Search 'Alice': {} matches found", matches.len());

            tested_count += 1;
        }
    }

    assert!(
        tested_count >= 7,
        "Expected at least 7 sample files tested, got {}",
        tested_count
    );
}
