use ebook_rs::section::{extract_plain_text, find_tag_end};

#[test]
fn test_find_tag_end_ignores_quotes() {
    let html = "<img src=\"test.jpg\" alt=\"x > y\" title=\"A > B\" /> trailing text";
    let end_idx = find_tag_end(html, 0).expect("Should find true tag end");
    let tag_str = &html[0..=end_idx];

    assert_eq!(
        tag_str,
        "<img src=\"test.jpg\" alt=\"x > y\" title=\"A > B\" />"
    );
}

#[test]
fn test_extract_plain_text_no_stray_brackets() {
    let html = "<p title=\"5 > 3\">Hello world!</p><img src=\"a.png\" alt=\"X > Y\" /><br />Text after break";
    let text = extract_plain_text(html);

    assert_eq!(text, "Hello world! Text after break");
    assert!(
        !text.contains('>'),
        "Extracted text should not contain stray '>' brackets"
    );
}

#[test]
fn test_parse_viewport_meta_black_box_contract() {
    use ebook_rs::section::parse_viewport_meta;

    // 1. Standard viewport with width and height
    let html1 = r#"<html><head><meta name="viewport" content="width=1024, height=768"></head><body></body></html>"#;
    assert_eq!(parse_viewport_meta(html1), (Some(1024.0), Some(768.0)));

    // 2. Viewport with only width
    let html2 = r#"<meta name="viewport" content="width=800">"#;
    assert_eq!(parse_viewport_meta(html2), (Some(800.0), None));

    // 3. Viewport with only height
    let html3 = r#"<meta name="viewport" content="height=600">"#;
    assert_eq!(parse_viewport_meta(html3), (None, Some(600.0)));

    // 4. Case-insensitive tags and attributes
    let html4 = r#"<META NAME="VIEWPORT" CONTENT="WIDTH=1920, HEIGHT=1080">"#;
    assert_eq!(parse_viewport_meta(html4), (Some(1920.0), Some(1080.0)));

    // 5. HTML without viewport tag
    let html5 = r#"<html><head><title>No Viewport</title></head></html>"#;
    assert_eq!(parse_viewport_meta(html5), (None, None));

    // 6. Empty HTML input
    assert_eq!(parse_viewport_meta(""), (None, None));

    // 7. Non-viewport meta tag containing width string (should be ignored)
    let html7 = r#"<meta name="description" content="width=1200, height=800">"#;
    assert_eq!(parse_viewport_meta(html7), (None, None));

    // 8. Viewport tag with malformed non-numeric values
    let html8 = r#"<meta name="viewport" content="width=invalid, height=bad">"#;
    assert_eq!(parse_viewport_meta(html8), (None, None));
}
