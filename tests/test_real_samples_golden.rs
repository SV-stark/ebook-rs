use ebook_rs::{
    Book, CbzBook, EbookError, RagChunkConfig, UniversalEpub3Exporter, UniversalKfxExporter,
};
use std::path::Path;

fn find_sample_path(rel_path: &str) -> Option<std::path::PathBuf> {
    let p = Path::new(rel_path);
    if p.exists() {
        return Some(p.to_path_buf());
    }
    let p_parent = Path::new("..").join(rel_path);
    if p_parent.exists() {
        return Some(p_parent);
    }
    None
}

#[test]
fn test_golden_real_epub2() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll EPUB2.epub") {
        Some(p) => p,
        None => return, // Skip if sample file not present in environment
    };

    let book = Book::from_file(&path).expect("Failed to parse real EPUB2 sample");
    assert!(
        !book.metadata().title.is_empty(),
        "Title should not be empty"
    );
    assert!(!book.sections.is_empty(), "Sections should not be empty");

    // Test search on real content
    let search_res = book.search("Rabbit");
    assert!(
        !search_res.is_empty(),
        "Should find 'Rabbit' in Alice in Wonderland EPUB2"
    );

    // Test RAG chunking
    let chunks = book.to_rag_chunks(&RagChunkConfig::default());
    assert!(
        !chunks.is_empty(),
        "Should produce RAG chunks from real EPUB2"
    );

    // Test roundtrip export to EPUB3
    let epub3_bytes =
        UniversalEpub3Exporter::export(&book).expect("Failed to export real EPUB2 to EPUB3");
    assert!(
        epub3_bytes.len() > 1000,
        "Exported EPUB3 should have valid size"
    );
}

#[test]
fn test_golden_real_epub3() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll EPUB3.epub") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real EPUB3 sample");
    assert!(!book.metadata().title.is_empty());
    assert!(!book.sections.is_empty());

    // Test Table of Contents extraction
    assert!(
        !book.toc().is_empty(),
        "Real EPUB3 should have navigation TOC"
    );

    // Test location generation
    assert!(
        !book.locations.entries.is_empty(),
        "Locations should be generated"
    );
}

#[test]
fn test_golden_real_kepub() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.kepub") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real KEPUB sample");
    assert!(!book.metadata().title.is_empty());
    assert!(!book.sections.is_empty());
}

#[test]
fn test_golden_real_azw3() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.azw3") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real AZW3 sample");
    assert!(!book.metadata().title.is_empty());
    assert!(!book.sections.is_empty());
}

#[test]
fn test_golden_real_mobi() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.mobi") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real MOBI sample");
    assert!(!book.sections.is_empty());
}

#[test]
fn test_golden_real_fb2() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.fb2") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real FB2 sample");
    assert!(!book.metadata().title.is_empty());
    let total_text_len: usize = book.sections.iter().map(|s| s.plain_text.len()).sum();
    assert!(
        total_text_len > 1000,
        "FB2 document should contain text content"
    );
}

#[test]
fn test_golden_real_kfx() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.kfx") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real KFX sample");
    assert!(
        !book.sections.is_empty(),
        "KFX sections should be populated"
    );
    assert!(
        !book.metadata().title.is_empty(),
        "KFX title should not be empty"
    );

    let total_chars: usize = book.sections.iter().map(|s| s.char_count).sum();
    assert!(
        total_chars > 20_000,
        "Real KFX book should extract substantial text content, got {}",
        total_chars
    );

    // 1. Search operations on real KFX
    let search_rabbit = book.search("Rabbit");
    assert!(
        !search_rabbit.is_empty(),
        "Should find 'Rabbit' in real KFX book"
    );

    let search_alice = book.search("Alice");
    assert!(
        !search_alice.is_empty(),
        "Should find 'Alice' in real KFX book"
    );

    // 2. Table of Contents & Locations
    assert!(!book.toc().is_empty(), "TOC entries should be generated");
    assert!(
        book.locations().total_locations > 0,
        "Locations should be generated"
    );

    // 3. AI RAG Chunking
    let rag_chunks = book.to_rag_chunks(&RagChunkConfig::default());
    assert!(
        !rag_chunks.is_empty(),
        "RAG chunks should be generated from KFX"
    );

    // 4. Universal KFX export roundtrip & re-parsing
    let kfx_bytes = UniversalKfxExporter::export(&book).expect("Failed to export KFX");
    assert!(kfx_bytes.len() > 100, "Exported KFX buffer should be valid");
    assert_eq!(
        &kfx_bytes[0..4],
        b"CONT",
        "Exported KFX should have CONT magic"
    );

    let roundtrip_book = Book::from_bytes(&kfx_bytes).expect("Roundtrip KFX should parse cleanly");
    assert_eq!(roundtrip_book.metadata().title, book.metadata().title);
    assert!(!roundtrip_book.sections.is_empty());
}

#[test]
fn test_golden_real_lit() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.lit") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real LIT sample");
    assert!(!book.sections.is_empty());
}

#[test]
fn test_golden_real_txt() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.txt") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real TXT sample");
    assert!(!book.sections.is_empty());
    assert!(book.sections[0].plain_text.contains("Alice"));
}

#[test]
fn test_golden_real_cbz() {
    let path = match find_sample_path("samples/Jumbo Comics 099.cbz") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real CBZ sample");
    assert!(!book.sections.is_empty());

    let pages = CbzBook::prefetch_page_images(&book, 0, 5);
    assert!(
        !pages.is_empty(),
        "CBZ should contain extracted comic pages"
    );
}

#[test]
fn test_golden_real_cbr_error_handling() {
    let path = match find_sample_path("samples/Jumbo Comics 082.cbr") {
        Some(p) => p,
        None => return,
    };

    let result = Book::from_file(&path);
    assert!(
        result.is_err(),
        "CBR (RAR format) without unrar should return structured error"
    );
    if let Err(err) = result {
        let msg = match err {
            EbookError::InvalidFormat(m) => m,
            EbookError::Custom(m) => m,
            other => other.to_string(),
        };
        assert!(msg.to_lowercase().contains("cbr") || msg.to_lowercase().contains("rar"));
    }
}

#[test]
fn test_golden_real_pdf() {
    let _path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.pdf") {
        Some(p) => p,
        None => return,
    };

    #[cfg(feature = "pdf")]
    {
        let book = Book::from_file(&_path).expect("Failed to parse real PDF sample");
        assert!(!book.sections.is_empty());
    }
}

#[test]
fn test_golden_real_docx() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.docx") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real DOCX sample");
    assert!(
        !book.metadata().title.is_empty(),
        "DOCX title should not be empty"
    );
    assert!(
        !book.sections.is_empty(),
        "DOCX sections should not be empty"
    );

    let total_len: usize = book.sections.iter().map(|s| s.plain_text.len()).sum();
    assert!(total_len > 1000, "DOCX plain text should be extracted");

    // Test search
    let search_res = book.search("Rabbit");
    assert!(
        !search_res.is_empty(),
        "Should find 'Rabbit' in DOCX sample"
    );
}

#[test]
fn test_golden_real_rtf() {
    let path = match find_sample_path("samples/Alice in Wonderland - Lewis Carroll.rtf") {
        Some(p) => p,
        None => return,
    };

    let book = Book::from_file(&path).expect("Failed to parse real RTF sample");
    assert!(
        !book.sections.is_empty(),
        "RTF sections should not be empty"
    );

    let total_len: usize = book.sections.iter().map(|s| s.plain_text.len()).sum();
    assert!(total_len > 1000, "RTF plain text should be extracted");

    // Test search
    let search_res = book.search("Rabbit");
    assert!(!search_res.is_empty(), "Should find 'Rabbit' in RTF sample");
}
