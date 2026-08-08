use crate::book::Book;
use crate::rag::RagChunkConfig;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// C-compatible opaque handle to a `Book` instance.
pub type CBookHandle = *mut Book;

/// Load an eBook from raw memory bytes.
/// Returns NULL on error or an opaque pointer handle on success.
/// Must be freed with `ebook_rs_book_free`.
///
/// # Safety
/// The caller must ensure `bytes_ptr` points to a valid buffer of `bytes_len` length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_book_from_bytes(
    bytes_ptr: *const u8,
    bytes_len: usize,
) -> CBookHandle {
    if bytes_ptr.is_null() || bytes_len == 0 {
        return ptr::null_mut();
    }
    unsafe {
        let slice = std::slice::from_raw_parts(bytes_ptr, bytes_len);
        match Book::from_bytes(slice) {
            Ok(book) => Box::into_raw(Box::new(book)),
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Free a `Book` instance created by `ebook_rs_book_from_bytes`.
///
/// # Safety
/// The caller must pass a valid handle returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_book_free(handle: CBookHandle) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Get publication metadata formatted as a C JSON string.
/// Caller must free returned string using `ebook_rs_string_free`.
///
/// # Safety
/// Handle must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_get_metadata_json(handle: CBookHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let book = &*handle;
        match serde_json::to_string(book.metadata()) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Get Table of Contents formatted as a C JSON string.
/// Caller must free returned string using `ebook_rs_string_free`.
///
/// # Safety
/// Handle must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_get_toc_json(handle: CBookHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let book = &*handle;
        match serde_json::to_string(book.toc()) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Get processed HTML for a specific section index.
/// Caller must free returned string using `ebook_rs_string_free`.
///
/// # Safety
/// Handle must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_get_section_html(
    handle: CBookHandle,
    index: usize,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let book = &*handle;
        match book.get_section(index) {
            Ok(sec) => match CString::new(sec.processed_html.as_str()) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Perform full-text search and return JSON array of search results with CFI anchors.
/// Caller must free returned string using `ebook_rs_string_free`.
///
/// # Safety
/// Handle and `query_ptr` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_search_json(
    handle: CBookHandle,
    query_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() || query_ptr.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let book = &*handle;
        let query = match CStr::from_ptr(query_ptr).to_str() {
            Ok(q) => q,
            Err(_) => return ptr::null_mut(),
        };
        let results = book.search(query);
        match serde_json::to_string(&results) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Generate AI / RAG document chunks formatted as a C JSON string.
/// Caller must free returned string using `ebook_rs_string_free`.
///
/// # Safety
/// Handle must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_to_rag_chunks_json(
    handle: CBookHandle,
    max_tokens: usize,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let book = &*handle;
        let config = RagChunkConfig {
            max_tokens: if max_tokens == 0 { 512 } else { max_tokens },
            ..Default::default()
        };
        let chunks = book.to_rag_chunks(&config);
        match serde_json::to_string(&chunks) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Free C string allocated by `ebook-rs` C FFI functions.
///
/// # Safety
/// Pointer must have been allocated by this library or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_builder::generate_sample_epub;

    #[test]
    fn test_c_ffi_lifecycle() {
        unsafe {
            let bytes = generate_sample_epub().unwrap();
            let handle = ebook_rs_book_from_bytes(bytes.as_ptr(), bytes.len());
            assert!(!handle.is_null());

            let meta_json = ebook_rs_get_metadata_json(handle);
            assert!(!meta_json.is_null());
            let meta_str = CStr::from_ptr(meta_json).to_str().unwrap();
            assert!(meta_str.contains("The Rustonomicon"));
            ebook_rs_string_free(meta_json);

            let rag_json = ebook_rs_to_rag_chunks_json(handle, 256);
            assert!(!rag_json.is_null());
            let rag_str = CStr::from_ptr(rag_json).to_str().unwrap();
            assert!(rag_str.contains("chunk-sec"));
            ebook_rs_string_free(rag_json);

            ebook_rs_book_free(handle);
        }
    }
}
