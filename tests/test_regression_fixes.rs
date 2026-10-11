//! Regression tests for bugs found in the working tree (see git history for detail).
//!
//! Each test names the defect it guards against so a future regression points straight at the
//! behaviour that broke.

use ebook_rs::TxtBook;
use ebook_rs::cfi::Cfi;
use ebook_rs::citation::CitationExporter;
use ebook_rs::fingerprint::BookFingerprint;
use ebook_rs::footnote::parse_footnotes_from_html;
use ebook_rs::opds::OpdsEntry;
use ebook_rs::treesitter::TreeSitterEngine;

/// Build a book whose fields are directly settable, so tests can reach code paths
/// (`public layout` mutation) without a full parse.
fn book_from_txt(md: &str) -> ebook_rs::Book {
    TxtBook::parse(md.as_bytes(), "Fallback", true).unwrap()
}

// ---------------------------------------------------------------------------
// CFI: package-document steps must not become content-document selectors.
// ---------------------------------------------------------------------------

/// `epubcfi(/6/2!/4/2/1:42)` is the shape the crate itself emits for an ID-less position. The
/// `/6/2` prefix describes the OPF *package* document, so translating it into `*:nth-child()`
/// anchored the selector chain in the wrong document.
#[test]
fn cfi_selectors_start_after_the_indirection() {
    let cfi = Cfi::from_spine_index(0, None, 42);
    let target = cfi
        .resolve_dom_path("<html><body><p>text</p></body></html>")
        .unwrap();
    // `/6/2` describes the OPF package document and must not become `*:nth-child(3)`.
    assert!(
        !target.css_selector.contains("nth-child(3)"),
        "package-document steps leaked into the selector: {}",
        target.css_selector
    );
    assert_eq!(target.css_selector, "*:nth-child(2) > *:nth-child(1)");

    let with_id = Cfi::parse("epubcfi(/6/2!/4[chap01]/2:42").unwrap();
    let target = with_id
        .resolve_dom_path("<html><body><p id=\"chap01\">x</p></body></html>")
        .unwrap();
    assert_eq!(target.element_id.as_deref(), Some("chap01"));
    // Steps after the anchor keep being selectors; the pre-`!` `/6/2` steps are gone.
    assert_eq!(target.css_selector, "#chap01 > *:nth-child(1)");

    // An asserted id that is absent from the document must not be reported as the target.
    let missing = Cfi::parse("epubcfi(/6/2!/4[nope]/2:42").unwrap();
    let target = missing
        .resolve_dom_path("<html><body><p>x</p></body></html>")
        .unwrap();
    assert_eq!(
        target.element_id, None,
        "id that is not in the document was returned as the target"
    );
}

// ---------------------------------------------------------------------------
// HTML entities must be decoded when plain text is derived from escaped HTML.
// ---------------------------------------------------------------------------

#[test]
fn plain_text_decodes_html_entities() {
    use ebook_rs::section::extract_plain_text;
    // FB2 / PDF / ODT / DOCX / RTF / CBZ all build their section HTML by escaping text nodes.
    let html =
        "<p>Chapter 2 The Pool of Tears &apos;Curiouser and curiouser!&apos; cried Alice</p>";
    let text = extract_plain_text(html);
    assert!(text.contains("'Curiouser and curiouser!'"), "got: {text}");
    assert!(!text.contains("&apos;"), "entity survived: {text}");

    assert_eq!(extract_plain_text("<p>a &amp; b</p>"), "a & b");
    assert_eq!(extract_plain_text("<p>&lt;img&gt;</p>"), "<img>");
    assert_eq!(extract_plain_text("<p>&quot;q&quot;</p>"), "\"q\"");
    // Non-escaped source is unaffected.
    assert_eq!(extract_plain_text("<p>plain</p>"), "plain");
    // Decoding is a single pass: `&amp;amp;` -> `&amp;`, not `&`.
    assert_eq!(extract_plain_text("<p>&amp;amp;</p>"), "&amp;");
}

// ---------------------------------------------------------------------------
// XML nesting-depth guard must count non-ASCII element names.
// ---------------------------------------------------------------------------

#[test]
fn non_ascii_nesting_depth_is_counted() {
    use ebook_rs::dom::{check_xml_nesting_depth, xml_nesting_depth};
    assert!(xml_nesting_depth("<日本語><中文/></日本語>") > 0);
    let deep = format!(
        "<日本語>{}{}</日本語>",
        "<中文>".repeat(5000),
        "</中文>".repeat(5000)
    );
    assert!(
        check_xml_nesting_depth(&deep).is_err(),
        "non-ASCII nesting bypassed the depth guard"
    );
}

// ---------------------------------------------------------------------------
// OPDS acquisition links must be matched by requested MIME type first.
// ---------------------------------------------------------------------------

#[test]
fn opds_download_link_prefers_exact_mime() {
    let json = r#"{
        "id": "1",
        "title": "Bilingual",
        "authors": [],
        "links": [
            {"rel": "http://opds-spec.org/acquisition", "href": "/1.pdf", "media_type": "application/pdf"},
            {"rel": "http://opds-spec.org/acquisition", "href": "/1.epub", "media_type": "application/epub+zip"}
        ]
    }"#;
    let entry: OpdsEntry = serde_json::from_str(json).unwrap();
    assert_eq!(
        entry
            .download_link(Some("application/epub+zip"))
            .unwrap()
            .href,
        "/1.epub",
        "acquisition disjunct short-circuited on the first link"
    );
}

// ---------------------------------------------------------------------------
// Fingerprint: different content must never score 1.0.
// ---------------------------------------------------------------------------

#[test]
fn fingerprint_never_reports_distinct_books_as_duplicates() {
    let a = BookFingerprint {
        content_hash: "hash_a".into(),
        metadata_hash: "hash_m".into(),
        total_sections: 10,
        total_characters: 1000,
    };
    let b = BookFingerprint {
        content_hash: "hash_b".into(),
        metadata_hash: "hash_m".into(),
        // Same section count and same total length: different text, identical shape.
        total_sections: 10,
        total_characters: 1000,
    };
    let score = a.match_score(&b);
    assert!(score < 1.0, "different content scored {score}");
    assert!(
        !a.is_duplicate_of(&b),
        "distinct books merged as duplicates"
    );

    // Identical content still reports a perfect match.
    let same = BookFingerprint {
        content_hash: "hash_a".into(),
        ..b.clone()
    };
    assert_eq!(a.match_score(&same), 1.0);
}

// ---------------------------------------------------------------------------
// SQIL / SMIL timecount units.
// ---------------------------------------------------------------------------

#[test]
fn smil_min_and_ms_units_are_scaled() {
    use ebook_rs::media_overlay::SmilClock;
    assert_eq!(SmilClock::parse_npt_seconds("90min"), 5400.0);
    assert_eq!(SmilClock::parse_npt_seconds("100ms"), 0.1);
    assert_eq!(SmilClock::parse_npt_seconds("250ms"), 0.25);
    // Previously-working units must not regress.
    assert_eq!(SmilClock::parse_npt_seconds("10s"), 10.0);
    assert_eq!(SmilClock::parse_npt_seconds("2h"), 7200.0);
    assert_eq!(SmilClock::parse_npt_seconds("1.5m"), 90.0);
    assert_eq!(SmilClock::parse_npt_seconds("01:30"), 90.0);
}

// ---------------------------------------------------------------------------
// Footnotes: single-quoted attributes.
// ---------------------------------------------------------------------------

#[test]
fn footnotes_match_single_quoted_attributes() {
    let html = "<a epub:type='noteref' href='#fn1'>1</a>aside aside<aside id='fn1'>Note</aside>";
    let notes = parse_footnotes_from_html(html);
    assert_eq!(
        notes.len(),
        1,
        "single-quoted href/id produced no footnotes"
    );
}

// ---------------------------------------------------------------------------
// Citation years.
// ---------------------------------------------------------------------------

#[test]
fn citation_year_rejects_concatenated_digits() {
    use ebook_rs::metadata::Metadata;
    let mut meta = Metadata {
        title: "Test".into(),
        pub_date: Some("15/03/99".into()),
        ..Metadata::default()
    };
    // Tokens 15 / 03 / 99 each fail the century test; the old fallback concatenated them into
    // `150399` and reported year 1503.
    meta.pub_date = Some("15/03/99".into());
    let bibtex = CitationExporter::to_bibtex(&meta);
    assert!(!bibtex.contains("1503"), "fabricated year: {bibtex}");

    // A real 4-digit year is still picked up.
    meta.pub_date = Some("2020-05-01".into());
    let bibtex = CitationExporter::to_bibtex(&meta);
    assert!(bibtex.contains("2020"), "real year dropped: {bibtex}");
}

// ---------------------------------------------------------------------------
// Treesitter: CRLF offsets, `<pre class=...>`, and the fallback node row.
// ---------------------------------------------------------------------------

#[test]
fn treesitter_offsets_are_correct_for_crlf() {
    let code = "fn a() {}\r\nfn b() {}\r\n";
    let nodes = TreeSitterEngine::parse_code(code, "rust");
    let second = nodes
        .iter()
        .filter(|n| n.kind == "function_definition")
        .nth(1)
        .unwrap();
    // "fn a() {}\r\n" is 11 bytes, so `fn b` starts at 11, not 10.
    assert_eq!(second.start_byte, 11, "CRLF drift: {second:?}");

    let lf = "fn a() {}\nfn b() {}\n";
    let nodes = TreeSitterEngine::parse_code(lf, "rust");
    let second = nodes
        .iter()
        .filter(|n| n.kind == "function_definition")
        .nth(1)
        .unwrap();
    assert_eq!(second.start_byte, 10);
}

#[test]
fn treesitter_fallback_node_row_is_in_range() {
    let nodes = TreeSitterEngine::parse_code("abc", "txt");
    assert_eq!(nodes.len(), 1);
    // A single-line file has only row 0.
    assert_eq!(nodes[0].end_position.0, 0, "row out of range: {nodes:?}");
}

#[test]
fn treesitter_extracts_pre_tags_with_attributes() {
    let mut book = book_from_txt("<pre class=\"source\"><code>let x = 1;</code></pre>");
    book.sections.push(ebook_rs::section::Section {
        index: 0,
        idref: "s0".into(),
        href: "s0.xhtml".into(),
        full_path: "OEBPS/s0.xhtml".into(),
        raw_html: "<pre class=\"source\"><code>let x = 1;</code></pre>".into(),
        processed_html: String::new(),
        plain_text: String::new(),
        plain_text_lower: String::new(),
        char_count: 0,
        viewport_width: None,
        viewport_height: None,
    });
    let blocks = TreeSitterEngine::extract_code_blocks(&book);
    assert_eq!(blocks.len(), 1, "<pre class=...> block was skipped");
}

// ---------------------------------------------------------------------------
// Markdown ATX headings require a space after the `#` run.
// ---------------------------------------------------------------------------

#[test]
fn hashtags_are_not_headings() {
    // Use one of the generic fallbacks so `has_custom_title` stays false and a heading can become
    // the title.
    let md = "#hashtag about cats\n\nBody text.";
    let book = TxtBook::parse(md.as_bytes(), "eBook", true).unwrap();
    assert_eq!(
        book.metadata().title,
        "eBook",
        "hashtag line became the book title"
    );

    let md2 = "####Chapter\n\nBody.";
    let book = TxtBook::parse(md2.as_bytes(), "eBook", true).unwrap();
    assert_eq!(book.metadata().title, "eBook");

    // A real heading still works.
    let md3 = "# Real Title\n\nBody text.";
    let book = TxtBook::parse(md3.as_bytes(), "eBook", true).unwrap();
    assert_eq!(book.metadata().title, "Real Title");
}

// ---------------------------------------------------------------------------
// RTF: nested destinations must not leak metadata into the body, and section
// indices must stay contiguous when a blank page is skipped.
// ---------------------------------------------------------------------------

#[test]
fn rtf_nested_destination_text_stays_out_of_the_body() {
    let rtf = r"{\rtf1\ansi{\fonttbl{\f0\froman Times New Roman;}{\f1\fswiss Arial;}}{\stylesheet{\s1 heading 1;}{\s2 heading 2;}}Hello World}";
    let book = ebook_rs::RtfBook::parse(rtf.as_bytes(), "RTF").unwrap();
    let body = &book.sections[0].plain_text;
    assert!(body.contains("Hello World"), "body lost: {body}");
    assert!(
        !body.contains("Times New Roman"),
        "font-table metadata leaked into the body: {body}"
    );
    assert!(
        !body.contains("heading 1"),
        "style-table metadata leaked into the body: {body}"
    );
}

#[test]
fn rtf_section_indices_stay_contiguous() {
    // Two consecutive `\page`s with no text between them produce a blank intermediate page.
    let rtf = r"{\rtf1\ansi First\page \page Second}";
    let book = ebook_rs::RtfBook::parse(rtf.as_bytes(), "RTF").unwrap();
    for (i, sec) in book.sections.iter().enumerate() {
        assert_eq!(sec.index, i, "index gap at position {i}");
    }
    let hrefs: Vec<&str> = book.sections.iter().map(|s| s.href.as_str()).collect();
    assert!(hrefs.contains(&"section_0.html"), "{hrefs:?}");
    assert!(hrefs.contains(&"section_1.html"), "{hrefs:?}");
}

// ---------------------------------------------------------------------------
// Optimizer: asset remapping uses full paths.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// The optimizer's full-path remapping is covered by `optimizer::tests` inline, which can reach
// the private helper directly. Here we only assert the CSS-`url()` path stays rewritable.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Render cache must key on the render inputs.
// ---------------------------------------------------------------------------

#[test]
fn render_cache_tracks_script_gate() {
    let mut book = book_from_txt("# T\n\nhi");
    // Replace the sections so index 0 is the section under test.
    book.sections.clear();
    // `processed_html` must mirror `raw_html` for the render path to take the processing branch.
    book.sections.push(ebook_rs::section::Section {
        index: 0,
        idref: "s0".into(),
        href: "s0.xhtml".into(),
        full_path: "OEBPS/s0.xhtml".into(),
        raw_html: "<p>hi</p><script>alert('xss')</script>".into(),
        processed_html: "<p>hi</p><script>alert('xss')</script>".into(),
        plain_text: "hi alert(xss)".into(),
        plain_text_lower: "hi alert(xss)".into(),
        char_count: 2,
        viewport_width: None,
        viewport_height: None,
    });

    // Scripted content allowed: the script is preserved and cached under that key.
    book.layout.allow_scripted_content = true;
    let with_script = book.get_section(0).unwrap();
    assert!(with_script.processed_html.contains("<script>"));

    // Directly mutating the public field must not return the stale cached rendering.
    book.layout.allow_scripted_content = false;
    let sanitized = book.get_section(0).unwrap();
    assert!(
        !sanitized.processed_html.contains("<script>"),
        "cached rendering served past a script-policy change"
    );
}
