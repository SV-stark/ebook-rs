use ebook_rs::section::{extract_plain_text, find_tag_end};

#[test]
fn test_find_tag_end_ignores_quotes() {
    let html = "<img src=\"test.jpg\" alt=\"x > y\" title=\"A > B\" /> trailing text";
    let end_idx = find_tag_end(html, 0).expect("Should find true tag end");
    let tag_str = &html[0..=end_idx];

    assert_eq!(tag_str, "<img src=\"test.jpg\" alt=\"x > y\" title=\"A > B\" />");
}

#[test]
fn test_extract_plain_text_no_stray_brackets() {
    let html = "<p title=\"5 > 3\">Hello world!</p><img src=\"a.png\" alt=\"X > Y\" /><br />Text after break";
    let text = extract_plain_text(html);

    assert_eq!(text, "Hello world! Text after break");
    assert!(!text.contains('>'), "Extracted text should not contain stray '>' brackets");
}
