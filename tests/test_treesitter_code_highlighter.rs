use ebook_rs::{TreeSitterEngine, TxtBook};

#[test]
fn test_treesitter_code_snippet_parser() {
    let rust_code = "pub fn main() {\n    // Print hello world\n    println!(\"Hello\");\n}";
    let nodes = TreeSitterEngine::parse_code(rust_code, "rust");

    assert!(!nodes.is_empty());
    assert!(nodes.iter().any(|n| n.kind == "function_definition"));
    assert!(nodes.iter().any(|n| n.kind == "comment"));
}

#[test]
fn test_treesitter_extract_code_blocks_from_markdown() {
    let md_content = "# Rust Programming Guide\n\nHere is an example function:\n\n<pre><code class=\"language-rust\">\npub fn calculate_hash() -> u64 {\n    // calculate\n    42\n}\n</code></pre>";
    let book = TxtBook::parse(md_content.as_bytes(), "Rust Guide", true)
        .expect("Markdown codebook should parse");

    let code_blocks = book.extract_code_blocks();
    assert_eq!(code_blocks.len(), 1, "Should extract 1 embedded code block");

    let block = &code_blocks[0];
    assert_eq!(block.language, "rust");
    assert!(block.code.contains("calculate_hash"));
    assert!(!block.ast_nodes.is_empty());
}

#[test]
fn test_treesitter_highlight_code_blocks_html() {
    let raw_html = "<p>Sample snippet:</p><pre><code>fn test() {}</code></pre>";
    let highlighted = TreeSitterEngine::highlight_code_blocks(raw_html);

    assert!(highlighted.contains("treesitter-highlighted"));
}
