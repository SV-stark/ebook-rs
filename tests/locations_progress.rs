use ebook_rs::Locations;

#[test]
fn test_locations_generation_and_mapping() {
    let mut locs = Locations::new(20);

    let text1 = "Hello world! This is section 1 of the book with some words.";
    let text2 = "Section 2 contains even more text for testing location chunking.";

    locs.add_spine_section(0, text1);
    locs.add_spine_section(1, text2);
    locs.finalize();

    assert!(locs.total_locations > 0);
    assert_eq!(
        locs.total_characters,
        text1.chars().count() + text2.chars().count()
    );

    // Test location 1 CFI lookup
    let first_cfi = locs.cfi_from_location(1).expect("Location 1 CFI");
    assert!(first_cfi.starts_with("epubcfi("));

    // Test reverse mapping
    let mapped = locs.location_from_cfi(&first_cfi).expect("Mapped entry");
    assert_eq!(mapped.location, 1);
    assert_eq!(mapped.spine_index, 0);

    // Test percentage progress
    let pct0 = locs.percentage_from_cfi(&first_cfi);
    assert!(pct0 > 0.0);

    let last_cfi = locs
        .cfi_from_location(locs.total_locations)
        .expect("Last CFI");
    let pct_last = locs.percentage_from_cfi(&last_cfi);
    assert!(pct_last > 0.5);
}

#[test]
fn test_locations_edge_cases() {
    let mut locs = Locations::new(50);
    locs.add_spine_section(0, "");
    locs.finalize();

    assert_eq!(locs.total_locations, 1);
    assert_eq!(locs.total_characters, 0);

    let cfi = locs.cfi_from_location(1);
    assert!(cfi.is_some());
}
