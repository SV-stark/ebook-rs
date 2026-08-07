use ebook_rs::Section;

#[test]
fn test_blackbox_tts_word_synchronizer_token_spans() {
    let text = "Antigravity 2.0 Fast eBook Engine";
    let sec = Section {
        index: 0,
        idref: "s1".to_string(),
        href: "s1.html".to_string(),
        full_path: "s1.html".to_string(),
        raw_html: format!("<p>{}</p>", text),
        processed_html: format!("<p>{}</p>", text),
        plain_text: text.to_string(),
        plain_text_lower: text.to_lowercase(),
        char_count: text.chars().count(),
        viewport_width: None,
        viewport_height: None,
    };

    let tokens = sec.tokenize_tts_words();
    assert!(!tokens.is_empty());
    assert_eq!(tokens[0].word, "Antigravity");
    assert_eq!(tokens[0].char_start, 0);
    assert_eq!(tokens[0].char_end, 11);

    // Multibyte character boundary test
    let unicode_text = "電子 書籍";
    let u_sec = Section {
        index: 0,
        idref: "s2".to_string(),
        href: "s2.html".to_string(),
        full_path: "s2.html".to_string(),
        raw_html: format!("<p>{}</p>", unicode_text),
        processed_html: format!("<p>{}</p>", unicode_text),
        plain_text: unicode_text.to_string(),
        plain_text_lower: unicode_text.to_lowercase(),
        char_count: unicode_text.chars().count(),
        viewport_width: None,
        viewport_height: None,
    };

    let u_tokens = u_sec.tokenize_tts_words();
    assert_eq!(u_tokens.len(), 2);
    assert_eq!(u_tokens[0].word, "電子");
    assert_eq!(u_tokens[0].char_start, 0);
    assert_eq!(u_tokens[0].char_end, 2);
}
