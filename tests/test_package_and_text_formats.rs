use ebook_rs::Book;
use std::io::{Cursor, Write};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

// 1x1 valid PNG
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

fn build_sample_book() -> Book {
    let container_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

    let opf_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="uid">urn:uuid:tier1-test-001</dc:identifier>
    <dc:title>Tier One Sample Book</dc:title>
    <dc:creator>Ada Lovelace</dc:creator>
    <dc:language>en</dc:language>
    <dc:description>A comprehensive test book for Tier 1 pure-Rust formats.</dc:description>
  </metadata>
  <manifest>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="css" href="style.css" media-type="text/css"/>
    <item id="img1" href="images/dot.png" media-type="image/png" properties="cover-image"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;

    let css = b"body { font-family: Georgia, serif; }";
    let ch1 = br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter One</title><link rel="stylesheet" href="style.css"/></head>
<body>
  <h1>Chapter One</h1>
  <p>First sentence here. Second sentence with <strong>bold</strong> and <em>italic</em> text!</p>
  <p><img src="images/dot.png" alt="Tiny Dot"/></p>
  <blockquote><p>Quoted wisdom.</p></blockquote>
  <table><tr><th>Col A</th><th>Col B</th></tr><tr><td>Val 1</td><td>Val 2</td></tr></table>
</body>
</html>"#;

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();
        let deflated =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (p, d) in [
            ("META-INF/container.xml", container_xml.as_slice()),
            ("OEBPS/content.opf", opf_xml.as_slice()),
            ("OEBPS/style.css", css.as_slice()),
            ("OEBPS/ch1.xhtml", ch1.as_slice()),
            ("OEBPS/images/dot.png", TINY_PNG),
        ] {
            zip.start_file(p, deflated).unwrap();
            zip.write_all(d).unwrap();
        }
        zip.finish().unwrap();
    }
    Book::from_bytes(&buf).expect("Sample EPUB should parse")
}

#[test]
fn tier1_kepub_roundtrip_instruments_kobo_spans() {
    let book = build_sample_book();
    let kepub = book.export_kepub_bytes().expect("KEPUB export");
    let parsed = Book::from_bytes(&kepub).expect("KEPUB parse");
    assert_eq!(parsed.metadata().title, "Tier One Sample Book");
    let sec = parsed.get_section(0).unwrap();
    assert!(sec.raw_html.contains("id=\"book-columns\""));
    assert!(sec.raw_html.contains("id=\"book-inner\""));
    assert!(sec.raw_html.contains("class=\"koboSpan\""));
    assert!(sec.raw_html.contains("id=\"kobo."));
    assert!(sec.plain_text.contains("First sentence here."));
    assert!(sec.plain_text.contains("Second sentence"));
}

#[test]
fn tier1_txt_and_md_roundtrip() {
    let book = build_sample_book();

    let txt = book.export_txt_bytes().expect("TXT export");
    let txt_str = String::from_utf8(txt.clone()).unwrap();
    assert!(txt_str.contains("Tier One Sample Book"));
    assert!(txt_str.contains("First sentence here."));
    let parsed_txt = Book::from_bytes_with_title(&txt, "sample.txt").unwrap();
    assert!(
        parsed_txt
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Quoted wisdom.")
    );

    let md = book.export_markdown_bytes().expect("MD export");
    let md_str = String::from_utf8(md.clone()).unwrap();
    assert!(md_str.starts_with("---\n"));
    assert!(md_str.contains("title: \"Tier One Sample Book\""));
    assert!(md_str.contains("# Chapter One"));
    assert!(md_str.contains("**bold**"));
    assert!(md_str.contains("*italic*"));
    assert!(md_str.contains("> Quoted wisdom."));
    assert!(md_str.contains("| Col A | Col B |"));
    let parsed_md = Book::from_bytes_with_title(&md, "sample.md").unwrap();
    assert_eq!(parsed_md.metadata().title, "Tier One Sample Book");
}

#[test]
fn tier1_fb2_roundtrip_with_images_and_metadata() {
    let book = build_sample_book();
    let fb2 = book.export_fb2_bytes().expect("FB2 export");
    let fb2_str = String::from_utf8(fb2.clone()).unwrap();
    assert!(fb2_str.contains("<FictionBook"));
    assert!(fb2_str.contains("<book-title>Tier One Sample Book</book-title>"));
    assert!(fb2_str.contains("<binary "));
    assert!(fb2_str.contains("<strong>bold</strong>"));
    assert!(fb2_str.contains("<emphasis>italic</emphasis>"));

    let parsed = Book::from_bytes(&fb2).expect("FB2 round-trip parse");
    assert_eq!(parsed.metadata().title, "Tier One Sample Book");
    assert!(
        parsed
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Quoted wisdom.")
    );
}

#[test]
fn tier1_html_standalone_roundtrip() {
    let book = build_sample_book();
    let html = book.export_html_bytes().expect("HTML export");
    let html_str = String::from_utf8(html.clone()).unwrap();
    assert!(html_str.starts_with("<!DOCTYPE html>"));
    assert!(html_str.contains("data:image/png;base64,"));
    assert!(html_str.contains("<title>Tier One Sample Book</title>"));

    let parsed = Book::from_bytes(&html).expect("HTML parse");
    assert_eq!(parsed.metadata().title, "Tier One Sample Book");
    assert_eq!(parsed.metadata().creators, vec!["Ada Lovelace".to_string()]);
    assert!(
        parsed
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("First sentence here.")
    );
}

#[test]
fn tier1_htmlz_txtz_zip_roundtrip() {
    let book = build_sample_book();

    let htmlz = book.export_htmlz_bytes().expect("HTMLZ export");
    let parsed_htmlz = Book::from_bytes(&htmlz).expect("HTMLZ parse");
    assert_eq!(parsed_htmlz.metadata().title, "Tier One Sample Book");
    assert!(parsed_htmlz.cover_image().is_some());
    assert!(
        parsed_htmlz
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Quoted wisdom.")
    );

    let txtz = book.export_txtz_bytes().expect("TXTZ export");
    let parsed_txtz = Book::from_bytes(&txtz).expect("TXTZ parse");
    assert_eq!(parsed_txtz.metadata().title, "Tier One Sample Book");
    assert!(parsed_txtz.cover_image().is_some());
    assert!(
        parsed_txtz
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("First sentence here.")
    );

    let zip_bytes = book.export_zip_bytes().expect("ZIP export");
    let parsed_zip = Book::from_bytes(&zip_bytes).expect("ZIP parse");
    assert_eq!(parsed_zip.metadata().title, "Tier One Sample Book");
}

#[test]
fn tier1_oeb_dir_and_zip_roundtrip() {
    let book = build_sample_book();

    // OEB ZIP roundtrip
    let oeb_zip = book.export_oeb_bytes().expect("OEB bytes export");
    let parsed_oeb_zip = Book::from_bytes(&oeb_zip).expect("OEB ZIP parse");
    assert_eq!(parsed_oeb_zip.metadata().title, "Tier One Sample Book");

    // OEB directory + .opf file roundtrip
    let tmp_dir = std::env::temp_dir().join("ebook_rs_test_oeb_dir");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    book.export_oeb_dir(&tmp_dir).expect("OEB dir export");
    assert!(tmp_dir.join("content.opf").exists());

    let from_dir = Book::from_file(&tmp_dir).expect("OEB dir parse");
    assert_eq!(from_dir.metadata().title, "Tier One Sample Book");

    let from_opf = Book::from_file(tmp_dir.join("content.opf")).expect("OEB .opf parse");
    assert_eq!(from_opf.metadata().title, "Tier One Sample Book");

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[test]
fn tier1_tcr_roundtrip_and_malformed_guard() {
    let book = build_sample_book();
    let tcr = book.export_tcr_bytes().expect("TCR export");
    assert!(tcr.starts_with(b"!!8-Bit!!"));

    let parsed = Book::from_bytes_with_title(&tcr, "sample.tcr").expect("TCR parse");
    let text = &parsed.get_section(0).unwrap().plain_text;
    assert!(text.contains("Tier One Sample Book"));
    assert!(text.contains("First sentence here."));
    assert!(text.contains("Quoted wisdom."));

    // Truncated TCR must return clean Err without panicking
    assert!(Book::from_bytes(b"!!8-Bit!!\x05abc").is_err());
}
