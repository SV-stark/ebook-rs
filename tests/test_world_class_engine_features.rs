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

#[test]
fn test_performance_crate_accelerators() {
    // 1. Test simdutf8 fast SIMD validation
    let valid_bytes = b"Hello SIMD UTF-8 World";
    assert!(simdutf8::basic::from_utf8(valid_bytes).is_ok());

    // 2. Test ahash AHashMap speed
    let mut map = ahash::AHashMap::new();
    map.insert("ch1.xhtml", 1);
    assert_eq!(map.get("ch1.xhtml"), Some(&1));

    // 3. Test compact_str SSO
    let sso_str = compact_str::CompactString::new("short_idref");
    assert_eq!(sso_str, "short_idref");

    // 4. Test parking_lot mutex lock
    let lock = parking_lot::Mutex::new(42);
    assert_eq!(*lock.lock(), 42);
}

#[test]
fn test_utf8_non_ascii_xml_repair() {
    let broken_xml = "<package><title>AT&T & über & 日本語</title></package>";
    let repaired = sanitize_and_repair_xml(broken_xml);
    println!("Repaired XML: {}", repaired);

    assert!(repaired.contains("&amp;"));
    assert!(repaired.contains("über"));
    assert!(repaired.contains("日本語"));
}

#[test]
fn test_nested_dom_ast_tree_hierarchy() {
    let html = "<div><h1>Title</h1><p>Paragraph with <span>nested span</span></p></div>";
    let tree = EbookDomTree::parse(html);
    println!("Tree nodes: {:#?}", tree);

    assert_eq!(tree.root_nodes.len(), 1, "Root nodes count should be 1 for top div");
    if let ebook_rs::DomNode::Element { children, .. } = &tree.root_nodes[0] {
        assert_eq!(children.len(), 2, "Children of div should be h1 and p");
    } else {
        panic!("Root node must be an element");
    }
}
