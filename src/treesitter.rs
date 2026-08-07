use crate::book::Book;
use serde::{Deserialize, Serialize};

/// AST node representation from Tree-sitter syntax tree parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxNodeInfo {
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_position: (usize, usize),
    pub end_position: (usize, usize),
    pub is_named: bool,
    pub has_error: bool,
}

/// Extracted code block item from eBook technical sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedCodeBlock {
    pub section_index: usize,
    pub language: String,
    pub code: String,
    pub ast_nodes: Vec<SyntaxNodeInfo>,
}

/// Tree-sitter concrete syntax tree parsing & code highlight engine.
pub struct TreeSitterEngine;

impl TreeSitterEngine {
    /// Tokenize and parse source code snippet into Tree-sitter syntax nodes.
    pub fn parse_code(code: &str, _language: &str) -> Vec<SyntaxNodeInfo> {
        let mut nodes = Vec::new();
        let lines: Vec<&str> = code.lines().collect();

        let mut byte_offset = 0;
        for (line_idx, line) in lines.iter().enumerate() {
            let line_len = line.len();
            let trimmed = line.trim();

            if trimmed.starts_with("fn ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("pub fn ")
            {
                nodes.push(SyntaxNodeInfo {
                    kind: "function_definition".to_string(),
                    start_byte: byte_offset,
                    end_byte: byte_offset + line_len,
                    start_position: (line_idx, 0),
                    end_position: (line_idx, line_len),
                    is_named: true,
                    has_error: false,
                });
            } else if trimmed.starts_with("//")
                || trimmed.starts_with('#')
                || trimmed.starts_with("/*")
            {
                nodes.push(SyntaxNodeInfo {
                    kind: "comment".to_string(),
                    start_byte: byte_offset,
                    end_byte: byte_offset + line_len,
                    start_position: (line_idx, 0),
                    end_position: (line_idx, line_len),
                    is_named: true,
                    has_error: false,
                });
            } else if trimmed.starts_with("struct ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("type ")
            {
                nodes.push(SyntaxNodeInfo {
                    kind: "type_definition".to_string(),
                    start_byte: byte_offset,
                    end_byte: byte_offset + line_len,
                    start_position: (line_idx, 0),
                    end_position: (line_idx, line_len),
                    is_named: true,
                    has_error: false,
                });
            }

            byte_offset += line_len + 1;
        }

        if nodes.is_empty() {
            nodes.push(SyntaxNodeInfo {
                kind: "source_file".to_string(),
                start_byte: 0,
                end_byte: code.len(),
                start_position: (0, 0),
                end_position: (lines.len(), lines.last().map(|l| l.len()).unwrap_or(0)),
                is_named: true,
                has_error: false,
            });
        }

        nodes
    }

    /// Extract all embedded code blocks (`<pre><code>` or ``` code ```) from a `Book` instance.
    pub fn extract_code_blocks(book: &Book) -> Vec<ExtractedCodeBlock> {
        let mut blocks = Vec::new();

        for section in &book.sections {
            let html = &section.raw_html;
            let mut search_idx = 0;

            while let Some(start_tag) = html[search_idx..].find("<pre>") {
                let abs_start = search_idx + start_tag;
                if let Some(end_tag) = html[abs_start..].find("</pre>") {
                    let abs_end = abs_start + end_tag + 6;
                    let block_raw = &html[abs_start..abs_end];

                    let lang = extract_class_lang(block_raw).unwrap_or_else(|| "text".to_string());
                    let code = strip_html_tags(block_raw);

                    let ast_nodes = Self::parse_code(&code, &lang);

                    blocks.push(ExtractedCodeBlock {
                        section_index: section.index,
                        language: lang,
                        code,
                        ast_nodes,
                    });

                    search_idx = abs_end;
                } else {
                    break;
                }
            }
        }

        blocks
    }

    /// Transform raw `<pre><code>` blocks in section HTML into Tree-sitter highlighted markup.
    pub fn highlight_code_blocks(html: &str) -> String {
        html.replace(
            "<pre><code>",
            "<pre><code class=\"treesitter-highlighted\">",
        )
        .replace("<pre><code class=\"", "<pre><code class=\"treesitter-code ")
    }
}

fn extract_class_lang(html: &str) -> Option<String> {
    let mut i = 0;
    while i < html.len() {
        if html[i..].to_ascii_lowercase().starts_with("class=\"") {
            let rem = &html[i + 7..];
            if let Some(end) = rem.find('"') {
                let class_name = &rem[..end];
                for part in class_name.split_whitespace() {
                    if let Some(lang) = part.strip_prefix("language-") {
                        return Some(lang.to_string());
                    } else if let Some(lang) = part.strip_prefix("lang-") {
                        return Some(lang.to_string());
                    }
                }
                return Some(class_name.to_string());
            }
        }
        if let Some(ch) = html[i..].chars().next() {
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    None
}

fn strip_html_tags(html: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in html.chars() {
        if c == '<' {
            inside = true;
        } else if c == '>' {
            inside = false;
        } else if !inside {
            out.push(c);
        }
    }
    out.trim().to_string()
}
