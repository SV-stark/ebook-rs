use ebook_rs::{
    Book,
    deobfuscate::{FontDeobfuscator, deobfuscate_adobe, deobfuscate_idpf},
    generate_sample_epub,
    nav::{parse_landmarks, parse_page_list},
};

#[test]
fn test_epub3_landmarks_and_page_list_parsing() {
    let html = r#"
        <html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
            <body>
                <nav epub:type="landmarks">
                    <h2>Landmarks</h2>
                    <ol>
                        <li><a epub:type="cover" href="cover.xhtml">Cover</a></li>
                        <li><a epub:type="toc" href="toc.xhtml">Table of Contents</a></li>
                        <li><a epub:type="bodymatter" href="chapter1.xhtml">Start Reading</a></li>
                    </ol>
                </nav>
                <nav epub:type="page-list">
                    <h2>Print Page List</h2>
                    <ol>
                        <li><a href="chap1.xhtml#page1">1</a></li>
                        <li><a href="chap1.xhtml#page2">2</a></li>
                    </ol>
                </nav>
            </body>
        </html>
    "#;

    let landmarks = parse_landmarks(html);
    assert_eq!(landmarks.len(), 3);
    assert_eq!(landmarks[0].epub_type, "cover");
    assert_eq!(landmarks[0].href, "cover.xhtml");

    let page_list = parse_page_list(html);
    assert_eq!(page_list.len(), 2);
    assert_eq!(page_list[0].page, "1");
    assert_eq!(page_list[0].href, "chap1.xhtml#page1");
}

#[test]
fn test_before_display_transformation_hooks() {
    let bytes = generate_sample_epub().unwrap();
    let mut book = Book::from_bytes(&bytes).unwrap();

    // Register a pre-display hook that injects custom watermarks into section HTML
    book.register_before_display_hook(|html, path| {
        html.push_str(&format!(
            "<div class=\"watermark\">Processed: {}</div>",
            path
        ));
    });

    let sec0 = book.get_section(0).expect("Section 0");
    assert!(sec0.processed_html.contains("Processed: OEBPS/ch1.xhtml"));
}

#[test]
fn test_font_deobfuscation_idpf_and_adobe() {
    let identifier = "urn:uuid:9780694014545";
    let original = [0x4F, 0x74, 0x74, 0x6F, 0x00, 0x01, 0x00, 0x00];

    // Test IDPF XOR de-obfuscation roundtrip
    let mut encrypted = original;
    deobfuscate_idpf(&mut encrypted, identifier);
    assert_ne!(encrypted, original); // Obfuscated

    deobfuscate_idpf(&mut encrypted, identifier);
    assert_eq!(encrypted, original); // De-obfuscated back to original!

    // Test Adobe XOR de-obfuscation roundtrip
    let mut encrypted_adobe = original;
    deobfuscate_adobe(&mut encrypted_adobe, identifier);
    assert_ne!(encrypted_adobe, original);

    deobfuscate_adobe(&mut encrypted_adobe, identifier);
    assert_eq!(encrypted_adobe, original);

    // Test encryption.xml parsing
    let xml = r#"
        <encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
            <EncryptedData>
                <EncryptionMethod Algorithm="http://www.idpf.org/2008/embedding"/>
                <CipherData>
                    <CipherReference URI="OEBPS/Fonts/custom.otf"/>
                </CipherData>
            </EncryptedData>
        </encryption>
    "#;
    let deobf = FontDeobfuscator::parse_encryption_xml(xml);
    assert!(deobf.is_encrypted("OEBPS/Fonts/custom.otf"));
}

#[test]
fn test_script_content_sanitization_and_sandboxing() {
    let bytes = generate_sample_epub().unwrap();
    let book = Book::from_bytes(&bytes).unwrap();

    // Default: allow_scripted_content is false -> scripts are stripped
    let mut sec = book.get_section(0).unwrap();
    sec.processed_html.push_str("<script>alert('XSS')</script><div onload=\"evil()\">Text</div><a href=\"javascript:bad()\">Click</a>");
    sec.strip_script_content();

    assert!(!sec.processed_html.contains("<script>"));
    assert!(!sec.processed_html.contains("alert('XSS')"));
    assert!(!sec.processed_html.contains("javascript:bad()"));
    assert!(sec.processed_html.contains("Text"));
}

#[test]
fn test_fxl_viewport_scaling_and_meta_parsing() {
    let html = r#"
        <html>
            <head>
                <meta name="viewport" content="width=1024, height=768"/>
            </head>
            <body>Fixed Layout Page</body>
        </html>
    "#;

    let (w, h) = ebook_rs::section::parse_viewport_meta(html);
    assert_eq!(w, Some(1024.0));
    assert_eq!(h, Some(768.0));

    let layout = ebook_rs::RenditionLayout::default();
    let (scale, css) = layout
        .compute_fxl_scale(1024.0, 768.0, 512.0, 384.0)
        .expect("Scale computation");

    assert_eq!(scale, 0.5);
    assert!(css.contains("transform: scale(0.5)"));
    assert!(css.contains("width: 1024px; height: 768px;"));
}

#[test]
fn test_asset_delivery_strategy_and_resource_streaming() {
    let bytes = generate_sample_epub().unwrap();
    let mut book = Book::from_bytes(&bytes).unwrap();

    // Set AssetDeliveryStrategy to ResourceStream
    book.layout.asset_delivery = ebook_rs::AssetDeliveryStrategy::ResourceStream;
    let sec = book.get_section(0).unwrap();

    // Verify HTML contains resource streaming URLs instead of Base64 Data URIs
    assert!(!sec.processed_html.contains("data:image/"));

    // Verify raw resource bytes retrieval API
    let style_res = book.get_resource_bytes("OEBPS/style.css");
    assert!(style_res.is_ok());
    let (style_bytes, mime) = style_res.unwrap();
    assert!(!style_bytes.is_empty());
    assert_eq!(mime, "text/css");
}

#[test]
fn test_footnote_and_endnote_extraction() {
    let html = r##"
        <html xmlns:epub="http://www.idpf.org/2007/ops">
            <body>
                <p>Quantum Mechanics<a id="ref1" href="#fn1" epub:type="noteref">1</a> is fascinating.</p>
                <aside id="fn1" epub:type="footnote">
                    <p>Footnote 1: A fundamental theory in physics.</p>
                </aside>
            </body>
        </html>
    "##;

    let footnotes = ebook_rs::footnote::parse_footnotes_from_html(html);
    assert_eq!(footnotes.len(), 1);
    assert_eq!(footnotes[0].target_id, "fn1");
    assert_eq!(footnotes[0].label, "1");
    assert!(footnotes[0].plain_text.contains("fundamental theory"));
}

#[test]
#[cfg(feature = "opds")]
fn test_opds_feed_parsing_atom_xml_and_json() {
    let atom_xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <id>urn:opds:catalog</id>
            <title>Standard Ebooks Catalog</title>
            <entry>
                <id>urn:isbn:9781234567890</id>
                <title>Alice in Wonderland</title>
                <author><name>Lewis Carroll</name></author>
                <summary>A classic fantasy novel.</summary>
                <link rel="http://opds-spec.org/acquisition" href="https://example.com/alice.epub" type="application/epub+zip"/>
            </entry>
        </feed>
    "#;

    let feed = ebook_rs::OpdsFeed::parse_atom_xml(atom_xml).expect("Parse OPDS Atom XML");
    assert_eq!(feed.title, "Standard Ebooks Catalog");
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.entries[0].title, "Alice in Wonderland");
    assert_eq!(feed.entries[0].authors, vec!["Lewis Carroll"]);
    assert!(feed.entries[0].download_link(None).is_some());
}

#[test]
fn test_http_range_request_header_formatting() {
    let req = ebook_rs::HttpRangeRequest::new("https://example.com/book.epub", 0, Some(1023));
    let (header_name, header_val) = req.to_range_header();
    assert_eq!(header_name, "Range");
    assert_eq!(header_val, "bytes=0-1023");

    let parsed = ebook_rs::HttpRangeRequest::parse_range_header("bytes=2048-4096").unwrap();
    assert_eq!(parsed, (2048, Some(4096)));
}

#[test]
fn test_reflow_paginator_offline_page_breaks() {
    let paginator = ebook_rs::ReflowPaginator::new(16, 1.6, 800, 600, 32);
    let sample_text = "Quantum Mechanics is a fundamental theory in physics that describes the physical properties of nature at the scale of atoms and subatomic particles.".repeat(30);

    let page_map = paginator.paginate_text(&sample_text);
    assert!(page_map.total_pages > 1);
    assert_eq!(page_map.page_ranges[0].page_number, 1);
    assert_eq!(page_map.page_ranges[0].start_char, 0);
    assert!(page_map.page_ranges[0].end_char > 0);
}

#[test]
fn test_reading_analytics_nlp_engine() {
    let text = "Quantum physics explores quantum mechanics and subatomic particles. Quantum entanglement is fascinating.";
    let analytics = ebook_rs::ReadingAnalytics::analyze_text(text);

    assert_eq!(analytics.word_count, 12);
    assert!(analytics.reading_time_minutes > 0.0);
    assert!(!analytics.top_keywords.is_empty());
    assert_eq!(analytics.top_keywords[0].0, "quantum");
}

#[test]
fn test_remote_zip_central_directory_streamer() {
    let bytes = generate_sample_epub().unwrap();
    let entries = ebook_rs::ZipHeaderReader::parse_central_directory(&bytes)
        .expect("Parse Central Directory");

    assert!(!entries.is_empty());
    let (header, val) = entries[0].to_http_range_header();
    assert_eq!(header, "Range");
    assert!(val.starts_with("bytes="));
}

#[test]
fn test_multibyte_utf8_cjk_script_sanitization() {
    let cjk_html = "<html><body><h1>量子力学</h1><script>alert('xss');</script><p>الفيزياء العربية</p></body></html>";
    let sanitized = ebook_rs::section::sanitize_html_scripts(cjk_html);
    assert!(!sanitized.contains("<script>"));
    assert!(sanitized.contains("量子力学"));
    assert!(sanitized.contains("الفيزياء العربية"));
}

#[test]
fn test_unclosed_style_tag_plain_text_extraction() {
    let unclosed_html = "<html><body><style>body { color: red; } <p>Important text content</p>";
    let text = ebook_rs::section::extract_plain_text(unclosed_html);
    assert!(text.contains("Important text content"));
}

#[test]
fn test_cfi_missing_indirection_error() {
    let cfi = ebook_rs::Cfi::parse("epubcfi(/6/4)").unwrap();
    assert!(cfi.try_spine_index().is_err());
}

#[test]
fn test_data_src_attribute_isolation() {
    let html = r#"<img data-src="lazy.jpg" src="real.jpg"/>"#;
    let sanitized = ebook_rs::section::sanitize_html_scripts(html);
    assert!(sanitized.contains("data-src=\"lazy.jpg\""));
}

#[test]
fn test_deep_cfi_dom_element_resolver() {
    let cfi = ebook_rs::Cfi::parse("epubcfi(/6/4[chap01]!/4/2/1:10)").unwrap();
    let target = cfi.resolve_dom_path("<p id='chap01'>Text</p>").unwrap();
    assert_eq!(target.element_id, Some("chap01".to_string()));
    assert_eq!(target.char_offset, 10);
}

#[test]
fn test_w3c_web_annotation_export() {
    let mut manager = ebook_rs::AnnotationManager::new();
    manager.create_highlight(
        "epubcfi(/6/4!/4/2/1:0)",
        "#ffff00",
        Some("quote"),
        Some("my note"),
    );

    let w3c_json = manager.to_w3c_json().unwrap();
    assert!(w3c_json.contains("http://www.w3.org/ns/anno.jsonld"));
    assert!(w3c_json.contains("highlighting"));
}

#[test]
fn test_custom_font_injection_css() {
    let mut layout = ebook_rs::RenditionLayout::default();
    layout.set_custom_font("Roboto", "https://fonts.com/roboto.woff2");

    let css = layout.to_css_override();
    assert!(css.contains("@font-face"));
    assert!(css.contains("font-family: 'Roboto'"));
}

#[test]
fn test_non_ascii_attribute_value_extraction() {
    let html = r#"<img alt="über" title="日本語" src="image.jpg"/>"#;
    let sanitized = ebook_rs::section::sanitize_html_scripts(html);
    assert!(sanitized.contains("alt=\"über\""));
    assert!(sanitized.contains("title=\"日本語\""));
}

#[test]
fn test_rar_v4_v5_cbr_detection() {
    let rar_v4_magic = b"Rar!\x1a\x07\x00extra_bytes";
    let rar_v5_magic = b"Rar!\x1a\x07\x01\x00extra_bytes";

    assert!(ebook_rs::Book::from_bytes(rar_v4_magic).is_err());
    assert!(ebook_rs::Book::from_bytes(rar_v5_magic).is_err());
}
