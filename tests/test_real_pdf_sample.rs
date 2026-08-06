use ebook_rs::Book;
use std::path::Path;

#[test]
fn test_real_pdf_sample_parsing() {
    let pdf_path = "samples/Alice in Wonderland - Lewis Carroll.pdf";

    if !Path::new(pdf_path).exists() {
        println!("⚠️ Skipping test_real_pdf_sample_parsing: sample PDF file not found at {}", pdf_path);
        return;
    }

    println!("🧪 Testing Real PDF Sample File: {}", pdf_path);

    let result = Book::from_file(pdf_path);

    #[cfg(feature = "pdf")]
    {
        let book = result.expect("Real PDF sample should be successfully parsed by PdfBook");

        println!("   - PDF Title: {}", book.metadata().title);
        println!("   - Page Count (Spine): {}", book.spine().len());
        println!("   - Total Locations: {}", book.locations.total_locations);

        assert!(!book.metadata().title.is_empty(), "PDF title should not be empty");
        assert!(book.spine().len() > 0, "PDF should have at least 1 page/spine item");
        assert!(book.sections.len() > 0, "PDF should have extracted section pages");

        // Inspect Page 0
        let page0 = book.get_section(0).expect("Page 0 should exist");
        println!("   - Page 0 plain text char count: {}", page0.char_count);
        assert!(
            page0.char_count > 0 || !page0.raw_html.is_empty(),
            "Page 0 should contain extracted text/HTML"
        );

        // Perform full-text search across PDF pages
        let matches = book.search("Alice");
        println!("   - Search 'Alice' in PDF: {} matches found", matches.len());
        assert!(!matches.is_empty(), "Search for 'Alice' in Alice in Wonderland PDF should yield matches");
    }

    #[cfg(not(feature = "pdf"))]
    {
        assert!(result.is_err(), "Opening PDF when 'pdf' feature is disabled should return an error");
    }
}
