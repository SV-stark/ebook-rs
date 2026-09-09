use ebook_rs::Book;
use rayon::prelude::*;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Collect all regular files recursively from a directory.
fn collect_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_files(&path));
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Mutate bytes with controlled corruptions to test parser resilience.
fn generate_corrupted_variants(original: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
    let mut variants = Vec::new();

    if original.is_empty() {
        return variants;
    }

    // 1. Truncated at 10%, 25%, 50%, 90%
    for pct in [10, 25, 50, 90] {
        let cut = (original.len() * pct) / 100;
        let name = match pct {
            10 => "Truncated (10%)",
            25 => "Truncated (25%)",
            50 => "Truncated (50%)",
            _ => "Truncated (90%)",
        };
        variants.push((name, original[..cut.max(1)].to_vec()));
    }

    // 2. Corrupted header (zero out first 16 bytes)
    let mut zeroed_header = original.to_vec();
    let header_len = 16.min(zeroed_header.len());
    for b in zeroed_header[..header_len].iter_mut() {
        *b = 0x00;
    }
    variants.push(("Zeroed Header", zeroed_header));

    // 3. Bit-flip mutation (flip bits in every 64th byte)
    let mut bit_flipped = original.to_vec();
    for (i, b) in bit_flipped.iter_mut().enumerate() {
        if i % 64 == 0 {
            *b ^= 0xFF;
        }
    }
    variants.push(("Bit-Flipped Noise", bit_flipped));

    // 4. Injected garbage suffix
    let mut with_garbage = original.to_vec();
    with_garbage.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF, 0x42, 0x13]);
    variants.push(("Garbage Appended", with_garbage));

    variants
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let target_dir = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("samples"));

    println!("# 🛡️ ebook-rs Mass Corpus & Resilience Stress Test\n");
    println!("**Target Directory**: `{}`", target_dir.display());

    let files = collect_files(&target_dir);
    if files.is_empty() {
        eprintln!("⚠️ No files found in directory: {}", target_dir.display());
        std::process::exit(1);
    }

    println!("**Total Sample Files**: {}\n", files.len());

    // Phase 1: Real-World Ingestion
    println!("### Phase 1: Real-World File Ingestion");
    println!("| File | Size (KB) | Status | Sections | Time (ms) |");
    println!("| :--- | :--- | :--- | :--- | :--- |");

    let parsed_ok = AtomicUsize::new(0);
    let handled_err = AtomicUsize::new(0);
    let panics = AtomicUsize::new(0);

    let start_phase1 = Instant::now();

    let results: Vec<(String, usize, Result<Result<Book, _>, _>, u128)> = files
        .iter()
        .map(|path| {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let bytes = fs::read(path).unwrap_or_default();
            let size_kb = bytes.len() / 1024;

            let file_start = Instant::now();
            let res = catch_unwind(AssertUnwindSafe(|| Book::from_bytes(&bytes)));
            let elapsed_ms = file_start.elapsed().as_millis();

            (file_name, size_kb, res, elapsed_ms)
        })
        .collect();

    for (file_name, size_kb, res, elapsed_ms) in &results {
        match res {
            Ok(Ok(book)) => {
                parsed_ok.fetch_add(1, Ordering::Relaxed);
                println!(
                    "| `{}` | {} | ✅ OK | {} sections | {}ms |",
                    file_name,
                    size_kb,
                    book.sections.len(),
                    elapsed_ms
                );
            }
            Ok(Err(e)) => {
                handled_err.fetch_add(1, Ordering::Relaxed);
                let err_str = e.to_string();
                let short_err = err_str.chars().take(40).collect::<String>();
                println!(
                    "| `{}` | {} | ⚠️ Handled Err (`{}`) | - | {}ms |",
                    file_name, size_kb, short_err, elapsed_ms
                );
            }
            Err(_) => {
                panics.fetch_add(1, Ordering::SeqCst);
                println!(
                    "| `{}` | {} | 💥 **PANIC** | - | {}ms |",
                    file_name, size_kb, elapsed_ms
                );
            }
        }
    }

    let phase1_dur = start_phase1.elapsed();

    // Phase 2: Chaos & Mutation Resilience Fuzzing
    println!("\n### Phase 2: Chaos & Corruption Mutation Testing");
    println!("Mutating valid input files with byte truncation, header zeroing, and bit flips...\n");

    let chaos_tests = AtomicUsize::new(0);
    let chaos_panics = AtomicUsize::new(0);
    let chaos_handled = AtomicUsize::new(0);
    let chaos_recovered = AtomicUsize::new(0);

    let start_phase2 = Instant::now();

    // Run parallel mutations across all valid files
    files.par_iter().for_each(|path| {
        if let Ok(bytes) = fs::read(path) {
            // Only generate mutations for non-trivial files
            if bytes.len() > 100 && bytes.len() < 10_000_000 {
                let mutations = generate_corrupted_variants(&bytes);
                for (_mutation_name, mutated_bytes) in mutations {
                    chaos_tests.fetch_add(1, Ordering::Relaxed);

                    let res = catch_unwind(AssertUnwindSafe(|| Book::from_bytes(&mutated_bytes)));

                    match res {
                        Ok(Ok(_)) => {
                            chaos_recovered.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(Err(_)) => {
                            chaos_handled.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(_) => {
                            eprintln!("💥 PANIC detected on mutated variant of {:?}", path);
                            chaos_panics.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            }
        }
    });

    let phase2_dur = start_phase2.elapsed();

    let total_panics = panics.load(Ordering::SeqCst) + chaos_panics.load(Ordering::SeqCst);
    let total_chaos = chaos_tests.load(Ordering::Relaxed);

    println!("| Metric | Phase 1 (Real Files) | Phase 2 (Mutated / Corrupted) |");
    println!("| :--- | :--- | :--- |");
    println!(
        "| **Evaluations** | {} files | {} mutated streams |",
        files.len(),
        total_chaos
    );
    println!(
        "| **Successful Parses** | {} | {} (Fuzzy Recovered) |",
        parsed_ok.load(Ordering::Relaxed),
        chaos_recovered.load(Ordering::Relaxed)
    );
    println!(
        "| **Clean Error Returns** | {} | {} |",
        handled_err.load(Ordering::Relaxed),
        chaos_handled.load(Ordering::Relaxed)
    );
    println!(
        "| **Panics / Crashes** | **{}** | **{}** |",
        panics.load(Ordering::SeqCst),
        chaos_panics.load(Ordering::SeqCst)
    );
    println!(
        "| **Execution Time** | {:.2?} | {:.2?} |",
        phase1_dur, phase2_dur
    );

    println!("\n### Summary Verdict");
    if total_panics == 0 {
        println!(
            "> **PASSED**: 0 unhandled panics across all real-world files and corrupted byte mutations."
        );
        println!("> `ebook-rs` demonstrated 100% memory-safe and graceful error handling.\n");
    } else {
        println!(
            "> ❌ **FAILED**: Encountered {} unhandled panics!",
            total_panics
        );
        std::process::exit(1);
    }
}
