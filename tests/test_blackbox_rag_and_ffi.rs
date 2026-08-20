use ebook_rs::{Book, RagChunkConfig, RenditionLayout, WritingMode, generate_sample_epub};
use std::ffi::CStr;

#[test]
fn test_blackbox_rag_chunking_engine() {
    let bytes = generate_sample_epub().expect("Failed to build sample epub");
    let book = Book::from_bytes(&bytes).expect("Failed to parse sample epub");

    let config = RagChunkConfig {
        max_tokens: 128,
        overlap_tokens: 16,
        preserve_headings: true,
        include_cfi: true,
        min_chunk_size: 20,
    };

    let chunks = book.to_rag_chunks(&config);
    assert!(!chunks.is_empty(), "RAG chunks should not be empty");

    let first = &chunks[0];
    assert!(first.id.starts_with("chunk-sec-"));
    assert!(!first.text.is_empty());
    assert!(!first.markdown.is_empty());
    assert!(first.cfi.contains("epubcfi"));
    assert_eq!(first.book_title, "The Rustonomicon & EBook-RS Guide");
    assert_eq!(first.book_author, "SV-Stark");
}

#[test]
fn test_blackbox_c_ffi_bindings() {
    use ebook_rs::ffi::*;

    unsafe {
        let bytes = generate_sample_epub().expect("Failed to build sample epub");
        let handle = ebook_rs_book_from_bytes(bytes.as_ptr(), bytes.len());
        assert!(!handle.is_null());

        let meta_json_ptr = ebook_rs_get_metadata_json(handle);
        assert!(!meta_json_ptr.is_null());
        let meta_str = CStr::from_ptr(meta_json_ptr).to_str().unwrap();
        assert!(meta_str.contains("The Rustonomicon"));
        ebook_rs_string_free(meta_json_ptr);

        let rag_json_ptr = ebook_rs_to_rag_chunks_json(handle, 128);
        assert!(!rag_json_ptr.is_null());
        let rag_str = CStr::from_ptr(rag_json_ptr).to_str().unwrap();
        assert!(rag_str.contains("chunk-sec-"));
        ebook_rs_string_free(rag_json_ptr);

        let sec_html_ptr = ebook_rs_get_section_html(handle, 0);
        assert!(!sec_html_ptr.is_null());
        let sec_str = CStr::from_ptr(sec_html_ptr).to_str().unwrap();
        assert!(!sec_str.is_empty());
        ebook_rs_string_free(sec_html_ptr);

        // Double-free guard verification
        ebook_rs_book_free(handle);
        ebook_rs_book_free(handle); // safe no-op

        // Accessing freed handle returns NULL without crashing
        let freed_meta = ebook_rs_get_metadata_json(handle);
        assert!(freed_meta.is_null());

        // Null string free is safe
        ebook_rs_string_free(std::ptr::null_mut());
    }
}

#[test]
fn test_blackbox_c_ffi_concurrent_access() {
    use ebook_rs::ffi::*;

    unsafe {
        let bytes = generate_sample_epub().expect("Failed to build sample epub");
        let handle = ebook_rs_book_from_bytes(bytes.as_ptr(), bytes.len());
        assert!(!handle.is_null());

        let handle_val = handle as usize;
        let mut handles = Vec::new();

        for _ in 0..8 {
            let h = std::thread::spawn(move || {
                let h_ptr = h_val_as_ptr(handle_val);
                let meta = ebook_rs_get_metadata_json(h_ptr);
                if !meta.is_null() {
                    let s = CStr::from_ptr(meta).to_str().unwrap();
                    assert!(s.contains("The Rustonomicon"));
                    ebook_rs_string_free(meta);
                }
            });
            handles.push(h);
        }

        for h in handles {
            h.join().unwrap();
        }

        ebook_rs_book_free(handle);
    }
}

fn h_val_as_ptr(val: usize) -> ebook_rs::ffi::CBookHandle {
    val as ebook_rs::ffi::CBookHandle
}

#[test]
fn test_blackbox_writing_mode_and_direction() {
    let mut layout = RenditionLayout::default();
    assert_eq!(layout.writing_mode, WritingMode::HorizontalLtr);

    layout.writing_mode = WritingMode::VerticalRl;
    let css = layout.to_css_override();
    assert!(css.contains("writing-mode: vertical-rl;"));

    layout.writing_mode = WritingMode::HorizontalRtl;
    let css_rtl = layout.to_css_override();
    assert!(css_rtl.contains("direction: rtl;"));
}
