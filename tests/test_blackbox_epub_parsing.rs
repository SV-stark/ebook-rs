use ebook_rs::Book;
use std::io::Write;
use zip::write::SimpleFileOptions;

fn build_synthetic_epub_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = SimpleFileOptions::default();

        zip.start_file("mimetype", options).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n  <rootfiles>\n    <rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/>\n  </rootfiles>\n</container>",
        )
        .unwrap();

        let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="pub-id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Synthetic Test Book</dc:title>
    <dc:creator>Author Name</dc:creator>
    <dc:language>en</dc:language>
    <dc:identifier id="pub-id">urn:uuid:12345678-1234-1234-1234-123456789abc</dc:identifier>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;
        zip.start_file("OEBPS/content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();

        let nav = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<body>
  <nav epub:type="toc">
    <h1>Table of Contents</h1>
    <ol>
      <li><a href="ch1.xhtml">Chapter 1: The Beginning</a></li>
    </ol>
  </nav>
</body>
</html>"#;
        zip.start_file("OEBPS/nav.xhtml", options).unwrap();
        zip.write_all(nav.as_bytes()).unwrap();

        let ch1 = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body>
  <h1>Chapter 1: The Beginning</h1>
  <p>This is the first chapter of the synthetic test book.</p>
</body>
</html>"#;
        zip.start_file("OEBPS/ch1.xhtml", options).unwrap();
        zip.write_all(ch1.as_bytes()).unwrap();

        zip.finish().unwrap();
    }
    buf
}

#[test]
fn test_blackbox_epub_parsing_and_metadata() {
    let epub_bytes = build_synthetic_epub_bytes();
    let book = Book::from_bytes(&epub_bytes).expect("Synthetic EPUB should parse cleanly");

    assert_eq!(book.metadata().title, "Synthetic Test Book");
    assert_eq!(book.metadata().creators, vec!["Author Name"]);
    assert_eq!(book.metadata().languages, vec!["en"]);
    assert_eq!(book.sections.len(), 1);
    assert_eq!(book.spine().len(), 1);

    // Verify section content retrieval
    let sec = book.get_section(0).expect("Section 0 should exist");
    assert!(sec.raw_html.contains("Chapter 1: The Beginning"));
    assert!(sec.plain_text.contains("This is the first chapter"));

    // Verify Table of Contents parsing
    assert_eq!(book.toc().len(), 1);
    assert_eq!(book.toc()[0].label, "Chapter 1: The Beginning");
}
