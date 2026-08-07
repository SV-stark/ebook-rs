use ebook_rs::Book as EbookRs;
use std::fs;
use std::time::Instant;

fn find_sample_epub() -> String {
    if let Ok(entries) = fs::read_dir("samples") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "epub") {
                return path.to_string_lossy().to_string();
            }
        }
    }
    panic!("No sample EPUB file found in samples/ directory");
}

#[test]
fn test_benchmark_performance() {
    let sample_path = find_sample_epub();
    println!("\n========================================================");
    println!("⚡ REAL EPUB PERFORMANCE BENCHMARK");
    println!("📖 Target EPUB: {}", sample_path);
    println!("========================================================\n");

    let iterations = 10;

    let start_ebook = Instant::now();
    for _ in 0..iterations {
        let _book = EbookRs::from_file(&sample_path).expect("ebook-rs should parse book");
    }
    let duration_ebook = start_ebook.elapsed();
    let avg_ebook_ms = (duration_ebook.as_secs_f64() * 1000.0) / (iterations as f64);

    println!(
        "📊 1. EPUB Open & Full Package Parsing Speed ({} iterations avg):",
        iterations
    );
    println!("   🚀 ebook-rs: {:.3} ms / parse", avg_ebook_ms);

    let book = EbookRs::from_file(&sample_path).unwrap();

    println!("\n📊 2. Feature Extracted Metrics:");
    println!("   - Title:              '{}'", book.metadata().title);
    println!("   - ebook-rs Spine:      {} sections", book.spine().len());

    let search_start = Instant::now();
    let matches = book.search("Rabbit");
    let search_dur = search_start.elapsed();

    println!(
        "   - ebook-rs Full-Text Search: {} matches for 'Rabbit' in {:.3} ms",
        matches.len(),
        search_dur.as_secs_f64() * 1000.0
    );

    println!("\n========================================================\n");
}

#[test]
fn test_all_sample_formats_benchmark() {
    use ebook_rs::EbookDomTree;

    println!("\n========================================================");
    println!("⚡ REAL-LIFE MULTI-FORMAT BENCHMARK (v0.10.5 vs v0.9.0)");
    println!("========================================================\n");

    let sample_files = [
        (
            "EPUB 3",
            "samples/Alice in Wonderland - Lewis Carroll EPUB3.epub",
        ),
        (
            "EPUB 2",
            "samples/Alice in Wonderland - Lewis Carroll EPUB2.epub",
        ),
        ("MOBI", "samples/Alice in Wonderland - Lewis Carroll.mobi"),
        ("AZW3", "samples/Alice in Wonderland - Lewis Carroll.azw3"),
        ("FB2", "samples/Alice in Wonderland - Lewis Carroll.fb2"),
        ("KEPUB", "samples/Alice in Wonderland - Lewis Carroll.kepub"),
        ("LIT", "samples/Alice in Wonderland - Lewis Carroll.lit"),
        ("TXT", "samples/Alice in Wonderland - Lewis Carroll.txt"),
        ("CBZ Comic", "samples/Jumbo Comics 099.cbz"),
    ];

    let mut total_parse_time_ms = 0.0;
    let mut total_search_time_ms = 0.0;
    let mut total_export_time_ms = 0.0;
    let mut total_dom_ast_time_ms = 0.0;

    for (fmt, path) in &sample_files {
        if !std::path::Path::new(path).exists() {
            continue;
        }

        // 1. Measure Book Open / Parsing
        let parse_start = Instant::now();
        let book = EbookRs::from_file(path).expect("Should parse book");
        let parse_ms = parse_start.elapsed().as_secs_f64() * 1000.0;
        total_parse_time_ms += parse_ms;

        // 2. Measure Full-Text Search
        let search_start = Instant::now();
        let matches = book.search("Alice");
        let search_ms = search_start.elapsed().as_secs_f64() * 1000.0;
        total_search_time_ms += search_ms;

        // 3. Measure Universal EPUB 3 Export
        let export_start = Instant::now();
        let _epub_bytes = book.export_epub3_bytes().unwrap();
        let export_ms = export_start.elapsed().as_secs_f64() * 1000.0;
        total_export_time_ms += export_ms;

        // 4. Measure DOM AST Tree Parsing
        let dom_start = Instant::now();
        if let Ok(sec) = book.get_section(0) {
            let _tree = EbookDomTree::parse(&sec.raw_html);
        }
        let dom_ms = dom_start.elapsed().as_secs_f64() * 1000.0;
        total_dom_ast_time_ms += dom_ms;

        println!(
            "📖 {:<12} | Parse: {:6.2} ms | Search ('Alice' -> {:3} matches): {:5.2} ms | DOM AST: {:4.2} ms | EPUB3 Export: {:6.2} ms",
            fmt,
            parse_ms,
            matches.len(),
            search_ms,
            dom_ms,
            export_ms
        );
    }

    println!("\n--------------------------------------------------------");
    println!("📊 SUMMARY RESULTS (v0.10.5 vs v0.9.0 Baseline)");
    println!("--------------------------------------------------------");
    println!(
        "   Total Real-Life Sample Parse Time:  {:.2} ms",
        total_parse_time_ms
    );
    println!(
        "   Total Full-Text Search Time:        {:.2} ms",
        total_search_time_ms
    );
    println!(
        "   Total DOM AST Parsing Time:         {:.2} ms",
        total_dom_ast_time_ms
    );
    println!(
        "   Total EPUB3 Conversion Time:        {:.2} ms",
        total_export_time_ms
    );
    println!("========================================================\n");
}
