use ebook_rs::{generate_sample_epub, Book, ReaderServer};
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("serve");

    match mode {
        "parse" => {
            let path = args.get(2).ok_or("Usage: ebook-rs parse <path.epub>")?;
            let book = Book::from_file(path)?;
            println!("📖 Metadata:\n{}", serde_json::to_string_pretty(book.metadata())?);
            println!("\n📌 Table of Contents:\n{}", serde_json::to_string_pretty(book.toc())?);
            println!("\n📚 Spine Items ({} total):\n{}", book.spine().len(), serde_json::to_string_pretty(book.spine())?);
        }
        "search" => {
            let path = args.get(2).ok_or("Usage: ebook-rs search <path.epub> <query>")?;
            let query = args.get(3).ok_or("Usage: ebook-rs search <path.epub> <query>")?;
            let book = Book::from_file(path)?;
            let results = book.search(query);
            println!("🔍 Search results for '{}' ({} found):", query, results.len());
            for r in results {
                println!("[Spine {}] CFI: {}\n    Snippet: {}\n", r.spine_index, r.cfi, r.snippet);
            }
        }
        "locations" => {
            let path = args.get(2).ok_or("Usage: ebook-rs locations <path.epub>")?;
            let book = Book::from_file(path)?;
            println!("📍 Generated Locations ({} total):", book.locations.total_locations);
            println!("{}", serde_json::to_string_pretty(&book.locations.entries)?);
        }
        "sample" => {
            let out_path = args.get(2).map(|s| s.as_str()).unwrap_or("sample.epub");
            let bytes = generate_sample_epub()?;
            fs::write(out_path, bytes)?;
            println!("✅ Generated sample EPUB file at: {}", out_path);
        }
        "serve" | _ => {
            let file_arg = if args.len() > 1 && !args[1].starts_with('-') && mode != "serve" {
                Some(&args[1])
            } else if args.len() > 2 {
                Some(&args[2])
            } else {
                None
            };

            let port: u16 = env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().unwrap_or(8080);

            let book = if let Some(p) = file_arg {
                if Path::new(p).exists() {
                    println!("📖 Loading EPUB from file: {}", p);
                    Book::from_file(p)?
                } else {
                    println!("⚠️ Specified file '{}' not found. Generating sample EPUB in memory...", p);
                    let bytes = generate_sample_epub()?;
                    Book::from_bytes(&bytes)?
                }
            } else {
                println!("💡 No EPUB file path provided. Generating built-in sample EPUB 3...");
                let bytes = generate_sample_epub()?;
                Book::from_bytes(&bytes)?
            };

            println!("✨ Title: {}", book.metadata().title);
            println!("✨ Author(s): {}", book.metadata().creators.join(", "));
            println!("✨ Spine Sections: {}", book.spine().len());
            println!("✨ Locations Generated: {}", book.locations.total_locations);

            let server = ReaderServer::new(book, port);
            server.listen()?;
        }
    }

    Ok(())
}
