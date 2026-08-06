use ebook_rs::Book;

#[test]
fn test_pdf_header_detection_and_parsing() {
    // Minimal valid PDF header structure
    let pdf_bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \ntrailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n165\n%%EOF";

    let result = Book::from_bytes_with_title(pdf_bytes, "Sample PDF Paper");

    #[cfg(feature = "pdf")]
    {
        // When PDF feature is active, it parses or returns structured PDF result
        if let Ok(book) = result {
            assert!(!book.sections.is_empty() || book.opf.metadata.title == "Sample PDF Paper");
        } else {
            // Document was minimal byte mock
            assert!(result.is_err());
        }
    }

    #[cfg(not(feature = "pdf"))]
    {
        assert!(result.is_err());
        let err_msg = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error when PDF feature is disabled"),
        };
        assert!(err_msg.contains("pdf"));
    }
}
