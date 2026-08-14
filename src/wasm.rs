#[cfg(feature = "wasm")]
use crate::book::Book;
#[cfg(feature = "wasm")]
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
        let book = Book::from_bytes(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
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

    /// Get tokenized SpeechSynthesis TTS word tokens with character offsets as JSON string.
    pub fn get_tts_words_json(&self, index: usize) -> Result<String, JsValue> {
        let tokens = self
            .inner
            .get_tts_tokens(index)
            .map_err(|e| JsValue::from_str(&e))?;
        serde_json::to_string(&tokens).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get section HTML with `<span id="tts-w-{index}">` tags for SpeechSynthesis word-by-word visual highlighting.
    pub fn get_tts_annotated_html(&self, index: usize) -> Result<String, JsValue> {
        self.inner
            .get_tts_section_html(index)
            .map_err(|e| JsValue::from_str(&e))
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

    /// Retrieve raw resource byte array Uint8Array for Blob URL creation in JS.
    pub fn get_resource_bytes(&self, path: &str) -> Result<Vec<u8>, JsValue> {
        let (bytes, _) = self
            .inner
            .get_resource_bytes(path)
            .map_err(|e| JsValue::from_str(&e))?;
        Ok(bytes)
    }

    /// Get NLP reading analytics for a section as JSON string (word count, reading time WPM, difficulty score, keywords).
    pub fn get_section_analytics_json(&self, index: usize) -> Result<String, JsValue> {
        let sec = self
            .inner
            .get_section(index)
            .map_err(|e| JsValue::from_str(&e))?;
        serde_json::to_string(&sec.analytics()).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get virtual reflow page map for a section as JSON string.
    pub fn paginate_section_json(&self, index: usize) -> Result<String, JsValue> {
        let sec = self
            .inner
            .get_section(index)
            .map_err(|e| JsValue::from_str(&e))?;
        serde_json::to_string(&sec.paginate(None)).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Resolve CFI string to DOM element selector and offset target JSON string (F1 WASM Fix).
    pub fn resolve_cfi_dom_json(
        &self,
        cfi_str: &str,
        section_index: usize,
    ) -> Result<String, JsValue> {
        let cfi = Cfi::parse(cfi_str).map_err(|e| JsValue::from_str(&e))?;
        let sec = self
            .inner
            .get_section(section_index)
            .map_err(|e| JsValue::from_str(&e))?;
        let target = cfi.resolve_dom_path(&sec.raw_html);
        serde_json::to_string(&target).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Set custom reader font family and font URL (F5 WASM Fix).
    pub fn set_custom_font(&mut self, font_family: &str, font_url_or_b64: &str) {
        self.inner
            .layout
            .set_custom_font(font_family, font_url_or_b64);
    }

    /// Generate AI / RAG document chunks formatted as JSON string.
    pub fn to_rag_chunks_json(&self, max_tokens: usize) -> Result<String, JsValue> {
        let config = crate::rag::RagChunkConfig {
            max_tokens: if max_tokens == 0 { 512 } else { max_tokens },
            ..Default::default()
        };
        let chunks = self.inner.to_rag_chunks(&config);
        serde_json::to_string(&chunks).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Export publication as Readium WebPub Manifest JSON string.
    pub fn get_webpub_manifest_json(&self) -> Result<String, JsValue> {
        let manifest = self.inner.to_webpub_manifest();
        serde_json::to_string(&manifest).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Re-export publication as Universal EPUB3 raw Uint8Array byte vector.
    pub fn export_epub_bytes(&self) -> Result<Vec<u8>, JsValue> {
        crate::UniversalEpub3Exporter::export(&self.inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Enable 2-Page Manga Spread mode (Right-to-Left reading progression).
    pub fn enable_manga_mode(&mut self) {
        crate::cbz::CbzBook::enable_manga_mode(&mut self.inner);
    }

    /// Get standalone zero-dependency `<ebook-reader>` HTMLElement Web Component JS definition string.
    pub fn get_custom_element_js() -> String {
        r#"
class EbookReaderElement extends HTMLElement {
    connectedCallback() {
        this.innerHTML = `<iframe style="width:100%;height:100%;border:none;" src="${this.getAttribute('src')}"></iframe>`;
    }
}
if (!customElements.get('ebook-reader')) {
    customElements.define('ebook-reader', EbookReaderElement);
}
"#.to_string()
    }
}
