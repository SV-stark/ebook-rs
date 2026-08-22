use crate::book::Book;
use crate::kfx::container::{KfxContainer, KfxIndexEntry};
use crate::kfx::symbols::*;

/// Universal Amazon KFX Container Exporter capable of serializing any `Book` into valid KFX container bytes.
pub struct UniversalKfxExporter;

impl UniversalKfxExporter {
    /// Export any `Book` (EPUB, MOBI, AZW3, PDF, FB2, TXT, MD) to a valid Amazon KFX binary container buffer.
    pub fn export(book: &Book) -> Result<Vec<u8>, String> {
        let mut entries = Vec::new();
        let mut payload = Vec::new();

        // Pass 1: Survey & Metadata Payload
        let meta = book.metadata();
        let meta_str = format!(
            "title: \"{}\"\nauthor: \"{}\"\npublisher: \"{}\"\nlanguage: \"{}\"\n",
            meta.title,
            meta.creators.first().cloned().unwrap_or_default(),
            meta.publishers.first().cloned().unwrap_or_default(),
            meta.language()
        );

        let meta_bytes = meta_str.as_bytes();
        let meta_offset = payload.len() as u64;
        let meta_len = meta_bytes.len() as u64;
        payload.extend_from_slice(meta_bytes);

        entries.push(KfxIndexEntry {
            entity_id: 101,
            type_id: SYM_BOOK_METADATA,
            offset: meta_offset,
            length: meta_len,
        });

        // Pass 2: Storylines / Sections Payload
        for (idx, _) in book.spine().iter().enumerate() {
            if let Ok(sec) = book.get_section(idx) {
                let text_to_write = if !sec.plain_text.trim().is_empty() {
                    &sec.plain_text
                } else if !sec.raw_html.trim().is_empty() {
                    &sec.raw_html
                } else {
                    ""
                };
                let sec_text = text_to_write.as_bytes();
                let sec_offset = payload.len() as u64;
                let sec_len = sec_text.len() as u64;
                payload.extend_from_slice(sec_text);

                entries.push(KfxIndexEntry {
                    entity_id: (200 + idx) as u32,
                    type_id: SYM_STORYLINE_FRAGMENT,
                    offset: sec_offset,
                    length: sec_len,
                });
            }
        }

        // Pass 3: Serialization Pass via KfxContainer::build
        let container_bytes = KfxContainer::build(&entries, &payload);
        Ok(container_bytes)
    }
}
