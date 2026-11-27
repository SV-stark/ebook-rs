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
    assert_eq!(cfi1.compare(&cfi2), std::cmp::Ordering::Less);

    // Step-by-step DOM path comparison: /4/2/1:5 < /4/4/1:0 even though offset 5 > 0
    let dom1 = Cfi::parse("epubcfi(/6/2!/4/2/1:5)").unwrap();
    let dom2 = Cfi::parse("epubcfi(/6/2!/4/4/1:0)").unwrap();
    assert_eq!(dom1.compare(&dom2), std::cmp::Ordering::Less);
}

#[test]
fn test_blackbox_locations_generation() {
    let md = "# Chapter 1\nThis is chapter 1 with some text content.\n\n# Chapter 2\nThis is chapter 2 with more text content.";
    let mut book = TxtBook::parse(md.as_bytes(), "Locations Test", true).unwrap();

    // Generate reading locations (1 location per 50 chars)
    book.generate_locations(50);
    assert!(book.locations().total_locations > 0);

    let loc1 = book.locations().location_from_char_offset(0, 5);
    assert!(loc1.is_some());
    assert_eq!(loc1.unwrap().location, 1);

    let pct = book.locations().percentage_from_location(1);
    assert!((0.0..=1.0).contains(&pct));
}

#[test]
fn test_smooth_progression_and_reverse_lookup() {
    let md = "# Chapter 1\n".to_string()
        + &"alpha beta gamma delta. ".repeat(30)
        + "\n# Chapter 2\n"
        + &"epsilon zeta eta theta. ".repeat(30);
    let mut book = TxtBook::parse(md.as_bytes(), "Smooth Progress Test", true).unwrap();
    assert!(book.sections().len() >= 2);

    book.generate_locations(100);
    let locs = book.locations();

    let p0 = locs.progression_from_char_offset(0, 0);
    assert_eq!(p0, 0.0);

    let last_idx = book.sections().len() - 1;
    let last_len = book.sections()[last_idx].char_count;
    let p_mid = locs.progression_from_char_offset(last_idx, last_len / 2);
    let p_end = locs.progression_from_char_offset(last_idx, last_len);
    assert!(p_mid > p0 && p_mid < p_end);
    assert!((p_end - 1.0).abs() < 1e-6);

    // CFI progression matches char offset progression
    let cfi_mid = Cfi::from_spine_index(last_idx, None, last_len / 2).to_string();
    assert!((locs.progression_from_cfi(&cfi_mid) - p_mid).abs() < 1e-9);
    assert_eq!(locs.progression_from_cfi("not-a-cfi"), 0.0);

    // Reverse lookup boundaries
    assert_eq!(locs.location_from_percentage(0.0).unwrap().location, 1);
    assert_eq!(
        locs.location_from_percentage(1.0).unwrap().location,
        locs.total_locations
    );
    assert_eq!(locs.position_from_progression(0.0), Some((0, 0)));
    assert_eq!(locs.position_from_progression(-0.5), Some((0, 0)));
    assert_eq!(locs.position_from_progression(f64::NAN), Some((0, 0)));
    let (end_sec, end_off) = locs.position_from_progression(1.5).unwrap();
    assert_eq!(end_sec, last_idx);
    assert_eq!(end_off, last_len);

    // Round-trip at 0.5
    let (mid_sec, mid_off) = locs.position_from_progression(0.5).unwrap();
    let roundtrip = locs.progression_from_char_offset(mid_sec, mid_off);
    assert!((roundtrip - 0.5).abs() < 0.02);
}

#[test]
fn test_book_progress_and_locator_without_generate_locations() {
    use ebook_rs::{Book, UniBook, generate_sample_epub};

    // 1. TxtBook without calling generate_locations()
    let md =
        "# Part One\n".to_string() + &"word ".repeat(200) + "\n# Part Two\n" + &"word ".repeat(200);
    let txt_book = TxtBook::parse(md.as_bytes(), "Progress Book", true).unwrap();

    let loc_start = txt_book.to_readium_locator(0, 0).unwrap();
    assert_eq!(loc_start.locations.total_progression, 0.0);

    let last_idx = txt_book.sections().len() - 1;
    let last_len = txt_book.sections()[last_idx].char_count;
    let loc_end = txt_book.to_readium_locator(last_idx, last_len).unwrap();
    assert!((loc_end.locations.total_progression - 1.0).abs() < 1e-6);

    let prog_start = txt_book.book_progress(0, 0, None).unwrap();
    assert_eq!(prog_start.words_per_minute, 200);
    assert_eq!(prog_start.book_progression, 0.0);
    assert_eq!(prog_start.chapter_progression, 0.0);
    assert!(prog_start.book_time_left_minutes >= prog_start.chapter_time_left_minutes);
    assert!(prog_start.book_time_left_minutes > 0.0);

    // Custom WPM and wpm = Some(0) clamping
    let prog_fast = txt_book.book_progress(0, 0, Some(400)).unwrap();
    assert_eq!(prog_fast.words_per_minute, 400);
    assert!(prog_fast.book_time_left_minutes <= prog_start.book_time_left_minutes);

    let prog_zero = txt_book.book_progress(0, 0, Some(0)).unwrap();
    assert_eq!(prog_zero.words_per_minute, 1);

    let total_time = txt_book.book_reading_time_minutes(None);
    assert!((total_time - prog_start.book_time_left_minutes).abs() < 0.2);

    // Reverse seek on Book
    let (seek_sec, seek_off) = txt_book.position_from_progression(0.5).unwrap();
    let seek_loc = txt_book.locator_from_progression(0.5).unwrap();
    assert!((seek_loc.locations.total_progression - 0.5).abs() < 0.05);
    assert!(seek_sec < txt_book.sections().len());
    assert!(seek_off <= txt_book.sections()[seek_sec].char_count);

    // 2. Eager EPUB via generate_sample_epub()
    let epub_bytes = generate_sample_epub().unwrap();
    let epub_book = Book::from_bytes(&epub_bytes).unwrap();
    let epub_prog = epub_book.book_progress(0, 0, None).unwrap();
    assert_eq!(epub_prog.spine_index, 0);
    assert!(epub_prog.total_locations >= 1);

    // 3. UniBook JSON progress methods
    let uni = UniBook::from_bytes(epub_bytes).unwrap();
    let locs_json = uni.get_locations_json().unwrap();
    assert!(locs_json.contains("total_locations"));
    let loc_json = uni.to_readium_locator_json(0, 0).unwrap();
    assert!(loc_json.contains("totalProgression"));
    let prog_json = uni.get_book_progress_json(0, 0, Some(250)).unwrap();
    assert!(prog_json.contains("\"words_per_minute\":250"));
    let rev_json = uni.locator_from_progression_json(0.5).unwrap();
    assert!(rev_json.contains("totalProgression"));
}
