use crate::book::Book;
use serde::{Deserialize, Serialize};

/// Syntax node representation from code block parsing.
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

/// Pure-Rust lightweight pattern-based code block syntax extractor & highlighter engine.
pub struct SyntaxHighlightEngine;

/// Backwards compatibility alias for `SyntaxHighlightEngine`.
pub type TreeSitterEngine = SyntaxHighlightEngine;

impl SyntaxHighlightEngine {
    /// Tokenize source code snippet into line-based syntax nodes with basic structural error checking.
    pub fn parse_code(code: &str, _language: &str) -> Vec<SyntaxNodeInfo> {
        let mut nodes = Vec::new();
        let lines: Vec<&str> = code.lines().collect();

        // Check for unbalanced or mismatched delimiters using stack validation
        let has_syntax_mismatch = {
            let mut stack = Vec::new();
            let mut mismatch = false;
            for c in code.chars() {
                match c {
                    '(' | '{' | '[' => stack.push(c),
                    ')' => {
                        if stack.pop() != Some('(') {
                            mismatch = true;
                            break;
                        }
                    }
                    '}' => {
                        if stack.pop() != Some('{') {
                            mismatch = true;
                            break;
                        }
                    }
                    ']' => {
                        if stack.pop() != Some('[') {
                            mismatch = true;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            mismatch || !stack.is_empty()
        };

        let mut byte_offset = 0;
        for (line_idx, line) in lines.iter().enumerate() {
            // A line's content length. The terminator is accounted for separately below: for a
            // CRLF pair it is two bytes, but a fixed `+ 1` assumed one, drifting every later
            // offset one byte left.
            let line_len = line.len();
            let trimmed = line.trim();
            // `str::lines()` strips the terminator, so read it back from the original text. A lone
            // `\n` is one byte and a `\r\n` pair is two; `lines()` also strips the `\r` of a CRLF
            // pair, which is what made a fixed `+ 1` drift every later offset.
            let term_len = code[byte_offset + line_len..]
                .bytes()
                .take_while(|b| *b == b'\n' || *b == b'\r')
                .count();

            let line_mismatch = {
                let mut l_stack = Vec::new();
                let mut l_err = false;
                for c in line.chars() {
                    match c {
                        '(' | '{' | '[' => l_stack.push(c),
                        ')' => {
                            if l_stack.pop() != Some('(') {
                                l_err = true;
                                break;
                            }
                        }
                        '}' => {
                            if l_stack.pop() != Some('{') {
                                l_err = true;
                                break;
                            }
                        }
                        ']' => {
                            if l_stack.pop() != Some('[') {
                                l_err = true;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                l_err || (has_syntax_mismatch && !l_stack.is_empty())
            };
            let line_err = line_mismatch
                || (has_syntax_mismatch
                    && (trimmed.contains('(') || trimmed.contains('{') || trimmed.contains(')')));

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
                    has_error: line_err,
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

            // `str::lines()` strips the `\r` of a `\r\n` pair, so `line_len` is one byte short for
            // every such line. Advancing by `line_len + 1` therefore drifted every subsequent
            // offset one byte left.
            byte_offset += line_len + term_len;
        }

        if nodes.is_empty() {
            let row = lines.len().saturating_sub(1);
            let col = lines.last().map_or(0, |l| l.chars().count());
            nodes.push(SyntaxNodeInfo {
                kind: "source_file".to_string(),
                start_byte: 0,
                end_byte: code.len(),
                // Using `lines.len()` here reported a row one past the last line of the file.
                start_position: (0, 0),
                end_position: (row, col),
                is_named: true,
                has_error: has_syntax_mismatch,
            });
        }

        nodes
    }

    /// Extract all embedded code blocks (`<pre><code>` or ``` code ```) from a `Book` instance.
    pub fn extract_code_blocks(book: &Book) -> Vec<ExtractedCodeBlock> {
        let mut blocks = Vec::new();
        let sections = book.get_all_sections_hydrated();

        for section in &sections {
            let html = &section.raw_html;
            let mut search_idx = 0;

            while let Some(start_tag) = html[search_idx..].find("<pre") {
                let abs_start = search_idx + start_tag;
                // A `<pre>` tag may carry attributes (`<pre class="source">`), so advance to the
                // real `>` of the opening tag instead of assuming it ends at `<pre>`.
                let Some(open_end_rel) = html[abs_start..].find('>') else {
                    break;
                };
                let content_start = abs_start + open_end_rel + 1;
                if let Some(end_tag) = html[content_start..].find("</pre>") {
                    let abs_end = content_start + end_tag + 6;
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
