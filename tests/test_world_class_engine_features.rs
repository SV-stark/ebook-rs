use ebook_rs::{Book, EbookDomTree, sanitize_and_repair_xml};

#[test]
fn test_fuzzy_malformed_xml_recovery() {
    let broken_xml = "<package><title>AT&T & R&D Guide</title></package>";
    let repaired = sanitize_and_repair_xml(broken_xml);
    assert!(repaired.contains("&amp;"));

    let doc = roxmltree::Document::parse(&repaired);
    assert!(
        doc.is_ok(),
        "Repaired XML should parse cleanly with roxmltree"
    );
}

#[test]
fn test_universal_epub3_exporter() {
    let book = Book::from_file("samples/Alice in Wonderland - Lewis Carroll EPUB3.epub")
        .expect("Sample EPUB should parse");

    let epub_bytes = book
        .export_epub3_bytes()
        .expect("EPUB to EPUB3 export should succeed");

    assert!(!epub_bytes.is_empty());
    assert_eq!(
        &epub_bytes[0..4],
        b"PK\x03\x04",
        "Exported EPUB3 bytes must be valid ZIP header"
    );

    let reloaded = Book::from_bytes(&epub_bytes)
        .expect("Exported EPUB3 bytes should be re-loadable by Book::from_bytes");
    assert_eq!(reloaded.opf.metadata.title, book.opf.metadata.title);
}

#[cfg(feature = "mmap")]
#[test]
fn test_mmap_zero_copy_reading() {
    let book = Book::from_mmap("samples/Alice in Wonderland - Lewis Carroll EPUB3.epub")
        .expect("Should mmap and parse EPUB3");
    assert!(!book.sections.is_empty());
}

#[test]
fn test_lightweight_dom_ast_tree() {
    let html = "<div><h1>Title</h1><script>alert(1)</script><p>Text</p></div>";
    let mut tree = EbookDomTree::parse(html);

    let h1_nodes = tree.find_elements_by_tag("h1");
    assert_eq!(h1_nodes.len(), 1);

    tree.strip_elements(&["script"]);
    let stripped_html = tree.to_html();
    assert!(!stripped_html.contains("script"));
    assert!(stripped_html.contains("Title"));
}
