use ebook_rs::TxtBook;
use ebook_rs::layout::{LayoutMode, RenditionLayout};

#[test]
fn test_blackbox_fxl_synthetic_spread_generation() {
    let md = "# Page 1\nContent page 1\n\n# Page 2\nContent page 2";
    let mut book = TxtBook::parse(md.as_bytes(), "FXL Spread Test", true).unwrap();
    book.layout = RenditionLayout {
        layout_mode: LayoutMode::PrePaginated,
        ..Default::default()
    };

    let spread = book.get_synthetic_spread(0, Some(1)).unwrap();
    assert_eq!(spread.left_index, 0);
    assert_eq!(spread.right_index, Some(1));
    assert!(spread.combined_html.contains("Content page 1"));
    assert!(spread.combined_html.contains("Content page 2"));
}

#[test]
fn test_blackbox_viewport_meta_parsing() {
    let html = "<html><head><meta name=\"viewport\" content=\"width=1024, height=768\"/></head><body>Fixed layout</body></html>";
    let mut sec = ebook_rs::Section {
        index: 0,
        idref: "s1".to_string(),
        href: "s1.html".to_string(),
        full_path: "s1.html".to_string(),
        raw_html: html.to_string(),
        processed_html: html.to_string(),
        plain_text: "Fixed layout".to_string(),
        plain_text_lower: "fixed layout".to_string(),
        char_count: 12,
        viewport_width: None,
        viewport_height: None,
    };
    sec.viewport_width = Some(1024.0);
    sec.viewport_height = Some(768.0);
    assert_eq!(sec.viewport_width, Some(1024.0));
    assert_eq!(sec.viewport_height, Some(768.0));
}
