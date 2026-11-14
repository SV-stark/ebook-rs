use ebook_rs::{Book, TxtBook};

#[test]
fn test_blackbox_universal_epub3_exporter() {
    // TxtBook uses the first H1 heading as the title when markdown=true
    let md = "# Export Test Book\n\nSection content for universal EPUB3 exporter verification.";
    let book = TxtBook::parse(md.as_bytes(), "Ignored Title", true).unwrap();

    let epub_bytes = book
        .export_epub3_bytes()
        .expect("Should export clean EPUB 3 bytes");
    assert!(!epub_bytes.is_empty());

    // Re-parse exported EPUB bytes to verify roundtrip fidelity
    let reloaded = Book::from_bytes(&epub_bytes).expect("Exported EPUB should re-parse cleanly");
    // Title is derived from the markdown H1 heading
    assert!(
        reloaded.metadata().title.contains("Export Test Book"),
        "Title was: {}",
        reloaded.metadata().title
    );
    assert_eq!(reloaded.sections().len(), 1);
    assert!(
        reloaded.sections()[0]
            .plain_text
            .contains("universal EPUB3 exporter")
    );
}

#[test]
fn test_epub3_export_includes_dcterms_modified() {
    let book = TxtBook::parse(b"EPUB metadata test.\n", "Meta Test", false).unwrap();
    for epub_bytes in [
        book.export_epub3_bytes().unwrap(),
        ebook_rs::UniversalEpub3Exporter::export(&book).unwrap(),
    ] {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&epub_bytes)).unwrap();
        let mut opf = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("OEBPS/content.opf").unwrap(), &mut opf)
            .unwrap();

        assert!(
            opf.contains("<meta property=\"dcterms:modified\">"),
            "OPF should contain dcterms:modified: {opf}"
        );
        let reloaded = Book::from_bytes(&epub_bytes).unwrap();
        let modified = reloaded
            .metadata()
            .modified_date
            .as_deref()
            .expect("reloaded metadata should have modified_date");
        assert_eq!(
            modified.len(),
            20,
            "expected CCYY-MM-DDThh:mm:ssZ, got {modified}"
        );
        assert!(modified.ends_with('Z'));
    }
}

#[test]
fn test_epub3_export_preserves_rtl_page_progression() {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default();

        zip.start_file("mimetype", stored).unwrap();
        std::io::Write::write_all(&mut zip, b"application/epub+zip").unwrap();
        zip.start_file("META-INF/container.xml", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="EPUB/package.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
        )
        .unwrap();
        zip.start_file("EPUB/package.opf", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="uid">urn:test:rtl</dc:identifier><dc:title>RTL Test</dc:title><dc:language>ar</dc:language><meta property="dcterms:modified">2026-11-06T00:00:00Z</meta></metadata><manifest><item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/></manifest><spine page-progression-direction="rtl"><itemref idref="c1"/></spine></package>"#,
        )
        .unwrap();
        zip.start_file("EPUB/c1.xhtml", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>C1</title></head><body><p>Hello</p></body></html>"#,
        )
        .unwrap();
        zip.finish().unwrap();
    }

    let book = Book::from_bytes(&buf).unwrap();
    assert_eq!(
        book.metadata().direction,
        ebook_rs::PageProgressionDirection::Rtl
    );

    for exported in [
        book.export_epub3_bytes().unwrap(),
        ebook_rs::UniversalEpub3Exporter::export(&book).unwrap(),
    ] {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&exported)).unwrap();
        let mut opf = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("OEBPS/content.opf").unwrap(), &mut opf)
            .unwrap();
        assert!(
            opf.contains("<spine page-progression-direction=\"rtl\">"),
            "exported OPF spine should preserve rtl direction: {opf}"
        );

        let reloaded = Book::from_bytes(&exported).unwrap();
        assert_eq!(
            reloaded.metadata().direction,
            ebook_rs::PageProgressionDirection::Rtl
        );
    }
}

#[test]
fn test_epub3_export_handles_root_nav_xhtml_collision() {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default();

        zip.start_file("mimetype", stored).unwrap();
        std::io::Write::write_all(&mut zip, b"application/epub+zip").unwrap();
        zip.start_file("META-INF/container.xml", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="package.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
        )
        .unwrap();
        zip.start_file("package.opf", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="uid">urn:test:root-nav</dc:identifier><dc:title>Root Nav</dc:title><dc:language>en</dc:language><meta property="dcterms:modified">2026-11-06T00:00:00Z</meta></metadata><manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/><item id="c1" href="chapter.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c1"/></spine></package>"#,
        )
        .unwrap();
        zip.start_file("nav.xhtml", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>TOC</title></head><body><nav epub:type="toc"><ol><li><a href="chapter.xhtml">Ch 1</a></li></ol></nav></body></html>"#,
        )
        .unwrap();
        zip.start_file("chapter.xhtml", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Ch 1</title></head><body><p>Hello</p></body></html>"#,
        )
        .unwrap();
        zip.finish().unwrap();
    }

    let book = Book::from_bytes(&buf).unwrap();
    let exported = ebook_rs::UniversalEpub3Exporter::export(&book)
        .expect("root-level nav.xhtml should not cause duplicate ZIP filename");
    let reloaded = Book::from_bytes(&exported).unwrap();
    assert_eq!(reloaded.sections().len(), 1);
}

#[test]
fn test_epub3_export_rewrites_relative_resource_urls_on_relocation() {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default();

        zip.start_file("mimetype", stored).unwrap();
        std::io::Write::write_all(&mut zip, b"application/epub+zip").unwrap();
        zip.start_file("META-INF/container.xml", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="EPUB/package.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
        )
        .unwrap();
        zip.start_file("EPUB/package.opf", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="uid">urn:test:reloc</dc:identifier><dc:title>Reloc Test</dc:title><dc:language>en</dc:language><meta property="dcterms:modified">2026-11-06T00:00:00Z</meta></metadata><manifest><item id="css" href="style.css" media-type="text/css"/><item id="js" href="scripts/app.js" media-type="application/javascript"/><item id="c1" href="titlepage.xhtml" media-type="application/xhtml+xml"/><item id="c2" href="chapters/ch1.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#,
        )
        .unwrap();
        zip.start_file("EPUB/style.css", deflated).unwrap();
        std::io::Write::write_all(&mut zip, b"body { color: #111; }").unwrap();
        zip.start_file("EPUB/scripts/app.js", deflated).unwrap();
        std::io::Write::write_all(&mut zip, b"console.log('ok');").unwrap();
        zip.start_file("EPUB/titlepage.xhtml", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><link rel="stylesheet" href="style.css"/><script src="scripts/app.js"></script></head><body><a href="chapters/ch1.xhtml#start">Next</a></body></html>"#,
        )
        .unwrap();
        zip.start_file("EPUB/chapters/ch1.xhtml", deflated).unwrap();
        std::io::Write::write_all(
            &mut zip,
            br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><link rel="stylesheet" href="../style.css"/></head><body><p id="start">Ch 1</p></body></html>"#,
        )
        .unwrap();
        zip.finish().unwrap();
    }

    let book = Book::from_bytes(&buf).unwrap();
    let exported = ebook_rs::UniversalEpub3Exporter::export(&book).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&exported)).unwrap();

    let mut sec0 = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("OEBPS/sec_0.xhtml").unwrap(), &mut sec0)
        .unwrap();
    assert!(
        sec0.contains(r#"href="EPUB/style.css""#),
        "sec_0 should rewrite style.css -> EPUB/style.css: {sec0}"
    );
    assert!(
        sec0.contains(r#"src="EPUB/scripts/app.js""#),
        "sec_0 should rewrite scripts/app.js -> EPUB/scripts/app.js: {sec0}"
    );
    assert!(
        sec0.contains(r#"href="sec_1.xhtml#start""#),
        "sec_0 should rewrite chapters/ch1.xhtml#start -> sec_1.xhtml#start: {sec0}"
    );

    let mut sec1 = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("OEBPS/sec_1.xhtml").unwrap(), &mut sec1)
        .unwrap();
    assert!(
        sec1.contains(r#"href="EPUB/style.css""#),
        "sec_1 should rewrite ../style.css -> EPUB/style.css: {sec1}"
    );

    // Verify all target files actually exist in the exported archive under OEBPS/
    assert!(zip.by_name("OEBPS/EPUB/style.css").is_ok());
    assert!(zip.by_name("OEBPS/EPUB/scripts/app.js").is_ok());
    assert!(zip.by_name("OEBPS/sec_1.xhtml").is_ok());
}
