use ebook_rs::TxtBook;
use ebook_rs::media_overlay::SmilClock;
use ebook_rs::validator::EpubValidator;

#[test]
fn test_blackbox_smil_clock_parsing() {
    assert_eq!(SmilClock::parse_npt_seconds("10s"), 10.0);
    assert_eq!(SmilClock::parse_npt_seconds("2.5s"), 2.5);
    assert_eq!(SmilClock::parse_npt_seconds("100ms"), 100.0);
    assert_eq!(SmilClock::parse_npt_seconds("01:30"), 90.0);
    assert_eq!(SmilClock::parse_npt_seconds("01:02:03"), 3723.0);
}

#[test]
fn test_blackbox_epub_structural_validator() {
    let md = "# Valid Book\n\nProperly formatted chapter text.";
    let book = TxtBook::parse(md.as_bytes(), "Valid Book", true).unwrap();

    let report = EpubValidator::validate(&book);
    assert!(report.is_valid);
    assert_eq!(report.errors_count, 0);
}
