use ebook_rs::{Section, TreeSitterEngine, TxtBook};

#[test]
fn test_blackbox_treesitter_code_snippet_parser() {
    let rust_code = "pub fn main() {\n    println!(\"Hello World\");\n}";
    let nodes = TreeSitterEngine::parse_code(rust_code, "rust");
    assert!(!nodes.is_empty());
    assert!(nodes.iter().any(|n| n.kind == "function_definition"));
}

#[test]
fn test_blackbox_treesitter_code_block_extraction() {
    let md = "# Code Guide\n\n```rust\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n```";
    let book = TxtBook::parse(md.as_bytes(), "Code Guide", true).unwrap();

    let blocks = book.extract_code_blocks();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].language, "rust");
    assert!(blocks[0].code.contains("add"));
    assert!(!blocks[0].ast_nodes.is_empty());
}

#[test]
fn test_blackbox_script_sanitization() {
    let unsafe_html =
        "<html><body><script>alert('xss')</script><h1>Title</h1><p>Text</p></body></html>";
    let sec = Section {
        index: 0,
        idref: "sec1".to_string(),
        href: "sec1.html".to_string(),
        full_path: "sec1.html".to_string(),
        raw_html: unsafe_html.to_string(),
        processed_html: unsafe_html.to_string(),
        plain_text: "Title Text".to_string(),
        plain_text_lower: "title text".to_string(),
        char_count: 10,
        viewport_width: None,
        viewport_height: None,
    };

    sec.to_tts_annotated_html();
    assert!(sec.plain_text.contains("Title Text"));
}

#[test]
fn test_blackbox_cjk_multibyte_utf8_handling() {
    let cjk_html = "<html><body><h1>電子書籍</h1><p>Antigravity 2.0 高速 Rust eBook ライブラリ</p></body></html>";
    let sec = Section {
        index: 0,
        idref: "sec1".to_string(),
        href: "sec1.html".to_string(),
        full_path: "sec1.html".to_string(),
        raw_html: cjk_html.to_string(),
        processed_html: cjk_html.to_string(),
        plain_text: "電子書籍 Antigravity 2.0 高速 Rust eBook ライブラリ".to_string(),
        plain_text_lower: "電子書籍 antigravity 2.0 高速 rust ebook ライブラリ".to_string(),
        char_count: 36,
        viewport_width: None,
        viewport_height: None,
    };

    let tokens = sec.tokenize_tts_words();
    assert!(!tokens.is_empty());
    assert_eq!(tokens[0].word, "電子書籍");
}
