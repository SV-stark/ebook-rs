use crate::annotations::Annotation;
use crate::book::Book;
use crate::web_ui::READER_HTML;
use std::sync::{Arc, RwLock};
use tiny_http::{Header, Response, Server, StatusCode};
use url::Url;

/// Maximum simultaneously serviced connections. Requests beyond this are rejected immediately.
const MAX_ACTIVE_REQUESTS: usize = 64;
/// Maximum accepted request body size for the JSON-RPC endpoint.
const MAX_MCP_BODY: u64 = 4 * 1024 * 1024;
/// Maximum accepted request body size for annotation writes.
const MAX_ANNOTATION_BODY: u64 = 2 * 1024 * 1024;
/// Hard cap on search results. Enforced inside the engine rather than after the fact: building
/// millions of `SearchResult`s and then truncating is itself the denial of service.
const MAX_SEARCH_RESULTS: usize = 500;
/// Maximum stored annotations.
const MAX_ANNOTATIONS: usize = 5000;

/// Embedded HTTP Reader Server.
pub struct ReaderServer {
    book: Arc<RwLock<Book>>,
    port: u16,
    auth_token: Option<String>,
    /// Root directory that filesystem-touching MCP tools are confined to. `None` disables those
    /// tools entirely.
    sandbox_root: Option<std::path::PathBuf>,
}

impl ReaderServer {
    pub fn new(book: Book, port: u16) -> Self {
        Self {
            book: Arc::new(RwLock::new(book)),
            port,
            auth_token: None,
            sandbox_root: None,
        }
    }

    /// Set an optional bearer authentication token.
    ///
    /// Enforced on **every** state-changing and book-content route (`/api/mcp` and annotation
    /// writes). An empty token is rejected outright: `with_auth_token("")` would otherwise make a
    /// bare `Authorization: ` header compare equal and disable the control entirely.
    ///
    /// # Errors
    /// Returns an error if `token` is empty or whitespace-only.
    pub fn try_with_auth_token(mut self, token: impl Into<String>) -> Result<Self, String> {
        let token = token.into();
        if token.trim().is_empty() {
            return Err("auth token must not be empty".to_string());
        }
        self.auth_token = Some(token);
        Ok(self)
    }

    /// Set an optional bearer authentication token for protected routes.
    ///
    /// Empty tokens are ignored (authentication stays disabled) rather than silently accepted.
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        let token = token.into();
        if token.trim().is_empty() {
            eprintln!("⚠️  Ignoring empty auth token: reader API remains unauthenticated.");
        } else {
            self.auth_token = Some(token);
        }
        self
    }

    /// Confine filesystem-touching MCP tools to `root`.
    ///
    /// When unset, tools that read or write arbitrary paths (`convert_ebook`, `get_metadata`,
    /// `read_section`, `validate_epub`, ...) are refused. This is the control that stops a
    /// caller-supplied `output_path` from becoming an arbitrary-file-write primitive.
    #[must_use]
    pub fn with_sandbox_root(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.sandbox_root = Some(root.into());
        self
    }

    /// Start listening and serving incoming HTTP requests.
    /// P6 & B7 Fix: Thread-pool request dispatching with rwlock poison protection.
    pub fn listen(&self) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let server = Arc::new(
            Server::http(&addr)
                .map_err(|e| format!("Failed to start server on {}: {}", addr, e))?,
        );
        println!(
            "🚀 EBook-RS Reader Server listening on http://localhost:{}",
            self.port
        );
        if self.auth_token.is_some() {
            println!("🔒 API authenticated (bearer token configured).");
        } else {
            println!(
                "⚠️  API is UNAUTHENTICATED. Set EBOOK_RS_AUTH_TOKEN (or pass --auth-token) \
                 before exposing this beyond a trusted machine."
            );
        }
        match &self.sandbox_root {
            Some(root) => println!("🔒 MCP filesystem tools sandboxed to {}", root.display()),
            None => println!("⚠️  MCP filesystem tools are DISABLED (no sandbox root configured)."),
        }
        println!("Press Ctrl+C to exit.");

        let active_threads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let server_auth_token = self.auth_token.clone();
        let sandbox_root = self.sandbox_root.clone();

        for mut request in server.incoming_requests() {
            // Refuse new work once the pool is saturated, before spawning anything.
            if active_threads.load(std::sync::atomic::Ordering::Relaxed) >= MAX_ACTIVE_REQUESTS
                || server.num_connections() >= MAX_ACTIVE_REQUESTS
            {
                let _ = request.respond(
                    Response::from_string("503 Service Unavailable: Server busy")
                        .with_status_code(StatusCode(503)),
                );
                continue;
            }

            let book_arc = Arc::clone(&self.book);
            let active_cnt = Arc::clone(&active_threads);
            let auth_tok = server_auth_token.clone();
            let sandbox = sandbox_root.clone();

            active_cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::thread::spawn(move || {
                let _guard = ThreadGuard(active_cnt);

                // DNS Rebinding Protection: Verify Host header if present.
                let host_header = request
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Host"))
                    .map(|h| h.value.as_str());
                if let Some(host) = host_header {
                    // `split(':').next()` turned `[::1]:8080` into `"["`, which is not in the
                    // allowlist, so every IPv6-literal request was rejected with a 403 and the
                    // `[::1]` / `::1` entries below were unreachable. Strip the bracketed IPv6
                    // form and the optional port separately.
                    let host_clean = host.trim();
                    let host_clean = host_clean
                        .strip_prefix('[')
                        .and_then(|h| h.split_once(']'))
                        .map_or_else(
                            || host_clean.split(':').next().unwrap_or(host_clean),
                            |(addr, _)| addr,
                        );
                    if !host_clean.eq_ignore_ascii_case("localhost")
                        && host_clean != "127.0.0.1"
                        && host_clean != "::1"
                    {
                        send_text(
                            request,
                            StatusCode(403),
                            "text/plain; charset=utf-8",
                            "403 Forbidden: Invalid Host Header",
                        );
                        return;
                    }
                }

                let url_str = format!("http://localhost{}", request.url());
                let parsed_url = Url::parse(&url_str)
                    .unwrap_or_else(|_| Url::parse("http://localhost/").unwrap());
                let path = parsed_url.path().to_string();

                match path.as_str() {
                    "/" | "/index.html" => {
                        send_text(
                            request,
                            StatusCode(200),
                            "text/html; charset=utf-8",
                            READER_HTML,
                        );
                    }

                    // ---- Authenticated routes -------------------------------------------
                    "/api/mcp" => {
                        // Auth is enforced for every state-changing route, not just this one.
                        if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                            let msg = mcp_error_message(code);
                            send_json(request, code, &jsonrpc_transport_error(code, msg));
                            return;
                        }

                        if request.method() != &tiny_http::Method::Post {
                            send_json(
                                request,
                                405,
                                "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Method not allowed: use POST\"},\"id\":null}",
                            );
                            return;
                        }

                        // Validate Content-Type: application/json
                        let content_type = header_value(&request, "Content-Type");
                        let is_json =
                            content_type.is_some_and(|ct| ct.starts_with("application/json"));
                        if !is_json {
                            send_json(
                                request,
                                415,
                                "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Unsupported Media Type: Content-Type must be application/json\"},\"id\":null}",
                            );
                            return;
                        }

                        // Anti-CSRF: reject cross-origin browser requests.
                        if !is_same_origin(&request) {
                            send_json(
                                request,
                                403,
                                "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32000,\"message\":\"CSRF protection: cross-origin request rejected\"},\"id\":null}",
                            );
                            return;
                        }

                        let body = match read_body_capped(&mut request, MAX_MCP_BODY) {
                            Ok(b) => b,
                            Err(BodyReadError::TooLarge) => {
                                send_json(
                                    request,
                                    StatusCode(413),
                                    "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Request entity too large\"},\"id\":null}",
                                );
                                return;
                            }
                        };
                        match serde_json::from_str::<crate::mcp::JsonRpcRequest>(&body) {
                            Ok(mcp_req) => {
                                if let Some(resp_val) =
                                    crate::mcp::process_mcp_request(&mcp_req, sandbox.as_deref())
                                {
                                    let json_resp = serde_json::to_string(&resp_val)
                                        .unwrap_or_else(|_| {
                                            jsonrpc_error(500, "Failed to serialize MCP response")
                                        });
                                    send_json(request, 200, &json_resp);
                                    return;
                                }
                                // Notification (no `id`): JSON-RPC requires no response.
                                send_empty(request, 204);
                            }
                            Err(e) => {
                                let err = format!(
                                    "Parse error: {}",
                                    e.to_string().replace('\\', "\\\\").replace('"', "\\\"")
                                );
                                send_json(request, 400, &jsonrpc_error(400, &err));
                            }
                        }
                    }

                    "/api/annotations" => {
                        if request.method() == &tiny_http::Method::Post {
                            // Previously this mutating, persistent route had no auth check at
                            // all, even when a token was configured.
                            if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                                send_text(
                                    request,
                                    StatusCode(code),
                                    "text/plain; charset=utf-8",
                                    "401 Unauthorized",
                                );
                                return;
                            }
                            if !is_same_origin(&request) {
                                send_text(
                                    request,
                                    StatusCode(403),
                                    "text/plain; charset=utf-8",
                                    "Forbidden (Cross-Origin Request Blocked)",
                                );
                                return;
                            }
                            let body_str = match read_body_capped(&mut request, MAX_ANNOTATION_BODY)
                            {
                                Ok(b) => b,
                                Err(BodyReadError::TooLarge) => {
                                    send_text(
                                        request,
                                        StatusCode(413),
                                        "text/plain; charset=utf-8",
                                        "413 Payload Too Large",
                                    );
                                    return;
                                }
                            };
                            if let Ok(mut ann) = serde_json::from_str::<Annotation>(&body_str) {
                                let mut book = book_arc.write().unwrap_or_else(|e| e.into_inner());
                                if book.annotations.list().len() >= MAX_ANNOTATIONS {
                                    send_text(
                                        request,
                                        StatusCode(400),
                                        "text/plain; charset=utf-8",
                                        "400 Bad Request: Maximum annotation capacity reached",
                                    );
                                    return;
                                }
                                // Assign server-managed ID to prevent client ID collisions or injection
                                if ann.id.is_empty()
                                    || ann.id.len() > 64
                                    || !ann.id.chars().all(|c| c.is_alphanumeric() || c == '-')
                                {
                                    static ANN_COUNTER: std::sync::atomic::AtomicU64 =
                                        std::sync::atomic::AtomicU64::new(1);
                                    let c = ANN_COUNTER
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_nanos())
                                        .unwrap_or(0);
                                    ann.id = format!("ann-{:x}-{:x}", now, c);
                                }
                                let ann_id = ann.id.clone();
                                book.annotations.add(ann);
                                let resp_json = serde_json::json!({
                                    "status": "success",
                                    "id": ann_id
                                })
                                .to_string();
                                send_json(request, 200, &resp_json);
                            } else {
                                send_text(
                                    request,
                                    StatusCode(400),
                                    "text/plain; charset=utf-8",
                                    "Invalid annotation payload",
                                );
                            }
                        } else if request.method() == &tiny_http::Method::Get {
                            if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                                send_text(
                                    request,
                                    StatusCode(code),
                                    "text/plain; charset=utf-8",
                                    "401 Unauthorized",
                                );
                                return;
                            }
                            let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                            let json =
                                serde_json::to_string(&book.annotations.list()).unwrap_or_default();
                            send_json(request, 200, &json);
                        } else {
                            send_text(
                                request,
                                StatusCode(405),
                                "text/plain; charset=utf-8",
                                "Method not allowed",
                            );
                        }
                    }

                    // ---- Book content routes --------------------------------------------
                    _ if path.starts_with("/api/book/section/") => {
                        if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                            send_text(
                                request,
                                StatusCode(code),
                                "text/plain; charset=utf-8",
                                "401 Unauthorized",
                            );
                            return;
                        }
                        let sec_idx_str = path.trim_start_matches("/api/book/section/");
                        let sec_idx: usize = match sec_idx_str.parse() {
                            Ok(v) => v,
                            Err(_) => {
                                send_text(
                                    request,
                                    StatusCode(400),
                                    "text/plain; charset=utf-8",
                                    "400 Bad Request: section index must be a non-negative integer",
                                );
                                return;
                            }
                        };
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        match book.get_section(sec_idx) {
                            Ok(sec) => {
                                send_text(
                                    request,
                                    StatusCode(200),
                                    "text/html; charset=utf-8",
                                    &sec.processed_html,
                                );
                            }
                            Err(_) => {
                                // Do not reflect internal error text (which can contain
                                // archive paths) back to the client.
                                send_text(
                                    request,
                                    StatusCode(404),
                                    "text/plain; charset=utf-8",
                                    "404 Not Found",
                                );
                            }
                        }
                    }

                    _ if path.starts_with("/resource/") || path.starts_with("/api/resource/") => {
                        if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                            send_text(
                                request,
                                StatusCode(code),
                                "text/plain; charset=utf-8",
                                "401 Unauthorized",
                            );
                            return;
                        }
                        let clean_path = path
                            .strip_prefix("/api/resource/")
                            .or_else(|| path.strip_prefix("/resource/"))
                            .unwrap_or(&path);
                        let decoded_path = percent_encoding::percent_decode_str(clean_path)
                            .decode_utf8_lossy()
                            .to_string();
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());

                        match book.get_resource_bytes(&decoded_path) {
                            Ok((bytes, mime)) => {
                                // Raw archive bytes are served in the reader's own origin, so
                                // serving active content types (XHTML/SVG/JS) here would hand an
                                // eBook author same-origin script execution -- and therefore
                                // access to every authenticated API on this server. Serve those
                                // as inert downloads instead.
                                let effective_mime = if is_active_content_type(mime) {
                                    "application/octet-stream"
                                } else {
                                    mime
                                };

                                if let Some(range_val) = header_value(&request, "Range") {
                                    if let Some((start, end_opt)) =
                                        crate::archive::HttpRangeRequest::parse_range_header(
                                            &range_val,
                                        )
                                    {
                                        let total = bytes.len() as u64;
                                        if total > 0 && start < total {
                                            let end = end_opt.unwrap_or(total - 1).min(total - 1);
                                            if start <= end {
                                                let slice = &bytes[start as usize..=end as usize];
                                                let cr_hdr = format!("bytes {start}-{end}/{total}");
                                                send_bytes_with_headers(
                                                    request,
                                                    StatusCode(206),
                                                    effective_mime,
                                                    slice,
                                                    &[
                                                        ("Content-Range", &cr_hdr),
                                                        ("Accept-Ranges", "bytes"),
                                                    ],
                                                );
                                                return;
                                            }
                                        }
                                        let cr_hdr = format!("bytes */{total}");
                                        send_bytes_with_headers(
                                            request,
                                            StatusCode(416),
                                            "text/plain; charset=utf-8",
                                            b"416 Range Not Satisfiable",
                                            &[("Content-Range", &cr_hdr)],
                                        );
                                        return;
                                    }
                                }

                                send_bytes_with_headers(
                                    request,
                                    StatusCode(200),
                                    effective_mime,
                                    &bytes,
                                    &[("Accept-Ranges", "bytes")],
                                );
                            }
                            Err(_) => {
                                send_text(
                                    request,
                                    StatusCode(404),
                                    "text/plain; charset=utf-8",
                                    "404 Not Found",
                                );
                            }
                        }
                    }

                    "/api/book/search" => {
                        if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                            send_text(
                                request,
                                StatusCode(code),
                                "text/plain; charset=utf-8",
                                "401 Unauthorized",
                            );
                            return;
                        }
                        let query = parsed_url
                            .query_pairs()
                            .find(|(k, _)| k == "q")
                            .map(|(_, v)| v.to_string())
                            .unwrap_or_default();

                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        // The engine truncates internally; do not materialize the full set.
                        let results = book.search_limited(&query, Some(MAX_SEARCH_RESULTS));

                        let json = serde_json::to_string(&results).unwrap_or_default();
                        send_json(request, 200, &json);
                    }

                    "/api/book/locations" => {
                        if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                            send_text(
                                request,
                                StatusCode(code),
                                "text/plain; charset=utf-8",
                                "401 Unauthorized",
                            );
                            return;
                        }
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        let json = serde_json::to_string(&book.locations).unwrap_or_default();
                        send_json(request, 200, &json);
                    }

                    "/api/book/cover" => {
                        if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                            send_text(
                                request,
                                StatusCode(code),
                                "text/plain; charset=utf-8",
                                "401 Unauthorized",
                            );
                            return;
                        }
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        if let Some((bytes, mime)) = book.cover_image() {
                            if is_active_content_type(mime) {
                                send_bytes(
                                    request,
                                    StatusCode(200),
                                    "application/octet-stream",
                                    &bytes,
                                );
                            } else {
                                send_bytes(request, StatusCode(200), mime, &bytes);
                            }
                        } else {
                            send_text(
                                request,
                                StatusCode(404),
                                "text/plain; charset=utf-8",
                                "No cover found",
                            );
                        }
                    }

                    "/api/book/metadata" => {
                        if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                            send_text(
                                request,
                                StatusCode(code),
                                "text/plain; charset=utf-8",
                                "401 Unauthorized",
                            );
                            return;
                        }
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        let json = serde_json::to_string(&book.metadata()).unwrap_or_default();
                        send_json(request, 200, &json);
                    }

                    "/api/book/spine" => {
                        if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                            send_text(
                                request,
                                StatusCode(code),
                                "text/plain; charset=utf-8",
                                "401 Unauthorized",
                            );
                            return;
                        }
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        let json = serde_json::to_string(&book.spine()).unwrap_or_default();
                        send_json(request, 200, &json);
                    }

                    "/api/book/toc" => {
                        if let Some(code) = auth_required(&request, auth_tok.as_deref()) {
                            send_text(
                                request,
                                StatusCode(code),
                                "text/plain; charset=utf-8",
                                "401 Unauthorized",
                            );
                            return;
                        }
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        let json = serde_json::to_string(&book.toc()).unwrap_or_default();
                        send_json(request, 200, &json);
                    }

                    _ => {
                        send_text(
                            request,
                            StatusCode(404),
                            "text/plain; charset=utf-8",
                            "404 Not Found",
                        );
                    }
                }
            });
        }

        Ok(())
    }
}

/// Why a request body could not be read.
enum BodyReadError {
    /// Declared or actual body exceeded the configured byte limit.
    TooLarge,
}

/// Read a request body with a byte cap.
///
/// The declared `Content-Length` is checked before any read happens, so an oversized upload is
/// refused without pulling bytes off the socket at all.
///
/// Note on slow-loris: `tiny_http` 0.12 returns `&mut dyn Read` from `Request::as_reader()`, which
/// is not `Send`, so the read cannot be moved to a helper thread, and the crate exposes no
/// socket-level read timeout. A client that trickles a small body can therefore still hold a
/// connection open until it finishes. The damage is bounded by the concurrent-connection cap
/// (`MAX_ACTIVE_REQUESTS`), which is checked in the accept loop. Fixing this properly needs a
/// different HTTP stack or an upstream `tiny_http` timeout API.
fn read_body_capped(
    request: &mut tiny_http::Request,
    max_bytes: u64,
) -> Result<String, BodyReadError> {
    use std::io::Read;

    if let Some(len) = request.body_length() {
        if len as u64 > max_bytes {
            return Err(BodyReadError::TooLarge);
        }
    }

    let mut buf = String::new();
    // `take` bounds the actual bytes read; `+1` lets us detect exceeding the declared length.
    let _ = request
        .as_reader()
        .take(max_bytes.saturating_add(1))
        .read_to_string(&mut buf);
    if buf.len() as u64 > max_bytes {
        return Err(BodyReadError::TooLarge);
    }
    Ok(buf)
}

/// Content types that can execute script when rendered by a browser in this origin.
fn is_active_content_type(mime: &str) -> bool {
    let m = mime.trim().to_ascii_lowercase();
    m.starts_with("text/html")
        || m.starts_with("application/xhtml")
        || m.starts_with("image/svg")
        || m.starts_with("application/javascript")
        || m.starts_with("text/javascript")
        || m.starts_with("application/ecmascript")
        || m.starts_with("text/xml")
        || m.starts_with("application/xml")
}

/// Look up a header value by name.
///
/// `name` must be `'static` because `tiny_http::Header::field` compares against a fixed set of
/// `&'static str` field names.
fn header_value(request: &tiny_http::Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        // tiny_http stores header values as an ASCII string type.
        .map(|h| h.value.as_str().to_owned())
}

/// Constant-time string equality, to avoid leaking token material through response timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Check bearer authentication for a protected route.
///
/// Returns `None` when the request is authorized, otherwise the HTTP status to return.
/// When no token is configured the server is unauthenticated by design (a loud warning is
/// printed at startup).
fn auth_required(request: &tiny_http::Request, required_token: Option<&str>) -> Option<u16> {
    let required = required_token?;
    if required.is_empty() {
        // Defensive: an empty configured token must never authenticate anything.
        return Some(401);
    }

    let authed = if let Some(hdr) = header_value(request, "Authorization") {
        match hdr.strip_prefix("Bearer ") {
            Some(tok) => constant_time_eq(tok.trim().as_bytes(), required.as_bytes()),
            None => constant_time_eq(hdr.as_bytes(), required.as_bytes()),
        }
    } else if let Some(tok) = header_value(request, "X-MCP-Token") {
        constant_time_eq(tok.as_bytes(), required.as_bytes())
    } else {
        false
    };

    if authed { None } else { Some(401) }
}

/// Whether the request originates from this application's own origin.
///
/// Defence in depth against CSRF; authentication is the primary control. `Sec-Fetch-Site` is
/// honoured when present because it cannot be forged by a non-browser client.
fn is_same_origin(request: &tiny_http::Request) -> bool {
    if let Some(site) = header_value(request, "Sec-Fetch-Site") {
        let site = site.trim().to_ascii_lowercase();
        if site == "cross-site" || site == "same-site" {
            return false;
        }
    }
    if let Some(origin) = header_value(request, "Origin") {
        return is_local_origin(&origin);
    }
    if let Some(referer) = header_value(request, "Referer") {
        return is_local_origin(&referer);
    }
    // Neither header present: not a browser-initiated cross-site request (browsers always send
    // `Origin` on cross-origin POSTs). Allowed through; auth is still required.
    true
}

/// Single source of truth for the local-origin allowlist.
///
/// Previously duplicated in two places, and the copies had already drifted: the `/api/mcp`
/// closure was missing the `https://[::1]` entries. It also compared raw prefixes, so a
/// malformed authority such as `http://localhost:@evil.com` was accepted; this parses instead.
fn is_local_origin(val: &str) -> bool {
    let val = val.trim();
    let Some((scheme, rest)) = val.split_once("://") else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    // Strip any path/query/fragment before inspecting the authority.
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() {
        return false;
    }
    let host = match authority.rsplit_once(':') {
        Some((h, port)) if port.chars().all(|c| c.is_ascii_digit()) && !port.is_empty() => h,
        _ => authority,
    };
    // Reject userinfo tricks: `user@host` must not be accepted as `host`.
    if host.contains('@') {
        return false;
    }
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

/// JSON-RPC error code used for transport-level rejections (auth failure, unsupported method,
/// payload too large, CSRF). This is the JSON-RPC 2.0 "Invalid Request" code — folding the HTTP
/// status into the code (`status - 32000`) produced `-31601` for 401 and similar values, all of
/// which lie outside the reserved range `-32700…-32000` and are rejected or mis-bucketed by
/// conforming clients.
const JSONRPC_INVALID_REQUEST: i64 = -32600;

/// Build a JSON-RPC error response body.
fn jsonrpc_error(code: i64, message: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "error": { "code": code, "message": message },
        "id": serde_json::Value::Null
    })
    .to_string()
}

/// Build a JSON-RPC error for a transport-level rejection.
///
/// The HTTP status is folded into the *message* rather than the code, so clients still see which
/// HTTP-level reason applied without a non-conforming error code.
fn jsonrpc_transport_error(status: u16, message: &str) -> String {
    jsonrpc_error(
        JSONRPC_INVALID_REQUEST,
        &format!("[HTTP {status}] {message}"),
    )
}

fn mcp_error_message(status: u16) -> &'static str {
    match status {
        401 => "Unauthorized: Invalid or missing authentication token",
        _ => "Request rejected",
    }
}

/// Security headers applied to every response.
fn security_headers() -> Vec<Header> {
    let mut v = Vec::new();
    for (name, value) in [
        ("X-Content-Type-Options", "nosniff"),
        ("X-Frame-Options", "SAMEORIGIN"),
        ("Referrer-Policy", "no-referrer"),
        // No 'unsafe-inline' for scripts: book content is untrusted and is rendered into this
        // origin. Styles still need 'unsafe-inline' because reader CSS is injected inline.
        (
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: blob:; media-src 'self' data: blob:; font-src 'self' data:; \
             frame-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; \
             frame-ancestors 'self'",
        ),
    ] {
        if let Ok(h) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            v.push(h);
        }
    }
    v
}

fn send_text(
    request: tiny_http::Request,
    status: impl Into<StatusCode>,
    content_type: &str,
    body: &str,
) {
    let mut response = Response::from_string(body.to_string()).with_status_code(status.into());
    if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()) {
        response = response.with_header(h);
    }
    for h in security_headers() {
        response = response.with_header(h);
    }
    let _ = request.respond(response);
}

fn send_json(request: tiny_http::Request, status: impl Into<StatusCode>, body: &str) {
    send_text(request, status, "application/json", body);
}

fn send_bytes(
    request: tiny_http::Request,
    status: impl Into<StatusCode>,
    content_type: &str,
    body: &[u8],
) {
    send_bytes_with_headers(request, status, content_type, body, &[]);
}

fn send_bytes_with_headers(
    request: tiny_http::Request,
    status: impl Into<StatusCode>,
    content_type: &str,
    body: &[u8],
    extra_headers: &[(&str, &str)],
) {
    let mut response = Response::from_data(body.to_vec()).with_status_code(status.into());
    if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()) {
        response = response.with_header(h);
    }
    for &(name, val) in extra_headers {
        if let Ok(h) = Header::from_bytes(name.as_bytes(), val.as_bytes()) {
            response = response.with_header(h);
        }
    }
    for h in security_headers() {
        response = response.with_header(h);
    }
    let _ = request.respond(response);
}

fn send_empty(request: tiny_http::Request, status: impl Into<StatusCode>) {
    let response = Response::empty(status.into()).with_header(
        Header::from_bytes(&b"Content-Length"[..], &b"0"[..]).expect("static header is valid"),
    );
    let _ = request.respond(response);
}

struct ThreadGuard(Arc<std::sync::atomic::AtomicUsize>);

impl Drop for ThreadGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_origin_allowlist_rejects_lookalikes() {
        assert!(is_local_origin("http://localhost"));
        assert!(is_local_origin("http://localhost:8080"));
        assert!(is_local_origin("https://127.0.0.1:9/x"));
        assert!(is_local_origin("http://[::1]:8080"));
        assert!(is_local_origin("https://[::1]"));

        // Must be rejected.
        assert!(!is_local_origin("http://localhost.evil.com"));
        assert!(!is_local_origin("http://evil.com"));
        assert!(!is_local_origin("https://localhost:@evil.com"));
        assert!(!is_local_origin("http://user@evil.com"));
        assert!(!is_local_origin("file://localhost"));
        assert!(!is_local_origin("localhost"));
        assert!(!is_local_origin(""));
    }

    #[test]
    fn constant_time_eq_matches_normal_equality() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secre"));
        assert!(!constant_time_eq(b"secret", b"secrets"));
        assert!(!constant_time_eq(b"secret", b"sekret"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn active_content_types_are_flagged() {
        for mime in [
            "application/xhtml+xml",
            "text/html",
            "image/svg+xml",
            "application/javascript",
            "text/javascript",
        ] {
            assert!(
                is_active_content_type(mime),
                "{mime} should be treated as active"
            );
        }
        for mime in [
            "image/png",
            "image/jpeg",
            "font/ttf",
            "text/css",
            "application/octet-stream",
        ] {
            assert!(!is_active_content_type(mime), "{mime} should be inert");
        }
    }
}
