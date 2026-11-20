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
    <dc:identifier id="uid">urn:uuid:tier34-test-001</dc:identifier>
    <dc:title>Kindle Palm PDF TeX PPTX Book</dc:title>
    <dc:creator>Alan Turing</dc:creator>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="img1" href="images/dot.png" media-type="image/png" properties="cover-image"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;

    let ch1 = br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter One</title></head>
<body>
  <h1>Chapter One</h1>
  <p>Computing machinery and intelligence with <strong>bold</strong> and <em>italic</em> prose.</p>
  <p><img src="images/dot.png" alt="Tiny Dot"/></p>
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
fn mobi_azw3_and_pdb_roundtrip() {
    let book = build_sample_book();

    let mobi = book.export_mobi_bytes().expect("MOBI export");
    assert_eq!(&mobi[60..68], b"BOOKMOBI");
    let parsed_mobi = Book::from_bytes(&mobi).expect("MOBI roundtrip parse");
    assert_eq!(
        parsed_mobi.metadata().title,
        "Kindle Palm PDF TeX PPTX Book"
    );
    assert!(
        parsed_mobi
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Computing machinery")
    );

    let azw3 = book.export_azw3_bytes().expect("AZW3 export");
    assert_eq!(&azw3[60..68], b"BOOKMOBI");
    let parsed_azw3 = Book::from_bytes(&azw3).expect("AZW3 roundtrip parse");
    assert_eq!(
        parsed_azw3.metadata().title,
        "Kindle Palm PDF TeX PPTX Book"
    );
    assert!(
        parsed_azw3
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Computing machinery")
    );

    let pdb = book.export_pdb_bytes().expect("PDB export");
    assert_eq!(&pdb[60..68], b"TEXtREAd");
    let parsed_pdb = Book::from_bytes(&pdb).expect("PDB roundtrip parse");
    assert!(
        parsed_pdb
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Computing machinery")
    );
}

#[test]
fn pdf_tex_and_pptx_roundtrip() {
    let book = build_sample_book();

    let pdf = book.export_pdf_bytes().expect("PDF export");
    assert!(pdf.starts_with(b"%PDF-1.4\n"));
    let pdf_str = String::from_utf8_lossy(&pdf);
    assert!(pdf_str.contains("/Type /Catalog"));
    assert!(pdf_str.contains("Computing machinery"));
    assert!(pdf_str.contains("%%EOF"));

    let tex = book.export_tex_bytes().expect("TeX export");
    let tex_str = String::from_utf8(tex.clone()).unwrap();
    assert!(tex_str.contains("\\documentclass"));
    assert!(tex_str.contains("\\chapter{Chapter One}"));
    assert!(tex_str.contains("\\textbf{bold}"));
    let parsed_tex = Book::from_bytes_with_title(&tex, "book.tex").expect("TeX parse");
    assert_eq!(parsed_tex.metadata().title, "Kindle Palm PDF TeX PPTX Book");
    assert!(
        parsed_tex
            .get_section(0)
            .unwrap()
            .plain_text
            .contains("Computing machinery")
    );

    let pptx = book.export_pptx_bytes().expect("PPTX export");
    let parsed_pptx = Book::from_bytes(&pptx).expect("PPTX parse");
    assert_eq!(
        parsed_pptx.metadata().title,
        "Kindle Palm PDF TeX PPTX Book"
    );
    assert!(parsed_pptx.spine().len() >= 2);
    assert!(
        parsed_pptx
            .get_section(1)
            .unwrap()
            .plain_text
            .contains("Computing machinery")
    );
}
