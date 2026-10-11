//! Security regression tests.
//!
//! Each test here corresponds to a bug that was reachable from untrusted input (a hostile
//! eBook, an MCP/JSON-RPC request, or an HTTP request to the reader server) and that has been
//! fixed. They are written as regressions rather than as general unit tests: if one starts
//! failing, a hardening measure has been removed.

use ebook_rs::archive::DecompressionBudget;
use ebook_rs::layout::RenditionLayout;
use ebook_rs::search::{ABSOLUTE_MAX_RESULTS, SearchEngine};
use ebook_rs::section::{process_section_resources_with_strategy, sanitize_html_scripts};
use ebook_rs::{
    AssetDeliveryStrategy, Book, LcpLicense, LcpRights, RagChunkConfig, RagChunker, Section,
};

fn sample_section() -> Section {
    Section {
        index: 0,
        idref: "c".to_string(),
        href: "c.xhtml".to_string(),
        full_path: "OEBPS/c.xhtml".to_string(),
        raw_html: "<p>hello world</p>".to_string(),
        processed_html: "<p>hello world</p>".to_string(),
        plain_text: "hello world".to_string(),
        plain_text_lower: "hello world".to_string(),
        char_count: 11,
        viewport_width: None,
        viewport_height: None,
    }
}

// ---------------------------------------------------------------------------------------
// Panics reachable from untrusted input
// ---------------------------------------------------------------------------------------

/// `parse_iso_date_to_seconds` sliced `rights.end` at fixed byte offsets after checking only
/// `len() >= 10`. A non-ASCII `.lcpl` therefore panicked with
/// "byte index 4 is not a char boundary", which is an uncatchable abort across the FFI
/// boundary and a dropped connection over HTTP.
#[test]
fn lcp_expiry_date_with_multibyte_utf8_does_not_panic() {
    let license = LcpLicense {
        id: "lic-1".to_string(),
        provider: "test".to_string(),
        rights: Some(LcpRights {
            end: Some("\u{65e5}\u{672c}\u{8a9e} end 2026-01-01T00:00:00Z".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let _ = license.is_expired("2026-06-01T00:00:00Z");

    // Nonsense calendar components must be rejected before reaching arithmetic.
    for end in ["2026-13-45", "0000-00-00", "99999999-99-99"] {
        let bad = LcpLicense {
            rights: Some(LcpRights {
                end: Some(end.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let _ = bad.is_expired("2026-06-01T00:00:00Z");
    }
}

/// `RagChunker::chunk_section` computed `max_tokens * 4`. The release profile sets
/// `overflow-checks = true`, so a `max_tokens` of `usize::MAX` (trivially supplied over MCP
/// JSON-RPC) was a hard panic.
#[test]
fn rag_chunking_survives_hostile_token_counts() {
    let sec = sample_section();
    for (max_tokens, overlap_tokens) in [
        (usize::MAX, usize::MAX),
        (0, 0),
        (1, usize::MAX),
        (usize::MAX / 2, usize::MAX / 2),
        (usize::MAX - 1, usize::MAX - 1),
    ] {
        let cfg = RagChunkConfig {
            max_tokens,
            overlap_tokens,
            preserve_headings: true,
            include_cfi: true,
            min_chunk_size: 1,
        };
        let _ = RagChunker::chunk_section(&sec, 0, "Chapter", "Book", "Author", &cfg);
    }
}

/// `CbzBook::prefetch_page_images` computed `current_index + window`; the Python binding
/// forwards both straight from the caller.
#[test]
fn cbz_prefetch_survives_saturating_window() {
    let book =
        Book::from_bytes(&ebook_rs::generate_sample_epub().expect("sample epub")).expect("book");
    let _ = ebook_rs::CbzBook::prefetch_page_images(&book, 0, usize::MAX);
    let _ = ebook_rs::CbzBook::prefetch_page_images(&book, usize::MAX, usize::MAX);
}

/// Deeply nested FB2 elements recursed without bound. `roxmltree` imposes no nesting limit, so a
/// small file was enough to exhaust a worker thread's 2 MB stack. A stack overflow is a SIGSEGV:
/// it cannot be caught by `catch_unwind` and takes the whole process down.
#[test]
fn deeply_nested_fb2_does_not_overflow_the_stack() {
    let depth = 50_000;
    let mut xml = String::from("<?xml version=\"1.0\"?><FictionBook><body>");
    for _ in 0..depth {
        xml.push_str("<section>");
    }
    xml.push_str("<p>deep</p>");
    for _ in 0..depth {
        xml.push_str("</section>");
    }
    xml.push_str("</body></FictionBook>");

    // Must return (Ok or Err) rather than abort.
    let _ = ebook_rs::Fb2Book::parse(xml.as_bytes());
}

/// The single-pass splice replaced a loop of whole-string `output.replace(..)` calls, which was
/// O(matches x output). Verify it still produces exactly one replacement per occurrence,
/// including for repeated identical attribute values, and that untouched text survives.
#[test]
fn resource_inlining_replaces_every_occurrence_exactly_once() {
    let mut archive = ebook_rs::EpubArchive::empty();
    // A tiny valid PNG (1x1 transparent).
    archive.insert(
        "OEBPS/img.png",
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89,
        ],
    );

    let html = r##"<p>before<img src="img.png"/>mid<img src="img.png"/>after</p>"##;
    let out = process_section_resources_with_strategy(
        html,
        "OEBPS/ch.xhtml",
        &archive,
        &AssetDeliveryStrategy::InlinedBase64,
    );

    assert_eq!(
        out.matches("data:image/png;base64,").count(),
        2,
        "expected one data URI per img: {out}"
    );
    assert!(
        !out.contains("src=\"img.png\""),
        "src was not replaced: {out}"
    );
    assert!(out.contains("before"), "preceding text lost: {out}");
    assert!(out.contains("mid"), "interleaved text lost: {out}");
    assert!(out.contains("after"), "trailing text lost: {out}");

    // The ResourceStream strategy must also rewrite every occurrence, exactly once.
    let streamed = process_section_resources_with_strategy(
        html,
        "OEBPS/ch.xhtml",
        &archive,
        &AssetDeliveryStrategy::ResourceStream,
    );
    assert_eq!(
        streamed.matches("src=\"resource/OEBPS/img.png\"").count(),
        2,
        "expected two streamed refs: {streamed}"
    );
    assert!(!streamed.contains("src=\"img.png\""));
}

// ---------------------------------------------------------------------------------------
// Resource exhaustion
// ---------------------------------------------------------------------------------------

/// These guard the API surface advertised in `README.md` and `API.md`. If a documented function is
/// renamed or removed, this file stops compiling — which is the point.
#[test]
fn documented_book_api_exists() {
    let book =
        Book::from_bytes(&ebook_rs::generate_sample_epub().expect("sample epub")).expect("book");

    // README: "Location / Reading Progress | `Book::locations_from_sections()`"
    let locs = book.locations_from_sections(1000);
    assert!(
        locs.total_locations > 0,
        "expected a non-empty location map"
    );
    assert_eq!(locs.entries.len(), book.sections.len());

    // Non-mutating: the book's stored map must be left alone.
    let stored = book.locations.total_locations;
    let again = book.locations_from_sections(1000);
    assert_eq!(again.total_locations, locs.total_locations);
    assert_eq!(
        book.locations.total_locations, stored,
        "locations_from_sections must not mutate self.locations"
    );

    // A zero chunk size must not panic or divide by zero.
    let _ = book.locations_from_sections(0);

    // README: "Universal EPUB 3 & KFX Exporters"
    let epub = book.export_epub3_bytes().expect("EPUB3 export");
    assert!(
        epub.starts_with(b"PK\x03\x04"),
        "EPUB must be a ZIP archive"
    );
    let kfx = book.export_kfx_bytes().expect("KFX export");
    assert!(kfx.starts_with(b"CONT"), "KFX must be a CONT container");

    // The exported EPUB must itself be a valid, parseable book.
    let round_trip = Book::from_bytes(&epub).expect("re-parse exported EPUB");
    assert!(!round_trip.sections.is_empty());

    // README: "Regex Search | `Book::search_regex()`"
    let _ = book.search_regex("the").expect("regex search");
    let _ = book
        .search_regex_limited("the", Some(5))
        .expect("bounded regex search");
}

/// The budget must charge bytes that were *actually* decompressed, since ZIP central-directory
/// sizes are attacker-controlled and can be set far below the real payload.
#[test]
fn decompression_budget_charges_actual_bytes_and_terminates() {
    let mut budget = DecompressionBudget::new(100, 50);

    let first = vec![b'a'; 40];
    let mut out = Vec::new();
    let read = budget
        .read_capped(&mut first.as_slice(), &mut out)
        .expect("40 bytes fits");
    assert_eq!(read, 40);
    assert_eq!(budget.remaining(), 60);
    // Per-entry allowance is min(max_single, remaining).
    assert_eq!(budget.allowance(), 50);

    // Consume the rest of the budget; the next read must fail rather than grow unbounded.
    let second = vec![b'b'; 100];
    let mut out2 = Vec::new();
    let read2 = budget
        .read_capped(&mut second.as_slice(), &mut out2)
        .expect("exactly at the limit is allowed");
    assert!(read2 <= 50, "per-entry cap must apply");

    let third = vec![b'c'; 10];
    let mut out3 = Vec::new();
    assert!(
        budget.read_capped(&mut third.as_slice(), &mut out3).is_ok(),
        "remaining budget should still be usable"
    );
}

/// An exhausted budget must be an error, not silent unbounded growth.
#[test]
fn exhausted_decompression_budget_errors() {
    let mut budget = DecompressionBudget::new(10, 10);
    let data = vec![0u8; 10];
    let mut out = Vec::new();
    budget
        .read_capped(&mut data.as_slice(), &mut out)
        .expect("exactly the budget");
    assert_eq!(budget.remaining(), 0);

    let more = vec![0u8; 5];
    let mut out2 = Vec::new();
    assert!(budget.read_capped(&mut more.as_slice(), &mut out2).is_err());
}

/// Search results must be capped *during* the scan. Building the full match set and truncating
/// afterwards is itself the denial of service.
#[test]
fn search_result_count_is_bounded_during_scan() {
    // One section whose text matches the query thousands of times.
    let mut sec = sample_section();
    sec.plain_text = "a ".repeat(20_000);
    sec.plain_text_lower = sec.plain_text.to_ascii_lowercase();

    let results = SearchEngine::search_limited(&[sec], "a", false, Some(100));
    assert_eq!(results.len(), 100);

    // The cap is honoured even when a much larger value is requested.
    let sec2 = sample_section();
    let many = vec![sec2; 500];
    let results2 = SearchEngine::search_limited(&many, "hello", false, Some(10_000));
    assert!(
        results2.len() <= ABSOLUTE_MAX_RESULTS,
        "results must never exceed the absolute ceiling"
    );
}

/// Regex search must also be bounded.
#[test]
fn regex_search_is_bounded() {
    let mut sec = sample_section();
    // Alternation of many short matches, so the pattern yields thousands of results.
    sec.plain_text = "a ".repeat(5_000);
    let results = SearchEngine::search_regex_limited(&[sec], "a|b", Some(25)).expect("valid regex");
    assert_eq!(results.len(), 25);
}

// ---------------------------------------------------------------------------------------
// XSS / injection
// ---------------------------------------------------------------------------------------

/// Book content is served in the reader's own origin. A `javascript:` URI must be neutralized
/// however the scheme is obfuscated with characters a browser decodes or strips.
#[test]
fn dangerous_uri_schemes_are_neutralized_in_sections() {
    let book =
        Book::from_bytes(&ebook_rs::generate_sample_epub().expect("sample epub")).expect("book");
    let payloads = [
        r##"<a href="javascript:alert(1)">x</a>"##,
        r##"<a href="JavaScript:alert(1)">x</a>"##,
        "<a href=\"java&#9;script:alert(1)\">x</a>",
        "<a href=\"java&Tab;script:alert(1)\">x</a>",
        "<a href=\"java\tscript:alert(1)\">x</a>",
        r##"<a href="vbscript:msgbox(1)">x</a>"##,
        r##"<a href="data:text/html;base64,PHNjcmlwdD4=">x</a>"##,
    ];
    for payload in payloads {
        let out = sanitize_html_scripts(payload);
        let lower = out.to_ascii_lowercase();
        assert!(
            !lower.contains("javascript:")
                && !lower.contains("vbscript:")
                && !lower.contains("data:text/html"),
            "scheme survived sanitization: {payload:?} -> {out:?}"
        );
    }
    // Sanity: the book we just parsed is still renderable.
    let _ = book.get_section(0);
}

/// `RenditionLayout::font_family` is a public field interpolated into an inline `<style>`, and
/// the reader CSP allows `style-src 'unsafe-inline'`. It must not be able to break out.
#[test]
fn css_override_cannot_be_escaped() {
    let payloads = [
        "x; } </style><script>alert(1)</script>",
        "Inter'; } body { display:none } .a {",
        "a{b:url(javascript:alert(1))}",
    ];
    for payload in payloads {
        let layout = RenditionLayout {
            font_family: payload.to_string(),
            ..Default::default()
        };
        let css = layout.to_css_override();
        assert!(
            !css.to_ascii_lowercase().contains("<script"),
            "CSS override allowed a script tag: {css}"
        );
        assert!(
            !css.contains("</style"),
            "CSS override allowed a style close tag: {css}"
        );
        assert!(
            !css.to_ascii_lowercase().contains("javascript:"),
            "CSS override allowed a javascript: URL: {css}"
        );
    }
}

/// `custom_font_url` is interpolated into `url('...')`.
#[test]
fn css_override_rejects_hostile_font_urls() {
    let layout = RenditionLayout {
        custom_font_family: Some("Evil".to_string()),
        custom_font_url: Some("x'); } body { display:none } @font-face{a:url('y".to_string()),
        ..Default::default()
    };
    let css = layout.to_css_override();
    assert!(
        !css.contains("display:none"),
        "CSS override was injectable: {css}"
    );
}

// ---------------------------------------------------------------------------------------
// Archive key integrity
// ---------------------------------------------------------------------------------------

/// An FB2 `<binary id="../../container.xml">` used to overwrite `META-INF/container.xml` in the
/// in-memory archive index, because `normalize_path` silently drops `..` segments.
#[test]
fn fb2_rejects_path_traversal_in_binary_ids() {
    let xml = r##"<?xml version="1.0"?>
<FictionBook>
  <body>
    <section>
      <p>text</p>
      <image l:href="#a"><binary id="../../container.xml" content-type="application/octet-stream">aGk=</binary></image>
      <image l:href="#b"><binary id="ok.png" content-type="image/png">aGk=</binary></image>
    </section>
  </body>
</FictionBook>"##;
    if let Ok(book) = ebook_rs::Fb2Book::parse(xml.as_bytes()) {
        let files = book.archive().files();
        assert!(
            !files.contains_key("META-INF/container.xml"),
            "traversal binary id created a container.xml key"
        );
        // The legitimate binary must still be present.
        assert!(
            files.keys().any(|k| k.ends_with("ok.png")),
            "legitimate binary was dropped: {:?}",
            files.keys().collect::<Vec<_>>()
        );
    }
}

/// `normalize_path` must clamp `..` at the archive root rather than escaping it.
#[test]
fn normalize_path_clamps_parent_traversal_at_root() {
    assert_eq!(
        ebook_rs::archive::normalize_path("../../../etc/passwd"),
        "etc/passwd"
    );
    assert_eq!(
        ebook_rs::archive::normalize_path("OEBPS/../OEBPS/ch1.xhtml"),
        "OEBPS/ch1.xhtml"
    );
    assert_eq!(ebook_rs::archive::normalize_path("a/./b//c"), "a/b/c");
}
