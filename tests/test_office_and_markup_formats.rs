use ebook_rs::Book;
use std::io::{Cursor, Write};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

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
    <dc:identifier id="uid">urn:uuid:tier2-test-001</dc:identifier>
    <dc:title>Tier Two Sample Book</dc:title>
    <dc:creator>Grace Hopper</dc:creator>
    <dc:language>en</dc:language>
    <dc:description>Test book for Tier 2 pure-Rust formats.</dc:description>
  </metadata>
  <manifest>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="img1" href="images/dot.png" media-type="image/png"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;

    let ch1 = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter One</title></head>
<body>
  <h1>Chapter One</h1>
  <p>Hello <strong>bold</strong> and <em>italic</em> world with Unicode café.</p>
  <p><img src="images/dot.png" alt="Tiny Dot"/></p>
  <table><tr><th>Key</th><th>Value</th></tr><tr><td>Alpha</td><td>Beta</td></tr></table>
</body>
</html>"#
        .as_bytes();

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
            ("OEBPS/ch1.xhtml", ch1),
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
fn tier2_docx_odt_rtf_roundtrip() {
    let book = build_sample_book();

    let docx = book.export_docx_bytes().expect("DOCX export");
    let parsed_docx = Book::from_bytes(&docx).expect("DOCX parse");
    assert_eq!(parsed_docx.metadata().title, "Tier Two Sample Book");
    assert!(
        parsed_docx
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Alpha")
    );

    let odt = book.export_odt_bytes().expect("ODT export");
    let parsed_odt = Book::from_bytes(&odt).expect("ODT parse");
    assert_eq!(parsed_odt.metadata().title, "Tier Two Sample Book");
    assert!(
        parsed_odt
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Chapter One")
    );

    let rtf = book.export_rtf_bytes().expect("RTF export");
    let rtf_str = String::from_utf8(rtf.clone()).unwrap();
    assert!(rtf_str.starts_with("{\\rtf1"));
    assert!(rtf_str.contains("\\pngblip"));
    let parsed_rtf = Book::from_bytes(&rtf).expect("RTF parse");
    assert!(
        parsed_rtf
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Chapter One")
    );
}

#[test]
fn tier2_pmlz_and_rb_roundtrip() {
    let book = build_sample_book();

    let pmlz = book.export_pmlz_bytes().expect("PMLZ export");
    let parsed_pmlz = Book::from_bytes_with_title(&pmlz, "sample.pmlz").expect("PMLZ parse");
    let pml_text = &parsed_pmlz.get_section(0).unwrap().plain_text;
    assert!(pml_text.contains("Chapter One"));
    assert!(pml_text.contains("bold"));

    let rb = book.export_rb_bytes().expect("RB export");
    assert!(rb.starts_with(b"\xb0\x0c\xb0\x0c\x02\x00NUVO"));
    let parsed_rb = Book::from_bytes(&rb).expect("RB parse");
    assert_eq!(parsed_rb.metadata().title, "Tier Two Sample Book");
    assert_eq!(
        parsed_rb.metadata().creators,
        vec!["Grace Hopper".to_string()]
    );
    assert!(
        parsed_rb
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Chapter One")
    );

    // Truncated RB must return clean Err without panicking
    assert!(Book::from_bytes(b"\xb0\x0c\xb0\x0c\x02\x00NUVO\x00\x00\x00\x00short").is_err());
}

#[test]
fn tier2_typst_adoc_org_rst_roundtrip() {
    let book = build_sample_book();

    let typ = book.export_typst_bytes().expect("Typst export");
    let typ_str = String::from_utf8(typ.clone()).unwrap();
    assert!(typ_str.contains("#set document("));
    assert!(typ_str.contains("= Chapter One"));
    let parsed_typ = Book::from_bytes_with_title(&typ, "book.typ").expect("Typst parse");
    assert_eq!(parsed_typ.metadata().title, "Tier Two Sample Book");
    assert!(
        parsed_typ
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Chapter One")
    );

    let adoc = book.export_adoc_bytes().expect("AsciiDoc export");
    let adoc_str = String::from_utf8(adoc.clone()).unwrap();
    assert!(adoc_str.starts_with("= Tier Two Sample Book"));
    assert!(adoc_str.contains(":author: Grace Hopper"));
    let parsed_adoc = Book::from_bytes_with_title(&adoc, "book.adoc").expect("AsciiDoc parse");
    assert_eq!(parsed_adoc.metadata().title, "Tier Two Sample Book");
    assert!(
        parsed_adoc
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Chapter One")
    );

    let org = book.export_org_bytes().expect("Org export");
    let org_str = String::from_utf8(org.clone()).unwrap();
    assert!(org_str.contains("#+TITLE: Tier Two Sample Book"));
    assert!(org_str.contains("#+AUTHOR: Grace Hopper"));
    let parsed_org = Book::from_bytes_with_title(&org, "book.org").expect("Org parse");
    assert_eq!(parsed_org.metadata().title, "Tier Two Sample Book");
    assert!(
        parsed_org
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Chapter One")
    );

    let rst = book.export_rst_bytes().expect("RST export");
    let rst_str = String::from_utf8(rst.clone()).unwrap();
    assert!(rst_str.contains(":Author: Grace Hopper"));
    let parsed_rst = Book::from_bytes_with_title(&rst, "book.rst").expect("RST parse");
    assert_eq!(parsed_rst.metadata().title, "Tier Two Sample Book");
    assert!(
        parsed_rst
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Chapter One")
    );
}
