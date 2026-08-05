use ebook_rs::{generate_sample_epub, Book, Cfi, FlowMode, RenditionLayout, SpreadMode, Theme};

#[test]
fn test_rendition_spread_and_flow_modes() {
    let mut layout = RenditionLayout::default();

    // Test Double Spread Mode (Feature 4)
    layout.spread_mode = SpreadMode::Double;
    assert_eq!(layout.spread_mode, SpreadMode::Double);

    // Test Continuous Scroll Mode (Feature 6)
    layout.flow_mode = FlowMode::Scrolled;
    assert_eq!(layout.flow_mode, FlowMode::Scrolled);

    // Test CSS Output for layout
    layout.theme = Theme::Dark;
    let css = layout.to_css_override();
    assert!(css.contains("--reader-bg: #0f172a"));
}

#[test]
fn test_live_dom_cfi_selection_bridge() {
    let bytes = generate_sample_epub().unwrap();
    let mut book = Book::from_bytes(&bytes).unwrap();

    // Simulate Live DOM Selection CFI Range: epubcfi(/6/2!/4/2/1,:10,:25)
    let selection_cfi_range = "epubcfi(/6/2!/4/2/1,:10,:25)";
    let parsed_cfi = Cfi::parse(selection_cfi_range).expect("Should parse selection CFI range");
    assert!(parsed_cfi.range_start.is_some());
    assert!(parsed_cfi.range_end.is_some());

    // Create highlight from selection
    let ann = book.annotations.create_highlight(
        selection_cfi_range,
        "#bbf7d0",
        Some("Live selected text"),
        Some("Note from live selection"),
    );

    assert_eq!(ann.color, "#bbf7d0");
    assert_eq!(ann.selected_text, Some("Live selected text".to_string()));
}
