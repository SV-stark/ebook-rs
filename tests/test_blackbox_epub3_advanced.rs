use ebook_rs::TxtBook;
use ebook_rs::media_overlay::SmilClock;
use ebook_rs::validator::EpubValidator;

#[test]
fn test_blackbox_smil_clock_parsing() {
    assert_eq!(SmilClock::parse_npt_seconds("10s"), 10.0);
    assert_eq!(SmilClock::parse_npt_seconds("2.5s"), 2.5);
    // 100 ms is one tenth of a second. (This assertion previously bundled the `m`/`ms` bug: the
    // old parser read `100ms` as 100 *minutes*' worth of "milliseconds" and returned 100.0.)
    assert_eq!(SmilClock::parse_npt_seconds("100ms"), 0.1);
    assert_eq!(SmilClock::parse_npt_seconds("250ms"), 0.25);
    assert_eq!(SmilClock::parse_npt_seconds("90min"), 5400.0);
    assert_eq!(SmilClock::parse_npt_seconds("1.5m"), 90.0);
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
