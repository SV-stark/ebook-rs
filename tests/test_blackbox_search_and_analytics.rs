use ebook_rs::{ReadingAnalytics, SearchEngine, TxtBook};

#[test]
fn test_blackbox_full_text_and_regex_search() {
    let content = "# Sherlock Holmes\n\nIt was a dark and stormy night. Holmes picked up his pipe.\n\n# The Clue\nWatson found another clue near the fireplace.";
    let book = TxtBook::parse(content.as_bytes(), "Sherlock", true).unwrap();

    // Exact string search
    let matches = book.search("Holmes");
    assert_eq!(matches.len(), 2); // Title + paragraph match
    assert_eq!(matches[0].spine_index, 0);

    // Regex search
    let regex_matches = book.search_regex(r"(?i)watson|fireplace").unwrap();
    assert!(!regex_matches.is_empty());
    assert_eq!(regex_matches[0].spine_index, 1);

    // Case-insensitive match check
    let case_matches = book.search("DARK");
    assert_eq!(case_matches.len(), 1);
    assert!(!case_matches[0].snippet.is_empty());
}

#[test]
fn test_blackbox_readium_search_json_export() {
    let content = "# Search Test\nFinding Alice in Wonderland.";
    let book = TxtBook::parse(content.as_bytes(), "Search Test", true).unwrap();
    let results = book.search("Alice");
    assert_eq!(results.len(), 1);

    let json_str = SearchEngine::to_readium_search_json(&results, "Alice").unwrap();
    assert!(json_str.contains("Alice"));
    assert!(json_str.contains("locators") || json_str.contains("loc"));
}

#[test]
fn test_blackbox_reading_analytics_nlp() {
    let md = "# Analytics Test\n\nThis is a sample book section with 12 words to measure reading time and word count.";
    let book = TxtBook::parse(md.as_bytes(), "Analytics Test", true).unwrap();

    let analytics = ReadingAnalytics::analyze_text(&book.sections[0].plain_text);
    assert!(analytics.word_count >= 10);
    assert!(analytics.reading_time_minutes >= 0.0);
}
