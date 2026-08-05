use ebook_rs::Cfi;
use std::cmp::Ordering;

#[test]
fn test_cfi_parsing_simple() {
    let raw = "epubcfi(/6/2)";
    let cfi = Cfi::parse(raw).expect("Should parse simple CFI");
    assert_eq!(cfi.spine_index(), 0); // /6/2 -> index 0
    assert_eq!(cfi.char_offset(), 0);
    assert_eq!(cfi.to_string(), raw);
}

#[test]
fn test_cfi_parsing_complex_with_assertions() {
    let raw = "epubcfi(/6/4[chap01ref]!/4/2/10/1:5)";
    let cfi = Cfi::parse(raw).expect("Should parse complex CFI");
    assert_eq!(cfi.spine_index(), 1); // step 4 -> index 1
    assert_eq!(cfi.char_offset(), 5);
    assert_eq!(
        cfi.spine_path.steps[1].id_assertion,
        Some("chap01ref".to_string())
    );
    assert_eq!(cfi.to_string(), raw);
}

#[test]
fn test_cfi_parsing_range() {
    let raw = "epubcfi(/6/4!/4,/2/1:0,/2/1:5)";
    let cfi = Cfi::parse(raw).expect("Should parse range CFI");
    assert_eq!(cfi.spine_index(), 1);
    assert!(cfi.range_start.is_some());
    assert!(cfi.range_end.is_some());
}

#[test]
fn test_cfi_from_spine_index() {
    let cfi = Cfi::from_spine_index(2, Some("sec3"), 42);
    assert_eq!(cfi.spine_index(), 2);
    assert_eq!(cfi.char_offset(), 42);
    assert_eq!(
        cfi.spine_path.steps[1].id_assertion,
        Some("sec3".to_string())
    );
}

#[test]
fn test_cfi_comparison() {
    let cfi1 = Cfi::from_spine_index(0, None, 10);
    let cfi2 = Cfi::from_spine_index(0, None, 50);
    let cfi3 = Cfi::from_spine_index(1, None, 5);

    assert_eq!(cfi1.compare(&cfi2), Ordering::Less);
    assert_eq!(cfi2.compare(&cfi1), Ordering::Greater);
    assert_eq!(cfi1.compare(&cfi3), Ordering::Less);
    assert_eq!(cfi3.compare(&cfi2), Ordering::Greater);
}
