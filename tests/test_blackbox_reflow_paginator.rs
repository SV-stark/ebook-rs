use ebook_rs::Section;
use ebook_rs::paginator::ReflowPaginator;

#[test]
fn test_blackbox_reflow_paginator_breaks() {
    let long_text = "Word ".repeat(500);
    let html = format!("<html><body><p>{}</p></body></html>", long_text);
    let sec = Section {
        index: 0,
        idref: "s1".to_string(),
        href: "s1.html".to_string(),
        full_path: "s1.html".to_string(),
        raw_html: html.clone(),
        processed_html: html,
        plain_text: long_text,
        plain_text_lower: "word ".repeat(500),
        char_count: 2500,
        viewport_width: None,
        viewport_height: None,
    };

    let paginator = ReflowPaginator::new(16, 1.5, 800, 600, 20);
    let paginated = paginator.paginate_section(&sec);
    assert!(paginated.total_pages > 0);
    assert!(!paginated.page_ranges.is_empty());
}
