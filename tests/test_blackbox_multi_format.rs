use ebook_rs::{Book, Fb2Book, MobiBook, PdfBook, TxtBook};
use std::io::Write;
use zip::write::SimpleFileOptions;

#[test]
fn test_blackbox_pdf_parsing() {
    let pdf_bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
    match PdfBook::parse(pdf_bytes, "Test PDF") {
        Ok(pdf_book) => {
            assert_eq!(pdf_book.metadata().title, "Test PDF");
            assert_eq!(pdf_book.sections.len(), 1);
        }
        Err(err) => {
            assert!(err.to_lowercase().contains("pdf"));
        }
    }
}

#[test]
fn test_blackbox_mobi_pdb_parsing() {
    let mut pdb_bytes = vec![0u8; 78];
    let name = b"MOBI Book Title";
    pdb_bytes[..name.len()].copy_from_slice(name);
    pdb_bytes[60..68].copy_from_slice(b"BOOKMOBI");

    // 2 records
    pdb_bytes[76] = 0;
    pdb_bytes[77] = 2;

    let rec0_start = 94;
    let rec0_len = 56;
    let rec1_start = rec0_start + rec0_len;

    pdb_bytes.extend_from_slice(&(rec0_start as u32).to_be_bytes());
    pdb_bytes.extend_from_slice(&0u32.to_be_bytes());
    pdb_bytes.extend_from_slice(&(rec1_start as u32).to_be_bytes());
    pdb_bytes.extend_from_slice(&0u32.to_be_bytes());

    let mut rec0 = vec![0u8; rec0_len];
    rec0[1] = 1; // compression = 1 (uncompressed)
    rec0[9] = 1; // record count = 1
    rec0[16..20].copy_from_slice(b"MOBI");
    rec0[20..24].copy_from_slice(&24u32.to_be_bytes());
    pdb_bytes.extend_from_slice(&rec0);

    let html_content = b"<html><body><h1>Chapter 1</h1><p>MOBI text content</p></body></html>";
    pdb_bytes.extend_from_slice(html_content);

    let mobi = MobiBook::parse(&pdb_bytes).expect("MobiBook parse");
    assert_eq!(mobi.metadata().title, "MOBI Book Title");
    assert!(mobi.sections[0].plain_text.contains("MOBI text content"));
}

#[test]
fn test_blackbox_fb2_parsing() {
    let fb2_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info>
      <book-title>FB2 Test Book</book-title>
      <author><first-name>John</first-name><last-name>Doe</last-name></author>
    </title-info>
  </description>
  <body>
    <section>
      <title><p>FB2 Chapter 1</p></title>
      <p>FB2 paragraph content.</p>
    </section>
  </body>
</FictionBook>"#;

    let fb2 = Fb2Book::parse(fb2_xml.as_bytes()).expect("FB2 parse");
    assert_eq!(fb2.metadata().title, "FB2 Test Book");
    assert!(fb2.sections[0].plain_text.contains("FB2 paragraph content"));

    let auto_fb2 = Book::from_bytes(fb2_xml.as_bytes()).expect("Auto FB2");
    assert_eq!(auto_fb2.metadata().title, "FB2 Test Book");
}

#[test]
fn test_blackbox_txt_and_markdown_parsing() {
    let txt = "Line 1\nLine 2\nLine 3";
    let txt_book = TxtBook::parse(txt.as_bytes(), "Plain Text", false).expect("TXT parse");
    assert_eq!(txt_book.metadata().title, "Plain Text");
    assert!(txt_book.sections[0].plain_text.contains("Line 1"));

    let md = "# Title Heading\n\n## Section 1\nParagraph in markdown.\n\n## Section 2\nSecond paragraph.";
    let md_book = TxtBook::parse(md.as_bytes(), "MD Book", true).expect("MD parse");
    assert_eq!(md_book.metadata().title, "Title Heading");
    assert_eq!(md_book.sections.len(), 3);
    assert_eq!(md_book.toc().len(), 3);
}

#[test]
fn test_blackbox_cbz_synthetic_parsing() {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        let dummy_img = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0xFF, 0xD9,
        ];
        zip.start_file("page01.jpg", options).unwrap();
        zip.write_all(&dummy_img).unwrap();
        zip.start_file("page02.png", options).unwrap();
        zip.write_all(&dummy_img).unwrap();
        zip.finish().unwrap();
    }

    let cbz = Book::from_bytes_with_title(&buf, "CBZ Comic").expect("CBZ parse");
    assert_eq!(cbz.metadata().title, "CBZ Comic");
    assert_eq!(cbz.sections.len(), 2);
    assert!(cbz.sections[0].plain_text.contains("page01.jpg"));
}
