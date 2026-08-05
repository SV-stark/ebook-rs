use ebook_rs::Book as EbookRs;
use std::fs;
use std::time::Instant;

fn find_sample_epub() -> String {
    if let Ok(entries) = fs::read_dir("samples") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "epub") {
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
