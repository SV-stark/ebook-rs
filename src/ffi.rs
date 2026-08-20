use crate::book::Book;
use crate::rag::RagChunkConfig;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::Arc;

use ahash::{AHashMap, AHashSet};
use parking_lot::Mutex;
use std::sync::LazyLock;

static LIVE_BOOKS: LazyLock<Mutex<AHashMap<usize, Arc<Book>>>> =
    LazyLock::new(|| Mutex::new(AHashMap::new()));
static LIVE_STRINGS: LazyLock<Mutex<AHashSet<usize>>> =
    LazyLock::new(|| Mutex::new(AHashSet::new()));

/// C-compatible opaque handle to a `Book` instance.
pub type CBookHandle = *mut Book;

/// Load an eBook from raw memory bytes.
/// Returns NULL on error or an opaque pointer handle on success.
/// Must be freed with `ebook_rs_book_free`.
///
/// # Safety
/// The caller must ensure `bytes_ptr` points to a valid buffer of `bytes_len` length.
/// Returned handles are thread-safe for concurrent read access across multiple threads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_book_from_bytes(
    bytes_ptr: *const u8,
    bytes_len: usize,
) -> CBookHandle {
    if bytes_ptr.is_null() || bytes_len == 0 {
        return ptr::null_mut();
    }
    let res = catch_unwind(AssertUnwindSafe(|| unsafe {
        let slice = std::slice::from_raw_parts(bytes_ptr, bytes_len);
        match Book::from_bytes(slice) {
            Ok(book) => {
                let arc = Arc::new(book);
                let raw = Arc::into_raw(arc.clone()) as *mut Book;
                LIVE_BOOKS.lock().insert(raw as usize, arc);
                raw
            }
            Err(_) => ptr::null_mut(),
        }
    }));
    res.unwrap_or(ptr::null_mut())
}

/// Free a `Book` instance created by `ebook_rs_book_from_bytes`.
/// Safely handles NULL and repeated calls (no-op on double-free).
///
/// # Safety
/// The caller must pass a handle returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_book_free(handle: CBookHandle) {
    if handle.is_null() {
        return;
    }
    let removed = LIVE_BOOKS.lock().remove(&(handle as usize));
    if let Some(_book_arc) = removed {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
            drop(Arc::from_raw(handle));
        }));
    }
}

/// Helper to allocate a C-string tracked in LIVE_STRINGS
fn alloc_c_string(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c_str) => {
            let raw = c_str.into_raw();
            LIVE_STRINGS.lock().insert(raw as usize);
            raw
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Helper to safely acquire an Arc<Book> handle reference without TOCTOU race
fn get_live_book(handle: CBookHandle) -> Option<Arc<Book>> {
    if handle.is_null() {
        return None;
    }
    LIVE_BOOKS.lock().get(&(handle as usize)).cloned()
}

/// Get publication metadata formatted as a C JSON string.
/// Caller must free returned string using `ebook_rs_string_free`.
///
/// # Safety
/// Handle must be a valid handle created by `ebook_rs_book_from_bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_get_metadata_json(handle: CBookHandle) -> *mut c_char {
    let book = match get_live_book(handle) {
        Some(b) => b,
        None => return ptr::null_mut(),
    };
    let res = catch_unwind(AssertUnwindSafe(|| {
        match serde_json::to_string(book.metadata()) {
            Ok(json) => alloc_c_string(json),
            Err(_) => ptr::null_mut(),
        }
    }));
    res.unwrap_or(ptr::null_mut())
}

/// Get Table of Contents formatted as a C JSON string.
/// Caller must free returned string using `ebook_rs_string_free`.
///
/// # Safety
/// Handle must be a valid handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_get_toc_json(handle: CBookHandle) -> *mut c_char {
    let book = match get_live_book(handle) {
        Some(b) => b,
        None => return ptr::null_mut(),
    };
    let res = catch_unwind(AssertUnwindSafe(|| {
        match serde_json::to_string(book.toc()) {
            Ok(json) => alloc_c_string(json),
            Err(_) => ptr::null_mut(),
        }
    }));
    res.unwrap_or(ptr::null_mut())
}

/// Get processed HTML for a specific section index.
/// Caller must free returned string using `ebook_rs_string_free`.
///
/// # Safety
/// Handle must be a valid handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_get_section_html(
    handle: CBookHandle,
    index: usize,
) -> *mut c_char {
    let book = match get_live_book(handle) {
        Some(b) => b,
        None => return ptr::null_mut(),
    };
    let res = catch_unwind(AssertUnwindSafe(|| match book.get_section(index) {
        Ok(sec) => alloc_c_string(sec.processed_html),
        Err(_) => ptr::null_mut(),
    }));
    res.unwrap_or(ptr::null_mut())
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
    if query_ptr.is_null() {
        return ptr::null_mut();
    }
    let book = match get_live_book(handle) {
        Some(b) => b,
        None => return ptr::null_mut(),
    };
    let res = catch_unwind(AssertUnwindSafe(|| unsafe {
        let query = match CStr::from_ptr(query_ptr).to_str() {
            Ok(q) => q,
            Err(_) => return ptr::null_mut(),
        };
        let results = book.search(query);
        match serde_json::to_string(&results) {
            Ok(json) => alloc_c_string(json),
            Err(_) => ptr::null_mut(),
        }
    }));
    res.unwrap_or(ptr::null_mut())
}

/// Generate AI / RAG document chunks formatted as a C JSON string.
/// Caller must free returned string using `ebook_rs_string_free`.
///
/// # Safety
/// Handle must be a valid handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_to_rag_chunks_json(
    handle: CBookHandle,
    max_tokens: usize,
) -> *mut c_char {
    let book = match get_live_book(handle) {
        Some(b) => b,
        None => return ptr::null_mut(),
    };
    let res = catch_unwind(AssertUnwindSafe(|| {
        let config = RagChunkConfig {
            max_tokens: if max_tokens == 0 { 512 } else { max_tokens },
            ..Default::default()
        };
        let chunks = book.to_rag_chunks(&config);
        match serde_json::to_string(&chunks) {
            Ok(json) => alloc_c_string(json),
            Err(_) => ptr::null_mut(),
        }
    }));
    res.unwrap_or(ptr::null_mut())
}

/// Free C string allocated by `ebook-rs` C FFI functions.
/// Safely handles NULL and repeated calls (no-op on double-free).
///
/// # Safety
/// Pointer must have been allocated by this library or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ebook_rs_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    let is_live = LIVE_STRINGS.lock().remove(&(ptr as usize));
    if !is_live {
        return; // Guard against double-free UB
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(CString::from_raw(ptr));
    }));
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
