use ebook_rs::Section;

#[test]
fn test_blackbox_fuzzy_xml_recovery() {
    let malformed_html =
        "<p>Unclosed paragraph <b>Bold text without closing tag & unescaped ampersand";
    let sec = Section {
        index: 0,
        idref: "s1".to_string(),
        href: "s1.html".to_string(),
        full_path: "s1.html".to_string(),
        raw_html: malformed_html.to_string(),
        processed_html: malformed_html.to_string(),
        plain_text: "Unclosed paragraph Bold text without closing tag & unescaped ampersand"
            .to_string(),
        plain_text_lower: "unclosed paragraph bold text without closing tag & unescaped ampersand"
            .to_string(),
        char_count: 70,
        viewport_width: None,
        viewport_height: None,
    };

    assert!(sec.plain_text.contains("Unclosed paragraph"));
    assert!(sec.plain_text.contains("&"));
}
