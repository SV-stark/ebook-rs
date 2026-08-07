use ebook_rs::{Cfi, TxtBook};

#[test]
fn test_blackbox_cfi_parsing_and_formatting() {
    let cfi_str = "epubcfi(/6/4[chap01.xhtml]!/4/2/10:5)";
    let cfi = Cfi::parse(cfi_str).expect("CFI string should parse cleanly");

    assert_eq!(cfi.spine_index(), 1); // (6 / 2) - 1 = 1
    assert!(cfi.raw.contains("chap01.xhtml"));

    // Verify roundtrip formatting
    let formatted = cfi.to_string();
    assert!(formatted.contains("/6/4[chap01.xhtml]!"));
    assert!(formatted.contains(":5"));

    // Builder from spine index
    let built = Cfi::from_spine_index(0, Some("sec-1"), 42);
    assert_eq!(built.spine_index(), 0);
    assert!(built.to_string().contains(":42"));
}

#[test]
fn test_blackbox_cfi_comparison_and_ordering() {
    let cfi1 = Cfi::parse("epubcfi(/6/2!/4/2:10)").unwrap();
    let cfi2 = Cfi::parse("epubcfi(/6/2!/4/2:20)").unwrap();

    assert_ne!(cfi1.to_string(), cfi2.to_string());
    assert!(cfi1.to_string().contains(":10"));
    assert!(cfi2.to_string().contains(":20"));
}

#[test]
fn test_blackbox_locations_generation() {
    let md = "# Chapter 1\nThis is chapter 1 with some text content.\n\n# Chapter 2\nThis is chapter 2 with more text content.";
    let mut book = TxtBook::parse(md.as_bytes(), "Locations Test", true).unwrap();

    // Generate reading locations (1 location per 50 chars)
    book.generate_locations(50);
    assert!(book.locations.total_locations > 0);

    let loc1 = book.locations.location_from_char_offset(0, 5);
    assert!(loc1.is_some());
    assert_eq!(loc1.unwrap().location, 1);

    let pct = book.locations.percentage_from_location(1);
    assert!((0.0..=1.0).contains(&pct));
}
