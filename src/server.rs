use crate::book::Book;
use crate::web_ui::READER_HTML;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Response, Server, StatusCode};
use url::Url;

/// Embedded HTTP Reader Server.
pub struct ReaderServer {
    book: Arc<Mutex<Book>>,
    port: u16,
}

impl ReaderServer {
    pub fn new(book: Book, port: u16) -> Self {
        Self {
            book: Arc::new(Mutex::new(book)),
            port,
        }
    }

    /// Start listening and serving incoming HTTP requests.
    pub fn listen(&self) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let server = Server::http(&addr)
            .map_err(|e| format!("Failed to start server on {}: {}", addr, e))?;
        println!(
            "🚀 EBook-RS Reader Server listening on http://localhost:{}",
            self.port
        );
        println!("Press Ctrl+C to exit.");

        for request in server.incoming_requests() {
            let url_str = format!("http://localhost{}", request.url());
            let parsed_url =
                Url::parse(&url_str).unwrap_or_else(|_| Url::parse("http://localhost/").unwrap());
            let path = parsed_url.path();

            match path {
                "/" | "/index.html" => {
                    let header =
                        Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                            .unwrap();
                    let response = Response::from_string(READER_HTML).with_header(header);
                    let _ = request.respond(response);
                }
                "/api/book/metadata" => {
                    let book = self.book.lock().unwrap();
                    let json = serde_json::to_string(book.metadata()).unwrap_or_default();
                    let header =
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let _ = request.respond(Response::from_string(json).with_header(header));
                }
                "/api/book/toc" => {
                    let book = self.book.lock().unwrap();
                    let json = serde_json::to_string(book.toc()).unwrap_or_default();
                    let header =
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let _ = request.respond(Response::from_string(json).with_header(header));
                }
                "/api/book/spine" => {
                    let book = self.book.lock().unwrap();
                    let json = serde_json::to_string(book.spine()).unwrap_or_default();
                    let header =
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let _ = request.respond(Response::from_string(json).with_header(header));
                }
                _ if path.starts_with("/api/book/section/") => {
                    let idx_str = path.trim_start_matches("/api/book/section/");
                    let idx: usize = idx_str.parse().unwrap_or(0);
                    let book = self.book.lock().unwrap();

                    match book.get_section(idx) {
                        Ok(sec) => {
                            let header = Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"text/html; charset=utf-8"[..],
                            )
                            .unwrap();
                            let _ = request.respond(
                                Response::from_string(&sec.processed_html).with_header(header),
                            );
                        }
                        Err(err) => {
                            let _ = request.respond(
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

                    let book = self.book.lock().unwrap();
                    let results = book.search(&query);
                    let json = serde_json::to_string(&results).unwrap_or_default();
                    let header =
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let _ = request.respond(Response::from_string(json).with_header(header));
                }
                "/api/book/locations" => {
                    let book = self.book.lock().unwrap();
                    let json = serde_json::to_string(&book.locations).unwrap_or_default();
                    let header =
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let _ = request.respond(Response::from_string(json).with_header(header));
                }
                "/api/book/cover" => {
                    let book = self.book.lock().unwrap();
                    if let Some((bytes, mime)) = book.cover_image() {
                        let header =
                            Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap();
                        let _ = request.respond(Response::from_data(bytes).with_header(header));
                    } else {
                        let _ = request.respond(
                            Response::from_string("No cover found")
                                .with_status_code(StatusCode(404)),
                        );
                    }
                }
                "/api/annotations" => {
                    let book = self.book.lock().unwrap();
                    let json = serde_json::to_string(&book.annotations.list()).unwrap_or_default();
                    let header =
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let _ = request.respond(Response::from_string(json).with_header(header));
                }
                _ => {
                    let _ = request.respond(
                        Response::from_string("404 Not Found").with_status_code(StatusCode(404)),
                    );
                }
            }
        }

        Ok(())
    }
}
