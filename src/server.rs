use crate::annotations::Annotation;
use crate::book::Book;
use crate::web_ui::READER_HTML;
use std::sync::{Arc, RwLock};
use tiny_http::{Header, Response, Server, StatusCode};
use url::Url;

/// Embedded HTTP Reader Server.
pub struct ReaderServer {
    book: Arc<RwLock<Book>>,
    port: u16,
}

impl ReaderServer {
    pub fn new(book: Book, port: u16) -> Self {
        Self {
            book: Arc::new(RwLock::new(book)),
            port,
        }
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
        println!("Press Ctrl+C to exit.");

        let active_threads = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        for mut request in server.incoming_requests() {
            let book_arc = Arc::clone(&self.book);
            let active_cnt = Arc::clone(&active_threads);

            if active_cnt.load(std::sync::atomic::Ordering::Relaxed) >= 64 {
                let res = Response::from_string("503 Service Unavailable: Server busy")
                    .with_status_code(StatusCode(503));
                let _ = request.respond(res);
                continue;
            }

            active_cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::thread::spawn(move || {
                let _guard = ThreadGuard(active_cnt);
                let url_str = format!("http://localhost{}", request.url());
                let parsed_url = Url::parse(&url_str)
                    .unwrap_or_else(|_| Url::parse("http://localhost/").unwrap());
                let path = parsed_url.path();

                match path {
                    "/" | "/index.html" => {
                        let header = Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"text/html; charset=utf-8"[..],
                        )
                        .unwrap();
                        send_response(
                            request,
                            Response::from_string(READER_HTML).with_header(header),
                        );
                    }
                    "/api/mcp" => {
                        let mut body = String::new();
                        let _ = request.as_reader().read_to_string(&mut body);
                        if let Ok(mcp_req) =
                            serde_json::from_str::<crate::mcp::JsonRpcRequest>(&body)
                        {
                            if let Some(resp_val) = crate::mcp::process_mcp_request(&mcp_req) {
                                let json_resp =
                                    serde_json::to_string(&resp_val).unwrap_or_default();
                                let header = Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"application/json"[..],
                                )
                                .unwrap();
                                send_response(
                                    request,
                                    Response::from_string(json_resp).with_header(header),
                                );
                                return;
                            }
                        }
                        let header =
                            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                .unwrap();
                        send_response(request, Response::from_string("{}").with_header(header));
                    }
                    "/api/book/metadata" => {
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        let json = serde_json::to_string(book.metadata()).unwrap_or_default();
                        let header =
                            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                .unwrap();
                        send_response(request, Response::from_string(json).with_header(header));
                    }
                    "/api/book/toc" => {
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        let json = serde_json::to_string(book.toc()).unwrap_or_default();
                        let header =
                            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                .unwrap();
                        send_response(request, Response::from_string(json).with_header(header));
                    }
                    "/api/book/spine" => {
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        let json = serde_json::to_string(book.spine()).unwrap_or_default();
                        let header =
                            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                .unwrap();
                        send_response(request, Response::from_string(json).with_header(header));
                    }
                    _ if path.starts_with("/api/book/section/") => {
                        let idx_str = path.trim_start_matches("/api/book/section/");
                        let idx: usize = idx_str.parse().unwrap_or(0);
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());

                        match book.get_section(idx) {
                            Ok(sec) => {
                                let header = Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"text/html; charset=utf-8"[..],
                                )
                                .unwrap();
                                send_response(
                                    request,
                                    Response::from_string(&sec.processed_html).with_header(header),
                                );
                            }
                            Err(err) => {
                                send_response(
                                    request,
                                    Response::from_string(err).with_status_code(StatusCode(404)),
                                );
                            }
                        }
                    }
                    _ if path.starts_with("/resource/") || path.starts_with("/api/resource/") => {
                        let clean_path = path
                            .strip_prefix("/api/resource/")
                            .or_else(|| path.strip_prefix("/resource/"))
                            .unwrap_or(path);
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());

                        match book.get_resource_bytes(clean_path) {
                            Ok((bytes, mime)) => {
                                let header =
                                    Header::from_bytes(&b"Content-Type"[..], mime.as_bytes())
                                        .unwrap();
                                send_response(
                                    request,
                                    Response::from_data(bytes).with_header(header),
                                );
                            }
                            Err(err) => {
                                send_response(
                                    request,
                                    Response::from_string(err).with_status_code(StatusCode(404)),
                                );
                            }
                        }
                    }
                    "/api/book/search" => {
                        let query = parsed_url
                            .query_pairs()
                            .find(|(k, _)| k == "q")
                            .map(|(_, v)| v.to_string())
                            .unwrap_or_default();

                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        let mut results =
                            crate::search::SearchEngine::search(&book.sections, &query, false);
                        if results.len() > 500 {
                            results.truncate(500);
                        }

                        let json = serde_json::to_string(&results).unwrap_or_default();
                        let header =
                            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                .unwrap();
                        send_response(request, Response::from_string(json).with_header(header));
                    }
                    "/api/book/locations" => {
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        let json = serde_json::to_string(&book.locations).unwrap_or_default();
                        let header =
                            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                .unwrap();
                        send_response(request, Response::from_string(json).with_header(header));
                    }
                    "/api/book/cover" => {
                        let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                        if let Some((bytes, mime)) = book.cover_image() {
                            let header =
                                Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap();
                            send_response(request, Response::from_data(bytes).with_header(header));
                        } else {
                            send_response(
                                request,
                                Response::from_string("No cover found")
                                    .with_status_code(StatusCode(404)),
                            );
                        }
                    }
                    "/api/annotations" => {
                        if request.method() == &tiny_http::Method::Post {
                            let mut body_str = String::new();
                            use std::io::Read;
                            let _ = request
                                .as_reader()
                                .take(2 * 1024 * 1024)
                                .read_to_string(&mut body_str);
                            if let Ok(ann) = serde_json::from_str::<Annotation>(&body_str) {
                                let mut book = book_arc.write().unwrap_or_else(|e| e.into_inner());
                                book.annotations.add(ann);
                                let header = Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"application/json"[..],
                                )
                                .unwrap();
                                send_response(
                                    request,
                                    Response::from_string("{\"status\":\"ok\"}")
                                        .with_header(header),
                                );
                            } else {
                                send_response(
                                    request,
                                    Response::from_string("Invalid JSON or payload too large")
                                        .with_status_code(StatusCode(400)),
                                );
                            }
                        } else {
                            let book = book_arc.read().unwrap_or_else(|e| e.into_inner());
                            let json =
                                serde_json::to_string(&book.annotations.list()).unwrap_or_default();
                            let header =
                                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                    .unwrap();
                            send_response(request, Response::from_string(json).with_header(header));
                        }
                    }
                    _ => {
                        send_response(
                            request,
                            Response::from_string("404 Not Found")
                                .with_status_code(StatusCode(404)),
                        );
                    }
                }
            });
        }

        Ok(())
    }
}

fn send_response<R: std::io::Read>(request: tiny_http::Request, response: Response<R>) {
    let res = response
        .with_header(Header::from_bytes(&b"X-Content-Type-Options"[..], &b"nosniff"[..]).unwrap())
        .with_header(Header::from_bytes(&b"X-Frame-Options"[..], &b"SAMEORIGIN"[..]).unwrap())
        .with_header(
            Header::from_bytes(
                &b"Content-Security-Policy"[..],
                &b"default-src 'self' 'unsafe-inline' data: blob:"[..],
            )
            .unwrap(),
        );
    let _ = request.respond(res);
}

struct ThreadGuard(Arc<std::sync::atomic::AtomicUsize>);

impl Drop for ThreadGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}
