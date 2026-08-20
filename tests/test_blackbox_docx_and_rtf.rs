use ebook_rs::{Book, DocxBook, RagChunkConfig, RagChunker, RtfBook};
use std::io::Write;
use zip::ZipWriter;
use zip::write::FileOptions;

fn create_synthetic_docx() -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buf);
        let options: FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 1. [Content_Types].xml
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="xml" ContentType="application/xml"/>
    <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#).unwrap();

        // 2. docProps/core.xml
        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
    xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Word Document</dc:title>
    <dc:creator>Test Author</dc:creator>
    <dc:description>Document description</dc:description>
</cp:coreProperties>"#).unwrap();

        // 3. word/document.xml
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
        <w:p>
            <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
            <w:r><w:t>Chapter 1: The Beginning</w:t></w:r>
        </w:p>
        <w:p>
            <w:r><w:rPr><w:b/></w:rPr><w:t>Bold intro text. </w:t></w:r>
            <w:r><w:rPr><w:i/></w:rPr><w:t>Italic secondary text. </w:t></w:r>
            <w:r><w:rPr><w:u/></w:rPr><w:t>Underlined text.</w:t></w:r>
        </w:p>
        <w:tbl>
            <w:tr>
                <w:tc><w:p><w:r><w:t>Cell 1</w:t></w:r></w:p></w:tc>
                <w:tc><w:p><w:r><w:t>Cell 2</w:t></w:r></w:p></w:tc>
            </w:tr>
        </w:tbl>
        <w:p>
            <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
            <w:r><w:t>Chapter 2: The Journey</w:t></w:r>
        </w:p>
        <w:p>
            <w:r><w:t>Continuation of the story across chapters.</w:t></w:r>
        </w:p>
    </w:body>
</w:document>"#,
        )
        .unwrap();

        zip.finish().unwrap();
    }
    buf.into_inner()
}

#[test]
fn test_docx_parsing_and_auto_detection() {
    let docx_bytes = create_synthetic_docx();

    // 1. Direct DocxBook parse
    let book = DocxBook::parse(&docx_bytes, "Fallback").expect("Should parse synthetic DOCX");
    assert_eq!(book.metadata().title, "Test Word Document");
    assert_eq!(book.metadata().creators, vec!["Test Author"]);
    assert_eq!(book.sections.len(), 2);
    assert_eq!(book.toc.len(), 2);
    assert_eq!(book.toc[0].label, "Chapter 1: The Beginning");
    assert_eq!(book.toc[1].label, "Chapter 2: The Journey");

    let sec0 = book.get_section(0).unwrap();
    assert!(sec0.raw_html.contains("<strong>Bold intro text. </strong>"));
    assert!(sec0.raw_html.contains("<em>Italic secondary text. </em>"));
    assert!(sec0.raw_html.contains("<u>Underlined text.</u>"));
    assert!(sec0.raw_html.contains("<table>"));
    assert!(sec0.raw_html.contains("<td>Cell 1</td>"));

    // 2. Book::from_bytes auto-detection
    let detected_book = Book::from_bytes(&docx_bytes).expect("Should auto-detect DOCX");
    assert_eq!(detected_book.metadata().title, "Test Word Document");
    assert_eq!(detected_book.sections.len(), 2);

    // 3. Universal EPUB3 export
    let epub_bytes = book
        .export_epub3_bytes()
        .expect("Should export EPUB3 from DOCX");
    assert!(epub_bytes.starts_with(b"PK\x03\x04"));
    let exported_epub = Book::from_bytes(&epub_bytes).expect("Should parse exported EPUB3");
    assert_eq!(exported_epub.metadata().title, "Test Word Document");
    assert_eq!(exported_epub.sections.len(), 2);
}

#[test]
fn test_rtf_parsing_and_auto_detection() {
    let rtf_str = r#"{\rtf1\ansi\deff0
{\info{\title My RTF Masterpiece}{\author Rust Ace}}
\b Bold RTF Header\b0\par
\i Italicized paragraph content.\i0\par
\ul Underlined sentence.\ulnone\par
\page
Section 2 after explicit page break with Unicode \u12371?\u12435?\u12395?\u12385?\u12399? greeting.\par
}"#;

    let rtf_bytes = rtf_str.as_bytes();

    // 1. Direct RtfBook parse
    let book = RtfBook::parse(rtf_bytes, "Default Title").expect("Should parse RTF");
    assert_eq!(book.metadata().title, "My RTF Masterpiece");
    assert_eq!(book.metadata().creators, vec!["Rust Ace"]);
    assert_eq!(book.sections.len(), 2);

    let sec0 = book.get_section(0).unwrap();
    assert!(sec0.raw_html.contains("<strong>Bold RTF Header</strong>"));
    assert!(
        sec0.raw_html
            .contains("<em>Italicized paragraph content.</em>")
    );
    assert!(sec0.raw_html.contains("<u>Underlined sentence.</u>"));

    let sec1 = book.get_section(1).unwrap();
    assert!(sec1.plain_text.contains("こんにちは"));

    // 2. Book::from_bytes auto-detection
    let detected_book = Book::from_bytes(rtf_bytes).expect("Should auto-detect RTF");
    assert_eq!(detected_book.metadata().title, "My RTF Masterpiece");
    assert_eq!(detected_book.sections.len(), 2);

    // 3. AI RAG chunking & BM25 ranking
    let chunks = detected_book.to_rag_chunks(&RagChunkConfig::default());
    assert!(!chunks.is_empty());
    let ranked = RagChunker::rank_chunks_bm25(&chunks, "Italicized", 1);
    assert_eq!(ranked.len(), 1);

    // 4. Universal EPUB3 export
    let epub_bytes = detected_book
        .export_epub3_bytes()
        .expect("Should export EPUB3 from RTF");
    let reloaded = Book::from_bytes(&epub_bytes).expect("Should parse exported EPUB3");
    assert_eq!(reloaded.metadata().title, "My RTF Masterpiece");
}

fn create_synthetic_odt_with_images() -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buf);
        let options: FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 1. mimetype (stored uncompressed)
        let stored_options: FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("mimetype", stored_options).unwrap();
        zip.write_all(b"application/vnd.oasis.opendocument.text")
            .unwrap();

        // 2. meta.xml
        zip.start_file("meta.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
    <office:meta>
        <dc:title>Test ODT With Images</dc:title>
        <dc:creator>Stark ODT Author</dc:creator>
    </office:meta>
</office:document-meta>"#).unwrap();

        // 3. content.xml
        zip.start_file("content.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
    xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
    xmlns:xlink="http://www.w3.org/1999/xlink">
    <office:body>
        <office:text>
            <text:h text:outline-level="1">Chapter 1: Visuals</text:h>
            <text:p>Here is an illustration:</text:p>
            <text:p>
                <draw:frame draw:name="Graphic1">
                    <draw:image xlink:href="Pictures/100000000000032000000258B63D5600.png"/>
                </draw:frame>
            </text:p>
            <text:p>End of chapter.</text:p>
        </office:text>
    </office:body>
</office:document-content>"#,
        )
        .unwrap();

        // 4. Pictures/100000000000032000000258B63D5600.png
        zip.start_file("Pictures/100000000000032000000258B63D5600.png", options)
            .unwrap();
        zip.write_all(&[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        ])
        .unwrap();
    }
    buf.into_inner()
}

#[test]
fn test_odt_parsing_with_embedded_images() {
    use ebook_rs::OdtBook;

    let odt_bytes = create_synthetic_odt_with_images();
    let book = OdtBook::parse(&odt_bytes, "Fallback").expect("Failed to parse synthetic ODT");

    assert_eq!(book.metadata().title, "Test ODT With Images");
    assert_eq!(book.metadata().creators, vec!["Stark ODT Author"]);
    assert_eq!(book.sections.len(), 1);

    let sec = book.get_section(0).expect("Failed to get section 0");
    assert!(sec.raw_html.contains("<h2>Chapter 1: Visuals</h2>"));
    assert!(
        sec.raw_html
            .contains("<img src=\"Pictures/100000000000032000000258B63D5600.png\"")
    );

    // Verify image data exists in archive
    let img_res = book.get_resource_bytes("Pictures/100000000000032000000258B63D5600.png");
    assert!(img_res.is_ok());
    let (data, mime) = img_res.unwrap();
    assert_eq!(data.len(), 12);
    assert_eq!(mime, "image/png");
}
