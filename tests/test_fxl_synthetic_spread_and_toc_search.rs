use ebook_rs::{NavPoint, TxtBook};

#[test]
fn test_toc_deep_search_and_flattening() {
    let subitem2 = NavPoint {
        id: "sec2.1".to_string(),
        label: "Quantum Entanglement Deep Dive".to_string(),
        href: "ch2.html#sec2.1".to_string(),
        full_path: "ch2.html".to_string(),
        subitems: Vec::new(),
    };

    let item2 = NavPoint {
        id: "ch2".to_string(),
        label: "Chapter 2: Quantum Physics".to_string(),
        href: "ch2.html".to_string(),
        full_path: "ch2.html".to_string(),
        subitems: vec![subitem2],
    };

    let item1 = NavPoint {
        id: "ch1".to_string(),
        label: "Chapter 1: Classical Mechanics".to_string(),
        href: "ch1.html".to_string(),
        full_path: "ch1.html".to_string(),
        subitems: Vec::new(),
    };

    let toc = vec![item1, item2];

    // 1. Test TOC Deep Search
    let search_results = NavPoint::search(&toc, "quantum");
    assert_eq!(
        search_results.len(),
        2,
        "Should find 2 matching TOC nodes across hierarchy"
    );
    assert_eq!(search_results[0].breadcrumb, "Chapter 2: Quantum Physics");
    assert_eq!(
        search_results[1].breadcrumb,
        "Chapter 2: Quantum Physics > Quantum Entanglement Deep Dive"
    );

    // 2. Test TOC Flattening
    let flat_toc = NavPoint::flatten(&toc);
    assert_eq!(flat_toc.len(), 3);
    assert_eq!(flat_toc[2].depth, 1);
}

#[test]
fn test_synthetic_fxl_2_page_spread_generation() {
    let md = "# Page 1\nLeft page content.\n\n# Page 2\nRight page content.";
    let book =
        TxtBook::parse(md.as_bytes(), "FXL Sample", true).expect("Markdown book should parse");

    let spread = book
        .get_synthetic_spread(0, Some(1))
        .expect("Synthetic spread generation should succeed");

    assert_eq!(spread.left_index, 0);
    assert_eq!(spread.right_index, Some(1));
    assert!(spread.combined_html.contains("epub-fxl-spread-container"));
    assert!(spread.combined_html.contains("page-left"));
    assert!(spread.combined_html.contains("page-right"));
    assert!(spread.combined_html.contains("Left page content"));
    assert!(spread.combined_html.contains("Right page content"));
}
