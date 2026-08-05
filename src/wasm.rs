use crate::book::Book;
use crate::cfi::Cfi;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// WASM exported Book class for JavaScript client applications.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmBook {
    inner: Book,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmBook {
    /// Load an EPUB book from raw byte array Uint8Array in JS.
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<WasmBook, JsValue> {
        let book = Book::from_bytes(bytes).map_err(|e| JsValue::from_str(&e))?;
        Ok(WasmBook { inner: book })
    }

    /// Get metadata as JSON string.
    pub fn get_metadata_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(self.inner.metadata()).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get Table of Contents as JSON string.
    pub fn get_toc_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(self.inner.toc()).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get spine list as JSON string.
    pub fn get_spine_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(self.inner.spine()).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get processed section HTML by spine index.
    pub fn get_section_html(&self, index: usize) -> Result<String, JsValue> {
        let sec = self
            .inner
            .get_section(index)
            .map_err(|e| JsValue::from_str(&e))?;
        Ok(sec.processed_html.clone())
    }

    /// Perform full-text search across chapters.
    pub fn search_json(&self, query: &str) -> Result<String, JsValue> {
        let results = self.inner.search(query);
        serde_json::to_string(&results).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Parse CFI string and return spine index.
    pub fn cfi_to_spine_index(&self, cfi_str: &str) -> Result<usize, JsValue> {
        let cfi = Cfi::parse(cfi_str).map_err(|e| JsValue::from_str(&e))?;
        Ok(cfi.spine_index())
    }
}
