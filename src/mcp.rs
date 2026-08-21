use crate::book::Book;
use crate::rag::{RagChunkConfig, RagChunker};
use crate::validator::EpubValidator;
use ahash::AHashMap;
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::{Arc, LazyLock};

struct CachedMcpBook {
    modified: Option<std::time::SystemTime>,
    book: Arc<Book>,
}

static MCP_BOOK_CACHE: LazyLock<RwLock<AHashMap<String, CachedMcpBook>>> =
    LazyLock::new(|| RwLock::new(AHashMap::new()));

pub fn get_cached_mcp_book(path: &str) -> Result<Arc<Book>, Box<dyn std::error::Error>> {
    let mod_time = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
    {
        let cache = MCP_BOOK_CACHE.read();
        if let Some(entry) = cache.get(path) {
            if entry.modified == mod_time {
                return Ok(entry.book.clone());
            }
        }
    }

    let book = Arc::new(Book::from_file(path)?);
    {
        let mut cache = MCP_BOOK_CACHE.write();
        if cache.len() >= 16 {
            cache.clear();
        }
        cache.insert(
            path.to_string(),
            CachedMcpBook {
                modified: mod_time,
                book: book.clone(),
            },
        );
    }
    Ok(book)
}

/// MCP JSON-RPC Request structure
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// Start the Model Context Protocol (MCP) server on stdio.
pub fn run_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    const MAX_MCP_LINE_SIZE: usize = 16 * 1024 * 1024;
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        if line.len() > MAX_MCP_LINE_SIZE {
            line.clear();
            let err_resp = json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32600,
                    "message": "Request entity too large: MCP line exceeds 16MB limit"
                },
                "id": Value::Null
            });
            let _ = send_json(&mut writer, &err_resp);
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => {
                handle_mcp_request(&req, &mut writer)?;
            }
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    },
                    "id": Value::Null
                });
                send_json(&mut writer, &err_resp)?;
            }
        }

        line.clear();
    }

    Ok(())
}

fn handle_mcp_request<W: Write>(
    req: &JsonRpcRequest,
    writer: &mut W,
) -> Result<(), Box<dyn std::error::Error>> {
    let resp = process_mcp_request(req);
    if let Some(val) = resp {
        send_json(writer, &val)?;
    }
    Ok(())
}

/// Process JSON-RPC request and return Value response (or None for notifications).
pub fn process_mcp_request(req: &JsonRpcRequest) -> Option<Value> {
    if req.jsonrpc != "2.0" {
        return Some(json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "error": {
                "code": -32600,
                "message": format!("Invalid Request: jsonrpc version must be '2.0', got '{}'", req.jsonrpc)
            }
        }));
    }

    match req.method.as_str() {
        "initialize" => {
            let requested_proto = req
                .params
                .as_ref()
                .and_then(|p| p.get("protocolVersion"))
                .and_then(|v| v.as_str())
                .unwrap_or("2024-11-05");

            let supported_proto = "2024-11-05";
            let negotiated_proto = match requested_proto {
                "2024-11-05" | "2024-10-07" => requested_proto,
                _ => supported_proto,
            };

            Some(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": {
                    "protocolVersion": negotiated_proto,
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                        "prompts": {}
                    },
                    "serverInfo": {
                        "name": "ebook-rs-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            }))
        }
        "notifications/initialized" => None,
        "ping" => Some(json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "result": {}
        })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "result": {
                "tools": list_mcp_tools()
            }
        })),
        "tools/call" => {
            let result = handle_tool_call(req.params.as_ref());
            Some(match result {
                Ok(content) => json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": content
                            }
                        ]
                    }
                }),
                Err(err) => json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Error: {}", err)
                            }
                        ],
                        "isError": true
                    }
                }),
            })
        }
        "resources/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "result": {
                "resources": [
                    {
                        "uri": "ebook://info",
                        "name": "ebook-rs System Info",
                        "description": "eBook engine version, supported formats, and feature capabilities",
                        "mimeType": "application/json"
                    }
                ]
            }
        })),
        "resources/read" => {
            let result = handle_resource_read(req.params.as_ref());
            Some(match result {
                Ok((uri, text, mime)) => json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "result": {
                        "contents": [
                            {
                                "uri": uri,
                                "mimeType": mime,
                                "text": text
                            }
                        ]
                    }
                }),
                Err(err) => json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "error": {
                        "code": -32602,
                        "message": format!("Resource error: {}", err)
                    }
                }),
            })
        }
        "prompts/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "result": {
                "prompts": list_mcp_prompts()
            }
        })),
        "prompts/get" => {
            let result = handle_prompt_get(req.params.as_ref());
            Some(match result {
                Ok(messages) => json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "result": {
                        "messages": messages
                    }
                }),
                Err(err) => json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "error": {
                        "code": -32602,
                        "message": format!("Prompt error: {}", err)
                    }
                }),
            })
        }
        _ => {
            if req.id.is_some() {
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", req.method)
                    }
                }))
            } else {
                None
            }
        }
    }
}

fn send_json<W: Write>(writer: &mut W, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let json_str = serde_json::to_string(value)?;
    writeln!(writer, "{}", json_str)?;
    writer.flush()?;
    Ok(())
}

fn list_mcp_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "get_metadata",
            "description": "Extract full metadata (title, author, publisher, description, language, rights, total section count) from an eBook file (EPUB, MOBI, AZW3, FB2, LIT, CBZ, PDF, ODT, TXT, MD).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or relative path to the eBook file" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "get_toc",
            "description": "Get the complete Table of Contents navigation hierarchy and chapter tree for an eBook.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the eBook file" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "read_section",
            "description": "Read a specific chapter/section from an eBook by section index or chapter title query. Returns clean plain text or Markdown with heading structure and CFI anchor.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the eBook file" },
                    "section_index": { "type": "integer", "description": "0-based index of the section to read" },
                    "chapter_title": { "type": "string", "description": "Title or partial title of the chapter to search for and read" },
                    "format": { "type": "string", "description": "Output format: 'text' or 'markdown' (default 'markdown')" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "search_book",
            "description": "Search for a keyword or phrase across an eBook file, returning matched snippets, line numbers, section indices, and CFI locator anchors.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the eBook file" },
                    "query": { "type": "string", "description": "Search query string" },
                    "max_results": { "type": "integer", "description": "Maximum number of matching snippets to return (default 20)" }
                },
                "required": ["path", "query"]
            }
        }),
        json!({
            "name": "chunk_book_for_rag",
            "description": "Chunk an eBook into semantic passages optimized for RAG retrieval, vector database embeddings, or LLM prompt injection with Okapi BM25 relevance ranking.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the eBook file" },
                    "max_tokens": { "type": "integer", "description": "Maximum tokens per chunk (default 512)" },
                    "overlap_tokens": { "type": "integer", "description": "Overlap tokens between chunks (default 64)" },
                    "query_rank": { "type": "string", "description": "Optional search query to rank chunks by Okapi BM25 relevance score" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "convert_ebook",
            "description": "Convert an eBook between supported formats (.epub, .kfx, or RAG .json).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input_path": { "type": "string", "description": "Path to input eBook file" },
                    "output_path": { "type": "string", "description": "Path to output file (.epub, .kfx, or .json)" }
                },
                "required": ["input_path", "output_path"]
            }
        }),
        json!({
            "name": "validate_epub",
            "description": "Validate an EPUB file against EPUB specifications, returning detailed reports on structural errors, broken links, or manifest warnings.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the EPUB file to validate" }
                },
                "required": ["path"]
            }
        }),
    ]
}

fn list_mcp_prompts() -> Vec<Value> {
    vec![
        json!({
            "name": "summarize_book",
            "description": "Generate a comprehensive chapter-by-chapter summary and key themes prompt for an eBook.",
            "arguments": [
                { "name": "path", "description": "Path to the eBook file", "required": true },
                { "name": "focus", "description": "Specific focus or topic of interest (optional)", "required": false }
            ]
        }),
        json!({
            "name": "extract_entities",
            "description": "Generate an entity extraction prompt for key characters, locations, and terminology in an eBook.",
            "arguments": [
                { "name": "path", "description": "Path to the eBook file", "required": true }
            ]
        }),
        json!({
            "name": "generate_study_guide",
            "description": "Generate a study guide with review questions and key takeaways for an eBook.",
            "arguments": [
                { "name": "path", "description": "Path to the eBook file", "required": true }
            ]
        }),
    ]
}

fn handle_resource_read(
    params: Option<&Value>,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let params_obj = params.ok_or("Missing params object")?;
    let uri = params_obj
        .get("uri")
        .and_then(|v| v.as_str())
        .ok_or("Missing required argument 'uri'")?;

    if uri == "ebook://info" {
        let info = json!({
            "engine": "ebook-rs",
            "version": env!("CARGO_PKG_VERSION"),
            "supported_formats": ["EPUB2", "EPUB3", "MOBI", "AZW3", "FB2", "LIT", "CBZ", "PDF", "ODT", "TXT", "MD"],
            "features": ["TTS Synchronizer", "Readium LCP/Locator", "Zstd State Caching", "RAG BM25 Chunker", "EPUB3 Exporter", "MCP Server"]
        });
        Ok((
            uri.to_string(),
            serde_json::to_string_pretty(&info)?,
            "application/json".to_string(),
        ))
    } else if uri.starts_with("ebook://") {
        let rest = uri.trim_start_matches("ebook://");
        if let Some((path, sub)) = rest.rsplit_once('/') {
            let book = get_cached_mcp_book(path)?;
            if sub == "metadata" {
                let meta = serde_json::to_string_pretty(book.metadata())?;
                return Ok((uri.to_string(), meta, "application/json".to_string()));
            } else if sub == "toc" {
                let toc = serde_json::to_string_pretty(book.toc())?;
                return Ok((uri.to_string(), toc, "application/json".to_string()));
            }
        }
        Err(format!("Unsupported resource URI: {}", uri).into())
    } else {
        Err(format!("Unsupported resource URI scheme: {}", uri).into())
    }
}

fn handle_prompt_get(params: Option<&Value>) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let params_obj = params.ok_or("Missing params object")?;
    let prompt_name = params_obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing prompt name")?;
    let args = params_obj.get("arguments").cloned().unwrap_or(json!({}));

    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing required prompt argument 'path'")?;
    let book = get_cached_mcp_book(path)?;
    let meta = book.metadata();

    match prompt_name {
        "summarize_book" => {
            let focus = args
                .get("focus")
                .and_then(|v| v.as_str())
                .unwrap_or("general overview");
            let prompt_text = format!(
                "You are analyzing the eBook '{}' by {}.\nFocus: {}\nTotal Sections: {}\n\nPlease provide a clear chapter-by-chapter summary and key insights using the book's contents.",
                meta.title,
                meta.creators.join(", "),
                focus,
                book.sections.len()
            );
            Ok(vec![json!({
                "role": "user",
                "content": {
                    "type": "text",
                    "text": prompt_text
                }
            })])
        }
        "extract_entities" => {
            let prompt_text = format!(
                "Please extract and catalog all key characters, major locations, organizations, and important concepts from the eBook '{}' by {}.",
                meta.title,
                meta.creators.join(", ")
            );
            Ok(vec![json!({
                "role": "user",
                "content": {
                    "type": "text",
                    "text": prompt_text
                }
            })])
        }
        "generate_study_guide" => {
            let prompt_text = format!(
                "Create a study guide for '{}' by {} including:\n1. Executive Summary\n2. Key Takeaways\n3. 10 Critical Discussion / Review Questions.",
                meta.title,
                meta.creators.join(", ")
            );
            Ok(vec![json!({
                "role": "user",
                "content": {
                    "type": "text",
                    "text": prompt_text
                }
            })])
        }
        _ => Err(format!("Unknown prompt: {}", prompt_name).into()),
    }
}

fn handle_tool_call(params: Option<&Value>) -> Result<String, Box<dyn std::error::Error>> {
    let params_obj = params.ok_or("Missing params object")?;
    let tool_name = params_obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing tool name")?;
    let args = params_obj.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "get_metadata" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required argument 'path'")?;
            let book = get_cached_mcp_book(path)?;
            let meta = book.metadata();
            let mut result_json = serde_json::to_value(meta)?;
            if let Some(obj) = result_json.as_object_mut() {
                obj.insert("total_sections".to_string(), json!(book.sections.len()));
                obj.insert(
                    "total_locations".to_string(),
                    json!(book.locations.total_locations),
                );
            }
            Ok(serde_json::to_string_pretty(&result_json)?)
        }
        "get_toc" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required argument 'path'")?;
            let book = get_cached_mcp_book(path)?;
            Ok(serde_json::to_string_pretty(book.toc())?)
        }
        "read_section" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required argument 'path'")?;
            let book = get_cached_mcp_book(path)?;

            let format = args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("markdown");
            let target_index = if let Some(idx) = args.get("section_index").and_then(|v| v.as_u64())
            {
                idx as usize
            } else if let Some(title_query) = args.get("chapter_title").and_then(|v| v.as_str()) {
                let mut matched_idx = None;
                for nav in &book.toc {
                    if nav
                        .label
                        .to_lowercase()
                        .contains(&title_query.to_lowercase())
                    {
                        for sec in &book.sections {
                            if nav.href.contains(&sec.href) || sec.href.contains(&nav.href) {
                                matched_idx = Some(sec.index);
                                break;
                            }
                        }
                    }
                    if matched_idx.is_some() {
                        break;
                    }
                }
                matched_idx.unwrap_or(0)
            } else {
                0
            };

            if target_index >= book.sections.len() {
                return Err(format!(
                    "Section index {} out of range (total sections: {})",
                    target_index,
                    book.sections.len()
                )
                .into());
            }

            let section = &book.sections[target_index];
            let cfi_anchor = crate::cfi::Cfi::from_spine_index(target_index, None, 0).to_string();
            let approx_tokens = section.plain_text.len() / 4;
            let section_title = format!("Section {}", section.index);

            if format == "text" {
                Ok(format!(
                    "--- {} (Index {}) ---\nHref: {}\nCFI Anchor: {}\nEstimated Tokens: ~{}\n\n{}",
                    section_title,
                    section.index,
                    section.href,
                    cfi_anchor,
                    approx_tokens,
                    section.plain_text
                ))
            } else {
                Ok(format!(
                    "# {}\n*Href: `{}` | Index: {} | CFI: `{}` | Tokens: ~{}*\n\n{}",
                    section_title,
                    section.href,
                    section.index,
                    cfi_anchor,
                    approx_tokens,
                    section.plain_text
                ))
            }
        }
        "search_book" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required argument 'path'")?;
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or("Missing required argument 'query'")?;
            let max_results = args
                .get("max_results")
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;

            let book = get_cached_mcp_book(path)?;
            let mut results = book.search(query);
            if results.len() > max_results {
                results.truncate(max_results);
            }

            Ok(serde_json::to_string_pretty(&results)?)
        }
        "chunk_book_for_rag" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required argument 'path'")?;
            let max_tokens = args
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(512) as usize;
            let overlap_tokens = args
                .get("overlap_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(64) as usize;
            let query_rank = args.get("query_rank").and_then(|v| v.as_str());

            let book = get_cached_mcp_book(path)?;
            let config = RagChunkConfig {
                max_tokens,
                overlap_tokens,
                preserve_headings: true,
                include_cfi: true,
                min_chunk_size: 50,
            };

            let chunks = book.to_rag_chunks(&config);
            if let Some(query) = query_rank {
                let ranked = RagChunker::rank_chunks_bm25(&chunks, query, 20);
                Ok(serde_json::to_string_pretty(&ranked)?)
            } else {
                Ok(serde_json::to_string_pretty(&chunks)?)
            }
        }
        "convert_ebook" => {
            let input_path = args
                .get("input_path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required argument 'input_path'")?;
            let output_path = args
                .get("output_path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required argument 'output_path'")?;

            if input_path.contains('\0') || output_path.contains('\0') || output_path.contains("..")
            {
                return Err("Invalid file path: path traversal or null byte detected".into());
            }

            if !Path::new(input_path).exists() {
                return Err(format!("Input file not found: {}", input_path).into());
            }

            if let Some(parent) = Path::new(output_path).parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }

            let book = Book::from_file(input_path)?;
            if output_path.ends_with(".epub") {
                let bytes = crate::UniversalEpub3Exporter::export(&book)?;
                std::fs::write(output_path, bytes)?;
                Ok(format!(
                    "Successfully converted '{}' to EPUB3 at '{}'",
                    input_path, output_path
                ))
            } else if output_path.ends_with(".kfx") {
                let bytes = crate::UniversalKfxExporter::export(&book)?;
                std::fs::write(output_path, bytes)?;
                Ok(format!(
                    "Successfully converted '{}' to KFX at '{}'",
                    input_path, output_path
                ))
            } else if output_path.ends_with(".json") {
                let chunks = book.to_rag_chunks(&RagChunkConfig::default());
                let json_data = serde_json::to_string_pretty(&chunks)?;
                std::fs::write(output_path, json_data)?;
                Ok(format!(
                    "Successfully exported RAG chunks from '{}' to JSON at '{}'",
                    input_path, output_path
                ))
            } else {
                Err("Unsupported output extension. Expected .epub, .kfx, or .json".into())
            }
        }
        "validate_epub" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required argument 'path'")?;
            if !Path::new(path).exists() {
                return Err(format!("File not found: {}", path).into());
            }

            let book = Book::from_file(path)?;
            let report = EpubValidator::validate(&book);
            Ok(serde_json::to_string_pretty(&report)?)
        }
        _ => Err(format!("Unknown tool: {}", tool_name).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize_and_list_tools() {
        let req_init: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#
        ).unwrap();

        let resp_init = process_mcp_request(&req_init).unwrap();
        let resp_str = resp_init.to_string();
        assert!(resp_str.contains("ebook-rs-mcp"));
        assert!(resp_str.contains("2024-11-05"));

        let req_tools: JsonRpcRequest =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
        let resp_tools = process_mcp_request(&req_tools).unwrap();
        let resp_tools_str = resp_tools.to_string();
        assert!(resp_tools_str.contains("get_metadata"));
        assert!(resp_tools_str.contains("read_section"));
        assert!(resp_tools_str.contains("search_book"));
        assert!(resp_tools_str.contains("chunk_book_for_rag"));

        let req_prompts: JsonRpcRequest =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":3,"method":"prompts/list"}"#).unwrap();
        let resp_prompts = process_mcp_request(&req_prompts).unwrap();
        assert!(resp_prompts.to_string().contains("summarize_book"));
    }

    #[test]
    fn test_mcp_book_cache() {
        let temp_dir = std::env::temp_dir();
        let test_epub_path = temp_dir.join("mcp_test_cache_sample.epub");
        let bytes = crate::generate_sample_epub().unwrap();
        std::fs::write(&test_epub_path, bytes).unwrap();

        let path_str = test_epub_path.to_str().unwrap();

        // First call populates cache
        let b1 = get_cached_mcp_book(path_str).expect("Failed first book load");
        assert_eq!(b1.metadata().title, "The Rustonomicon & EBook-RS Guide");

        // Second call retrieves from cache
        let b2 = get_cached_mcp_book(path_str).expect("Failed second book load from cache");
        assert_eq!(b2.metadata().title, b1.metadata().title);

        let _ = std::fs::remove_file(test_epub_path);
    }
}
