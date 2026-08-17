use crate::Cfi;
use crate::annotations::AnnotationManager;
use crate::archive::EpubArchive;
use crate::deobfuscate::FontDeobfuscator;
use crate::error::EbookError;
use crate::layout::RenditionLayout;
use crate::locations::Locations;
use crate::metadata::{ManifestItem, Metadata, SpineItem};
use crate::nav::{Landmark, NavPoint, PageListItem, parse_landmarks, parse_nav_xhtml, parse_ncx};
use crate::opf::parse_opf;
use crate::search::{SearchEngine, SearchResult};
use crate::section::Section;
use ahash::AHashMap;
use std::path::Path;
use std::sync::Arc;

pub type BeforeDisplayHook = Arc<dyn Fn(&mut String, &str) + Send + Sync>;

/// Main Book Core API engine.
pub struct Book {
    pub archive: EpubArchive,
    pub opf: crate::opf::OpfPackage,
    pub toc: Vec<NavPoint>,
    pub landmarks: Vec<Landmark>,
    pub page_list: Vec<PageListItem>,
    pub sections: Vec<Section>,
    pub locations: Locations,
    pub annotations: AnnotationManager,
    pub layout: RenditionLayout,
    pub font_deobfuscator: FontDeobfuscator,
    pub before_display_hooks: Vec<BeforeDisplayHook>,
    pub media_overlays: AHashMap<String, crate::media_overlay::MediaOverlayPackage>,
    pub render_cache: parking_lot::Mutex<AHashMap<usize, String>>,
}

impl Clone for Book {
    fn clone(&self) -> Self {
        Self {
            archive: self.archive.clone(),
            opf: self.opf.clone(),
            toc: self.toc.clone(),
            landmarks: self.landmarks.clone(),
            page_list: self.page_list.clone(),
            sections: self.sections.clone(),
            locations: self.locations.clone(),
            annotations: self.annotations.clone(),
            layout: self.layout.clone(),
            font_deobfuscator: self.font_deobfuscator.clone(),
            before_display_hooks: self.before_display_hooks.clone(),
            media_overlays: self.media_overlays.clone(),
            render_cache: parking_lot::Mutex::new(self.render_cache.lock().clone()),
        }
    }
}

impl Book {
    /// Load an EPUB, KEPUB, MOBI, AZW3, FB2, LIT, CBZ, or CBR book from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, EbookError> {
        let p = path.as_ref();
        let bytes = std::fs::read(p).map_err(|e| {
            EbookError::Io(format!("Failed to read ebook file {}: {}", p.display(), e))
        })?;
        let filename = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Comic Book");
        Self::from_bytes_with_title(&bytes, filename)
    }

    /// Open an EPUB, KEPUB, MOBI, AZW3, FB2, LIT, CBZ, or CBR ebook from an in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EbookError> {
        Self::from_bytes_with_title(bytes, "eBook")
    }

    /// Open an eBook from an in-memory byte slice with a title fallback.
    pub fn from_bytes_with_title(bytes: &[u8], title_fallback: &str) -> Result<Self, EbookError> {
        if bytes.starts_with(b"Rar!\x1a\x07\x00")
            || bytes.starts_with(b"Rar!\x1a\x07\x01\x00")
            || bytes.starts_with(b"Rar!\x1a\x07")
        {
            return Err(EbookError::InvalidFormat("CBR (RAR format) is not supported in pure-Rust mode (RARv4/RARv5 detected). Please convert the file to CBZ (ZIP format).".to_string()));
        }
        if bytes.starts_with(b"%PDF-") {
            return crate::pdf::PdfBook::parse(bytes, title_fallback);
        }
        if bytes.starts_with(b"PK\x03\x04") {
            if let Ok(archive) = EpubArchive::from_bytes(bytes) {
                if archive.contains("META-INF/container.xml") {
                    if let Ok(book) = Self::from_archive(archive.clone()) {
                        return Ok(book);
                    }
                } else if archive.contains("word/document.xml") {
                    if let Ok(docx) = crate::docx::DocxBook::parse(bytes, title_fallback) {
                        return Ok(docx);
                    }
                } else if archive.contains("content.xml") || archive.contains("meta.xml") {
                    if let Ok(odt) = crate::odt::OdtBook::parse(bytes, title_fallback) {
                        return Ok(odt);
                    }
                }
                return crate::cbz::CbzBook::from_archive(archive, title_fallback);
            }
        }
        if bytes.starts_with(b"{\\rtf") || bytes.starts_with(b"{\\rtf1") {
            return crate::rtf::RtfBook::parse(bytes, title_fallback);
        }
        if crate::kfx::KfxBook::is_kfx(bytes) {
            return crate::kfx::KfxBook::parse(bytes);
        }
        if is_mobi_bytes(bytes) {
            return crate::mobi::MobiBook::parse(bytes);
        }
        if bytes.starts_with(b"ITOLITLS") {
            return crate::lit::LitBook::parse(bytes);
        }
        if let Ok(fb2) = crate::fb2::Fb2Book::parse(bytes) {
            return Ok(fb2);
        }
        if let Ok(text) = std::str::from_utf8(bytes) {
            let is_md =
                text.contains("# ") || text.contains("## ") || title_fallback.ends_with(".md");
            return crate::txt::TxtBook::parse(bytes, title_fallback, is_md);
        }

        Err(EbookError::InvalidFormat(
            "Unsupported or corrupted eBook format".to_string(),
        ))
    }

    /// Open an eBook from any `Read + Seek` stream with a default title.
    pub fn from_reader<R: std::io::Read + std::io::Seek>(reader: R) -> Result<Self, EbookError> {
        Self::from_reader_with_title(reader, "eBook")
    }

    /// Open an eBook from any `Read + Seek` stream with custom title fallback.
    pub fn from_reader_with_title<R: std::io::Read + std::io::Seek>(
        mut reader: R,
        title_fallback: &str,
    ) -> Result<Self, EbookError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|e| EbookError::Io(format!("Failed to read stream: {}", e)))?;
        Self::from_bytes_with_title(&bytes, title_fallback)
    }

    /// Internal builder from an initialized archive.
    fn from_archive(archive: EpubArchive) -> Result<Self, EbookError> {
        // E8 Fix: Detect DRM encryption (ADEPT / LCP)
        if archive.contains("META-INF/rights.xml") || archive.contains("license.lcpl") {
            return Err(EbookError::DrmProtected("DRM protected eBook (ADEPT/LCP). Decryption keys are required to read encrypted content.".to_string()));
        }

        let opf_path = archive.get_opf_path()?;
        let opf_xml = archive.read_string(&opf_path)?;
        let opf = parse_opf(&opf_xml, &opf_path).map_err(EbookError::Xml)?;

        let mut toc = Vec::new();
        let mut landmarks = Vec::new();
        let mut page_list = Vec::new();

        // 1. Try NCX TOC
        if let Some(ncx_item) = opf.manifest.values().find(|i| {
            i.media_type == "application/x-dtbncx+xml" || i.href.ends_with(".ncx") || i.id == "ncx"
        }) {
            if let Ok(ncx_xml) = archive.read_string(&ncx_item.full_path) {
                if let Ok(points) = parse_ncx(&ncx_xml, &ncx_item.full_path) {
                    toc = points;
                }
            }
        }

        // 2. Try NAV XHTML TOC (EPUB 3) & Landmarks & PageList
        if let Some(nav_item) = opf.manifest.values().find(|i| {
            i.properties.contains(&"nav".to_string())
                || i.href.contains("nav.xhtml")
                || i.href.contains("nav.html")
        }) {
            if let Ok(nav_html) = archive.read_string(&nav_item.full_path) {
                if let Ok(points) = parse_nav_xhtml(&nav_html, &nav_item.full_path) {
                    if toc.is_empty() || !points.is_empty() {
                        toc = points;
                    }
                }
                landmarks = parse_landmarks(&nav_html);
                page_list = crate::nav::parse_page_list(&nav_html);
            }
        }

        // Parse META-INF/encryption.xml if present
        let font_deobfuscator = if let Ok(xml) = archive.read_string("META-INF/encryption.xml") {
            FontDeobfuscator::parse_encryption_xml(&xml)
        } else {
            FontDeobfuscator::parse_encryption_xml("")
        };

        // Load spine sections
        let mut sections = Vec::new();
        let mut locations = Locations::default();

        for (idx, spine_item) in opf.spine.iter().enumerate() {
            if let Some(man_item) = opf.manifest.get(&spine_item.idref) {
                match Section::new(
                    idx,
                    spine_item.idref.clone(),
                    man_item.href.clone(),
                    man_item.full_path.clone(),
                    &archive,
                ) {
                    Ok(section) => {
                        locations.add_spine_section(section.index, &section.plain_text);
                        sections.push(section);
                    }
                    Err(err) => {
                        eprintln!(
                            "Warning: Failed to load section {} ({}): {}",
                            idx, man_item.full_path, err
                        );
                    }
                }
            }
        }

        locations.finalize();

        // Load EPUB 3 Media Overlays (SMIL Sync)
        let mut media_overlays = AHashMap::new();
        for item in opf.manifest.values() {
            if item.media_type == "application/smil+xml" || item.href.ends_with(".smil") {
                if let Ok(smil_xml) = archive.read_string(&item.full_path) {
                    if let Ok(pkg) = crate::media_overlay::MediaOverlayPackage::parse_smil(
                        &smil_xml,
                        &item.full_path,
                    ) {
                        media_overlays.insert(item.full_path.clone(), pkg);
                    }
                }
            }
        }

        Ok(Self {
            archive,
            opf,
            toc,
            landmarks,
            page_list,
            sections,
            locations,
            annotations: AnnotationManager::new(),
            layout: RenditionLayout::default(),
            font_deobfuscator,
            before_display_hooks: Vec::new(),
            media_overlays,
            render_cache: parking_lot::Mutex::new(AHashMap::new()),
        })
    }

    /// Access reference to underlying `EpubArchive` container.
    pub fn archive(&self) -> &EpubArchive {
        &self.archive
    }

    /// Access mutable reference to underlying `EpubArchive` container.
    pub fn archive_mut(&mut self) -> &mut EpubArchive {
        &mut self.archive
    }

    /// Access reference to OPF package document.
    pub fn opf(&self) -> &crate::opf::OpfPackage {
        &self.opf
    }

    /// Access list of parsed ebook sections.
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// Access mutable list of parsed ebook sections.
    pub fn sections_mut(&mut self) -> &mut [Section] {
        &mut self.sections
    }

    /// Access reference to location and CFI index.
    pub fn locations(&self) -> &Locations {
        &self.locations
    }

    /// Access reference to annotations manager.
    pub fn annotations(&self) -> &AnnotationManager {
        &self.annotations
    }

    /// Access mutable reference to annotations manager.
    pub fn annotations_mut(&mut self) -> &mut AnnotationManager {
        &mut self.annotations
    }

    /// Access rendition layout property (Reflowable or Pre-paginated).
    pub fn layout(&self) -> &RenditionLayout {
        &self.layout
    }

    /// Access font deobfuscator.
    pub fn font_deobfuscator(&self) -> &FontDeobfuscator {
        &self.font_deobfuscator
    }

    /// Access registered before-display hooks.
    pub fn before_display_hooks(&self) -> &[BeforeDisplayHook] {
        &self.before_display_hooks
    }

    /// Access map of SMIL media overlays.
    pub fn media_overlays(&self) -> &AHashMap<String, crate::media_overlay::MediaOverlayPackage> {
        &self.media_overlays
    }

    /// Register a pre-display HTML transformation hook (Feature 2).
    pub fn register_before_display_hook<F>(&mut self, hook: F)
    where
        F: Fn(&mut String, &str) + Send + Sync + 'static,
    {
        self.before_display_hooks.push(Arc::new(hook));
    }

    /// Metadata of the publication.
    pub fn metadata(&self) -> &Metadata {
        &self.opf.metadata
    }

    /// Spine items list.
    pub fn spine(&self) -> &[SpineItem] {
        &self.opf.spine
    }

    /// Manifest map.
    pub fn manifest(&self) -> &AHashMap<String, ManifestItem> {
        &self.opf.manifest
    }

    /// Table of Contents.
    pub fn toc(&self) -> &[NavPoint] {
        &self.toc
    }

    /// Generate AI / RAG document chunks with EPUB CFI citations and heading hierarchy.
    pub fn to_rag_chunks(&self, config: &crate::rag::RagChunkConfig) -> Vec<crate::rag::RagChunk> {
        crate::rag::RagChunker::chunk_book(self, config)
    }

    /// EPUB 3 Landmarks navigation.
    pub fn landmarks(&self) -> &[Landmark] {
        &self.landmarks
    }

    /// EPUB 3 Page List navigation.
    pub fn page_list(&self) -> &[PageListItem] {
        &self.page_list
    }

    /// Retrieve cover image bytes and mime type (E7 Fix: fall back to cover_id in manifest).
    pub fn cover_image(&self) -> Option<(Vec<u8>, &'static str)> {
        let target_href = self.opf.metadata.cover_href.clone().or_else(|| {
            self.opf
                .metadata
                .cover_id
                .as_ref()
                .and_then(|id| self.opf.manifest.get(id).map(|item| item.full_path.clone()))
        });

        if let Some(ref href) = target_href {
            let mime = EpubArchive::get_mime_type(href);
            if mime.starts_with("image/") {
                if let Ok(bytes) = self.archive.read_bytes(href) {
                    return Some((bytes, mime));
                }
            } else if mime == "application/xhtml+xml" {
                if let Ok(html) = self.archive.read_string(href) {
                    let base_dir = if let Some(idx) = href.rfind('/') {
                        &href[..idx]
                    } else {
                        ""
                    };
                    if let Some(img_src) = extract_first_img_src(&html) {
                        let img_path = crate::archive::resolve_relative_path(base_dir, &img_src);
                        if let Ok(bytes) = self.archive.read_bytes(&img_path) {
                            let img_mime = EpubArchive::get_mime_type(&img_path);
                            return Some((bytes, img_mime));
                        }
                    }
                }
            }
        }
        None
    }

    /// Retrieve raw resource bytes and MIME type for on-demand HTTP streaming or WASM Blob URL creation.
    pub fn get_resource_bytes(&self, path: &str) -> Result<(Vec<u8>, &'static str), String> {
        let clean_path = path.strip_prefix("resource/").unwrap_or(path);
        let bytes = self
            .archive
            .read_bytes(clean_path)
            .map_err(|e| format!("Resource not found in archive: {} ({})", clean_path, e))?;
        let mime = EpubArchive::get_mime_type(clean_path);
        Ok((bytes, mime))
    }

    /// Automatically detects the primary language of the book using `whatlang`.
    /// Falls back to OPF metadata `dc:language` if present, or performs statistical language detection on section text.
    pub fn detect_language(&self) -> Option<String> {
        let meta_lang = self.opf.metadata.language();
        if !meta_lang.trim().is_empty() {
            return Some(meta_lang.to_string());
        }
        for sec in &self.sections {
            if let Some(lang) = sec.detect_language() {
                return Some(lang);
            }
        }
        None
    }

    /// Compresses and serializes the parsed `Book` state into a `zstd`-compressed byte buffer (`Vec<u8>`).
    /// This enables sub-millisecond instant caching and restoration of parsed books.
    #[cfg(feature = "zstd")]
    pub fn export_zstd_cache(&self) -> Result<Vec<u8>, String> {
        let mut sections = self.sections.clone();
        for sec in &mut sections {
            if sec.processed_html == sec.raw_html {
                sec.processed_html.clear();
            }
        }
        let cache = BookCacheState {
            opf: self.opf.clone(),
            toc: self.toc.clone(),
            landmarks: self.landmarks.clone(),
            page_list: self.page_list.clone(),
            sections,
            locations: self.locations.clone(),
            layout: self.layout.clone(),
            archive_files: self.archive.files().clone(),
            annotations: self.annotations.clone(),
            media_overlays: self.media_overlays.clone(),
        };
        let json_bytes = serde_json::to_vec(&cache)
            .map_err(|e| format!("Failed to serialize Book state to JSON: {}", e))?;
        zstd::encode_all(&json_bytes[..], 3).map_err(|e| format!("Zstd compression failed: {}", e))
    }

    /// Deserializes and restores a `Book` state from a `zstd`-compressed byte buffer (`&[u8]`).
    #[cfg(feature = "zstd")]
    pub fn from_zstd_cache(zstd_bytes: &[u8]) -> Result<Self, String> {
        let json_bytes = zstd::decode_all(zstd_bytes)
            .map_err(|e| format!("Zstd decompression failed: {}", e))?;
        let mut cache: BookCacheState = serde_json::from_slice(&json_bytes)
            .map_err(|e| format!("Failed to deserialize Book state from JSON: {}", e))?;

        for sec in &mut cache.sections {
            if sec.processed_html.is_empty() {
                sec.processed_html = sec.raw_html.clone();
            }
        }

        let mut archive = EpubArchive::empty();
        for (path, bytes) in &cache.archive_files {
            archive.insert(path, bytes.clone());
        }
        for sec in &cache.sections {
            if !archive.files().contains_key(&sec.full_path) {
                archive.insert(&sec.full_path, sec.raw_html.as_bytes().to_vec());
            }
        }

        // B6 Fix: Enforce Readium LCP DRM protection check on restored cache
        if archive.contains("META-INF/license.lcpl") {
            if let Ok(license_str) = archive.read_string("META-INF/license.lcpl") {
                if let Ok(lcp) = crate::lcp::LcpLicense::parse(&license_str) {
                    if lcp.encryption.is_some() {
                        return Err("Readium LCP protected eBook requires passphrase validation"
                            .to_string());
                    }
                }
            }
        }

        Ok(Self {
            archive,
            opf: cache.opf,
            toc: cache.toc,
            landmarks: cache.landmarks,
            page_list: cache.page_list,
            sections: cache.sections,
            locations: cache.locations,
            annotations: cache.annotations,
            layout: cache.layout,
            font_deobfuscator: FontDeobfuscator::new(),
            before_display_hooks: Vec::new(),
            media_overlays: cache.media_overlays,
            render_cache: parking_lot::Mutex::new(AHashMap::new()),
        })
    }

    /// Retrieve tokenized TTS words with character range offsets for SpeechSynthesis word synchronization.
    pub fn get_tts_tokens(
        &self,
        index: usize,
    ) -> Result<Vec<crate::section::TtsWordToken>, String> {
        let section = self.get_section(index)?;
        Ok(section.tokenize_tts_words())
    }

    /// Retrieve section HTML annotated with `<span id="tts-w-{index}">` tags for live SpeechSynthesis word-by-word visual highlighting.
    pub fn get_tts_section_html(&self, index: usize) -> Result<String, String> {
        let section = self.get_section(index)?;
        Ok(section.to_tts_annotated_html())
    }

    /// Lazy-load a section by spine index directly from the archive without caching in `self.sections`.
    /// This is ideal for large books (1000+ pages) where eager-loading all sections wastes RAM.
    /// The returned `Section` is fully processed (inlined assets, RTL injection, hooks applied).
    pub fn load_section_lazy(&self, index: usize) -> Result<Section, String> {
        let spine_item = self
            .opf
            .spine
            .get(index)
            .ok_or_else(|| format!("Spine index out of bounds: {}", index))?;
        let man_item =
            self.opf.manifest.get(&spine_item.idref).ok_or_else(|| {
                format!("Manifest item not found for idref: {}", spine_item.idref)
            })?;
        let mut section = Section::new(
            index,
            spine_item.idref.clone(),
            man_item.href.clone(),
            man_item.full_path.clone(),
            &self.archive,
        )?;
        // Inline assets
        section.processed_html = crate::section::process_section_resources(
            &section.raw_html,
            &section.full_path,
            &self.archive,
        );
        // RTL injection
        if self.opf.metadata.direction == crate::metadata::PageProgressionDirection::Rtl {
            if !section.processed_html.contains("dir=\"rtl\"")
                && !section.processed_html.contains("dir='rtl'")
            {
                section.processed_html =
                    section.processed_html.replace("<html", "<html dir=\"rtl\"");
                if !section.processed_html.contains("dir=\"rtl\"") {
                    section.processed_html =
                        section.processed_html.replace("<body", "<body dir=\"rtl\"");
                }
            }
        }
        // Script sanitization
        if !self.layout.allow_scripted_content {
            section.strip_script_content();
        }
        // Apply before_display hooks
        for hook in &self.before_display_hooks {
            hook(&mut section.processed_html, &section.full_path);
        }
        Ok(section)
    }

    /// Retrieve a raw section reference without executing rendering resource inlining hooks.
    pub fn get_section_raw(&self, index: usize) -> Option<&Section> {
        self.sections.get(index)
    }

    /// Retrieve a section by spine index (applying pre-display hooks and automatic RTL dir="rtl" injection).
    pub fn get_section(&self, index: usize) -> Result<Section, String> {
        let mut section = self
            .sections
            .get(index)
            .cloned()
            .ok_or_else(|| format!("Section index out of bounds: {}", index))?;

        if let Some(cached) = self.render_cache.lock().get(&index) {
            section.processed_html = cached.clone();
            return Ok(section);
        }

        if section.processed_html == section.raw_html && !section.raw_html.is_empty() {
            section.processed_html = crate::section::process_section_resources(
                &section.raw_html,
                &section.full_path,
                &self.archive,
            );
        }

        // F6 Fix: Automatic RTL dir="rtl" injection at render time
        if self.opf.metadata.direction == crate::metadata::PageProgressionDirection::Rtl {
            if !section.processed_html.contains("dir=\"rtl\"")
                && !section.processed_html.contains("dir='rtl'")
            {
                section.processed_html =
                    section.processed_html.replace("<html", "<html dir=\"rtl\"");
                if !section.processed_html.contains("dir=\"rtl\"") {
                    section.processed_html =
                        section.processed_html.replace("<body", "<body dir=\"rtl\"");
                }
            }
        }

        // Sanitize script content if scripted content is disabled for security
        if !self.layout.allow_scripted_content {
            section.strip_script_content();
        }

        // Apply registered before_display hooks
        for hook in &self.before_display_hooks {
            hook(&mut section.processed_html, &section.full_path);
        }

        self.render_cache
            .lock()
            .insert(index, section.processed_html.clone());

        Ok(section)
    }

    /// Retrieve section by relative href string (E6 Fix: strict path matching).
    pub fn get_section_by_href(&self, href: &str) -> Result<Section, String> {
        let clean = href.trim();
        let target = clean.split('#').next().unwrap_or(clean);

        for section in &self.sections {
            if section.href == target
                || section.full_path == target
                || section.href.ends_with(&format!("/{}", target))
                || section.full_path.ends_with(&format!("/{}", target))
            {
                return self.get_section(section.index);
            }
        }
        Err(format!("Section not found for href: {}", href))
    }

    /// Retrieve section for a given CFI string.
    pub fn get_section_by_cfi(&self, cfi_str: &str) -> Result<Section, String> {
        let cfi = Cfi::parse(cfi_str)?;
        let spine_idx = cfi.spine_index();
        self.get_section(spine_idx)
    }

    /// Perform full-text search across all spine sections.
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        SearchEngine::search(&self.sections, query, false)
    }

    /// Re-generate locations with custom character chunk size.
    pub fn generate_locations(&mut self, chunk_size: usize) {
        let mut new_locations = Locations::new(chunk_size);
        for section in &self.sections {
            new_locations.add_spine_section(section.index, &section.plain_text);
        }
        new_locations.finalize();
        self.locations = new_locations;
    }

    /// Generate a Readium-compliant unified Locator object for a spine index and character offset (Readium Locator Model).
    pub fn to_readium_locator(
        &self,
        spine_index: usize,
        char_offset: usize,
    ) -> Result<crate::locations::ReadiumLocator, String> {
        let section = self.get_section(spine_index)?;
        let cfi = Cfi::from_spine_index(spine_index, None, char_offset).to_string();

        let section_char_count = section.char_count.max(1);
        let progression = (char_offset as f64 / section_char_count as f64).clamp(0.0, 1.0);

        let loc_entry = self
            .locations
            .location_from_char_offset(spine_index, char_offset);
        let loc_idx = loc_entry.map(|e| e.location).unwrap_or(1);
        let total_progression = self.locations.percentage_from_location(loc_idx);

        let fragment = find_nearest_element_id_anchor(&section.raw_html, char_offset);

        Ok(crate::locations::ReadiumLocator {
            href: section.href.clone(),
            type_: "application/xhtml+xml".to_string(),
            title: Some(format!("Section {}", spine_index + 1)),
            locations: crate::locations::LocatorLocations {
                cfi: Some(cfi),
                fragment,
                position: Some(loc_idx),
                progression,
                total_progression,
            },
            text: Some(serde_json::json!({
                "highlight": section.plain_text.chars().skip(char_offset).take(100).collect::<String>()
            })),
        })
    }

    /// Perform full-text regular expression search across all book sections.
    pub fn search_regex(&self, pattern: &str) -> Result<Vec<crate::search::SearchResult>, String> {
        crate::search::SearchEngine::search_regex(&self.sections, pattern)
    }

    /// Perform structural validation on this Book instance (EpubValidator).
    pub fn validate(&self) -> crate::validator::ValidationReport {
        crate::validator::EpubValidator::validate(self)
    }

    /// Generate stable content & metadata fingerprint for deduplication.
    pub fn fingerprint(&self) -> crate::fingerprint::BookFingerprint {
        crate::fingerprint::FingerprintGenerator::generate(self)
    }

    /// Export BibTeX academic citation for this Book.
    pub fn to_bibtex(&self) -> String {
        crate::citation::CitationExporter::to_bibtex(self.metadata())
    }

    /// Export APA (7th ed.) academic citation for this Book.
    pub fn to_apa(&self) -> String {
        crate::citation::CitationExporter::to_apa(self.metadata())
    }

    /// Export MLA (9th ed.) academic citation for this Book.
    pub fn to_mla(&self) -> String {
        crate::citation::CitationExporter::to_mla(self.metadata())
    }

    /// Export Chicago (17th ed.) academic citation for this Book.
    pub fn to_chicago(&self) -> String {
        crate::citation::CitationExporter::to_chicago(self.metadata())
    }

    /// Extract embedded code blocks and AST nodes using Tree-sitter engine.
    pub fn extract_code_blocks(&self) -> Vec<crate::treesitter::ExtractedCodeBlock> {
        crate::treesitter::TreeSitterEngine::extract_code_blocks(self)
    }

    /// Perform deep case-insensitive search across Table of Contents nodes at any depth level.
    pub fn search_toc(&self, query: &str) -> Vec<crate::nav::TocSearchResult> {
        crate::nav::NavPoint::search(&self.toc, query)
    }

    /// Flatten hierarchical Table of Contents into a linear list with depth levels and breadcrumbs.
    pub fn flatten_toc(&self) -> Vec<crate::nav::NavPointFlat> {
        crate::nav::NavPoint::flatten(&self.toc)
    }

    /// Generate side-by-side synthetic 2-page spread HTML for EPUB 3 Fixed-Layout (FXL) and comic books.
    pub fn get_synthetic_spread(
        &self,
        left_spine_index: usize,
        right_spine_index: Option<usize>,
    ) -> Result<crate::layout::SyntheticSpread, String> {
        let left_section = self.get_section(left_spine_index)?;
        let right_section = match right_spine_index {
            Some(idx) => Some(self.get_section(idx)?),
            None => None,
        };

        let width = left_section.viewport_width.unwrap_or(600.0);
        let height = left_section.viewport_height.unwrap_or(800.0);

        let mut html = String::new();
        html.push_str("<div class=\"epub-fxl-spread-container\" style=\"display:flex; flex-direction:row; justify-content:center; align-items:center; width:100%; height:100vh; background-color:#0f1319;\">");

        html.push_str(&format!(
            "<div class=\"epub-fxl-page page-left\" style=\"width:{:.1}px; height:{:.1}px; overflow:hidden; box-shadow: -4px 0 16px rgba(0,0,0,0.5);\">",
            width, height
        ));
        html.push_str(&left_section.processed_html);
        html.push_str("</div>");

        if let Some(right_sec) = right_section {
            let r_width = right_sec.viewport_width.unwrap_or(width);
            let r_height = right_sec.viewport_height.unwrap_or(height);

            html.push_str(&format!(
                "<div class=\"epub-fxl-page page-right\" style=\"width:{:.1}px; height:{:.1}px; overflow:hidden; box-shadow: 4px 0 16px rgba(0,0,0,0.5);\">",
                r_width, r_height
            ));
            html.push_str(&right_sec.processed_html);
            html.push_str("</div>");
        }

        html.push_str("</div>");

        Ok(crate::layout::SyntheticSpread {
            left_index: left_spine_index,
            right_index: right_spine_index,
            combined_html: html,
            width,
            height,
        })
    }

    #[cfg(feature = "mmap")]
    /// Open eBook from a file using zero-copy memory-mapped I/O (via `memmap2`).
    pub fn from_mmap<P: AsRef<std::path::Path>>(path: P) -> Result<Self, EbookError> {
        let p = path.as_ref();
        let file = std::fs::File::open(p)
            .map_err(|e| EbookError::Io(format!("Failed to open file for mmap: {}", e)))?;
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(|e| EbookError::Io(format!("Failed to memory-map file: {}", e)))?
        };
        Self::from_bytes(&mmap)
    }

    /// Export any loaded eBook (EPUB, MOBI, AZW3, FB2, KEPUB, LIT, CBZ, PDF, ODT, TXT, MD) as a clean, compliant EPUB 3 ZIP archive buffer.
    pub fn export_epub3_bytes(&self) -> Result<Vec<u8>, String> {
        use std::io::Write;
        use zip::write::FileOptions;

        let mut zip_buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));

            let stored_options = FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Stored)
                .unix_permissions(0o644);
            zip.start_file("mimetype", stored_options)
                .map_err(|e| format!("Failed to write mimetype: {}", e))?;
            zip.write_all(b"application/epub+zip")
                .map_err(|e| format!("Failed to write mimetype content: {}", e))?;

            let deflated_options = FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o644);

            zip.start_file("META-INF/container.xml", deflated_options)
                .map_err(|e| format!("Failed to write container.xml: {}", e))?;
            let container_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
            zip.write_all(container_xml.as_bytes())
                .map_err(|e| format!("Failed to write container.xml content: {}", e))?;

            for section in &self.sections {
                let sec_path = format!("OEBPS/section_{}.html", section.index);
                zip.start_file(&sec_path, deflated_options)
                    .map_err(|e| format!("Failed to write {}: {}", sec_path, e))?;

                let html_source = if section.raw_html.is_empty() {
                    &section.processed_html
                } else {
                    &section.raw_html
                };

                let trimmed = html_source.trim();
                let trimmed_low = trimmed.to_lowercase();
                let doc_html = if trimmed_low.starts_with("<!doctype")
                    || trimmed_low.starts_with("<?xml")
                    || trimmed_low.starts_with("<html")
                {
                    html_source.to_string()
                } else {
                    format!(
                        "<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\">\n<head><title>Section {}</title></head>\n<body>\n{}\n</body>\n</html>",
                        section.index + 1,
                        html_source
                    )
                };
                zip.write_all(doc_html.as_bytes())
                    .map_err(|e| format!("Failed to write {} content: {}", sec_path, e))?;
            }

            zip.start_file("OEBPS/nav.xhtml", deflated_options)
                .map_err(|e| format!("Failed to write nav.xhtml: {}", e))?;
            let mut nav_html = String::from(
                "<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n<head><title>Navigation</title></head>\n<body>\n<nav epub:type=\"toc\" id=\"toc\">\n",
            );
            render_nav_points_xml(&self.toc, &mut nav_html, &self.sections);
            nav_html.push_str("</nav>\n</body>\n</html>");
            zip.write_all(nav_html.as_bytes())
                .map_err(|e| format!("Failed to write nav.xhtml content: {}", e))?;

            zip.start_file("OEBPS/content.opf", deflated_options)
                .map_err(|e| format!("Failed to write content.opf: {}", e))?;

            let uuid_str = generate_rfc4122_uuid_v4(&self.opf.metadata.title);
            let lang = self
                .opf
                .metadata
                .languages
                .first()
                .map(|s| s.as_str())
                .unwrap_or("en");
            let mut opf_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="pub-id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="pub-id">urn:uuid:{}</dc:identifier>
    <dc:title>{}</dc:title>
    <dc:language>{}</dc:language>
"#,
                uuid_str,
                xml_escape(&self.opf.metadata.title),
                xml_escape(lang)
            );
            // Emit dc:creator for every author
            for creator in &self.opf.metadata.creators {
                opf_xml.push_str(&format!(
                    "    <dc:creator>{}</dc:creator>\n",
                    xml_escape(creator)
                ));
            }
            opf_xml.push_str("  </metadata>\n  <manifest>\n    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n");

            for section in &self.sections {
                opf_xml.push_str(&format!(
                    "    <item id=\"sec_{}\" href=\"section_{}.html\" media-type=\"application/xhtml+xml\"/>\n",
                    section.index, section.index
                ));
            }

            opf_xml.push_str("  </manifest>\n  <spine>\n");
            for section in &self.sections {
                opf_xml.push_str(&format!("    <itemref idref=\"sec_{}\"/>\n", section.index));
            }
            opf_xml.push_str("  </spine>\n</package>");

            zip.write_all(opf_xml.as_bytes())
                .map_err(|e| format!("Failed to write content.opf content: {}", e))?;

            zip.finish()
                .map_err(|e| format!("Failed to finalize EPUB ZIP archive: {}", e))?;
        }

        Ok(zip_buf)
    }

    /// Export loaded eBook as a minified, asset-deduplicated, and CSS-purged EPUB 3 archive.
    pub fn export_optimized_epub3_bytes(
        &self,
        options: &crate::optimizer::EpubOptimizerOptions,
    ) -> Result<Vec<u8>, String> {
        let mut cloned = self.clone();
        crate::optimizer::EpubOptimizer::optimize(&mut cloned, options);
        cloned.export_epub3_bytes()
    }
}

fn is_mobi_bytes(bytes: &[u8]) -> bool {
    if bytes.len() < 68 {
        return false;
    }
    let type_creator = &bytes[60..68];
    type_creator == b"BOOKMOBI"
        || type_creator == b"TEXtRECD"
        || &bytes[60..64] == b"BOOK"
        || &bytes[64..68] == b"MOBI"
}

fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    if s.len() < prefix.len() || !s.is_char_boundary(prefix.len()) {
        return false;
    }
    s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

fn find_ignore_case(s: &str, pat: &str) -> Option<usize> {
    if pat.is_empty() {
        return Some(0);
    }
    if s.len() < pat.len() {
        return None;
    }
    for i in 0..=s.len() - pat.len() {
        if s.is_char_boundary(i) {
            if let Some(sub) = s.get(i..i + pat.len()) {
                if sub.eq_ignore_ascii_case(pat) {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn extract_first_img_src(html: &str) -> Option<String> {
    let mut i = 0;
    while i < html.len() {
        if starts_with_ignore_case(&html[i..], "<img") {
            let rem = &html[i..];
            let mut j = 0;
            while j < rem.len() {
                if starts_with_ignore_case(&rem[j..], "src=\"") {
                    let val_start = j + 5;
                    if let Some(end_idx) = rem[val_start..].find('"') {
                        return Some(rem[val_start..val_start + end_idx].to_string());
                    }
                } else if starts_with_ignore_case(&rem[j..], "src='") {
                    let val_start = j + 5;
                    if let Some(end_idx) = rem[val_start..].find('\'') {
                        return Some(rem[val_start..val_start + end_idx].to_string());
                    }
                }
                if let Some(ch) = rem[j..].chars().next() {
                    j += ch.len_utf8();
                } else {
                    break;
                }
            }
        }
        if let Some(ch) = html[i..].chars().next() {
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    None
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn render_nav_points_xml(pts: &[NavPoint], out: &mut String, sections: &[Section]) {
    render_nav_points_xml_depth(pts, out, 0, sections);
}

fn render_nav_points_xml_depth(
    pts: &[NavPoint],
    out: &mut String,
    depth: usize,
    sections: &[Section],
) {
    if depth > 32 || pts.is_empty() {
        return;
    }
    out.push_str("<ol>\n");
    for pt in pts {
        let remapped_href = remap_toc_href(&pt.href, sections);
        out.push_str(&format!(
            "<li><a href=\"{}\">{}</a>",
            xml_escape(&remapped_href),
            xml_escape(&pt.label)
        ));
        if !pt.subitems.is_empty() {
            out.push('\n');
            render_nav_points_xml_depth(&pt.subitems, out, depth + 1, sections);
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ol>\n");
}

fn remap_toc_href(href: &str, sections: &[Section]) -> String {
    let parts: Vec<&str> = href.split('#').collect();
    let path = parts[0];
    let anchor = if parts.len() > 1 {
        format!("#{}", parts[1])
    } else {
        String::new()
    };

    let norm_path = crate::archive::normalize_path(path);

    for sec in sections {
        let sec_norm = crate::archive::normalize_path(&sec.full_path);
        if sec_norm == norm_path || sec_norm.ends_with(&norm_path) || norm_path.ends_with(&sec_norm)
        {
            return format!("section_{}.html{}", sec.index, anchor);
        }
    }

    if let Some(sec) = sections.first() {
        format!("section_{}.html{}", sec.index, anchor)
    } else {
        href.to_string()
    }
}

fn generate_rfc4122_uuid_v4(seed_text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    seed_text.hash(&mut hasher);
    let h1 = hasher.finish();

    let mut hasher2 = DefaultHasher::new();
    (h1, seed_text).hash(&mut hasher2);
    let h2 = hasher2.finish();

    let bytes1 = h1.to_be_bytes();
    let bytes2 = h2.to_be_bytes();

    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&bytes1);
    b[8..].copy_from_slice(&bytes2);

    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;

    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        u16::from_be_bytes([b[4], b[5]]),
        u16::from_be_bytes([b[6], b[7]]),
        u16::from_be_bytes([b[8], b[9]]),
        ((u64::from_be_bytes([b[10], b[11], b[12], b[13], b[14], b[15], 0, 0])) >> 16)
    )
}

fn find_nearest_element_id_anchor(html: &str, char_offset: usize) -> Option<String> {
    let max_byte_offset = html
        .char_indices()
        .nth(char_offset)
        .map(|(i, _)| i)
        .unwrap_or_else(|| html.len());

    let mut search_idx = 0;
    let mut last_id: Option<String> = None;

    while search_idx < html.len() {
        if let Some(idx) = find_ignore_case(&html[search_idx..], " id=\"") {
            let abs_idx = search_idx + idx + 5;
            if abs_idx > max_byte_offset {
                break;
            }
            if let Some(end_quote) = html[abs_idx..].find('"') {
                let id_val = &html[abs_idx..abs_idx + end_quote];
                if !id_val.trim().is_empty() {
                    last_id = Some(id_val.to_string());
                }
                search_idx = abs_idx + end_quote + 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if last_id.is_none() {
        if let Some(idx) = find_ignore_case(html, " id=\"") {
            let abs_idx = idx + 5;
            if let Some(end_quote) = html[abs_idx..].find('"') {
                let id_val = &html[abs_idx..abs_idx + end_quote];
                if !id_val.trim().is_empty() {
                    return Some(id_val.to_string());
                }
            }
        }
    }

    last_id
}

/// Serializable cache representation of a `Book` state for `zstd` compressed state caching.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BookCacheState {
    pub opf: crate::opf::OpfPackage,
    pub toc: Vec<NavPoint>,
    pub landmarks: Vec<Landmark>,
    pub page_list: Vec<PageListItem>,
    pub sections: Vec<Section>,
    pub locations: Locations,
    pub layout: RenditionLayout,
    pub archive_files: AHashMap<String, Vec<u8>>,
    pub annotations: AnnotationManager,
    pub media_overlays: AHashMap<String, crate::media_overlay::MediaOverlayPackage>,
}

// ─── Async API (requires `async` feature + tokio) ──────────────────────────

/// Non-blocking async eBook loading API.
/// These functions require the `async` feature and a tokio runtime.
#[cfg(feature = "async")]
pub mod async_api {
    use super::Book;
    use crate::error::EbookError;
    use std::path::Path;

    /// Asynchronously load an eBook from a filesystem path.
    /// Uses `tokio::fs::read` for non-blocking I/O — ideal for async server handlers.
    ///
    /// # Example
    /// ```ignore
    /// let book = ebook_rs::book::async_api::from_file_async("book.epub").await?;
    /// ```
    pub async fn from_file_async<P: AsRef<Path>>(path: P) -> Result<Book, EbookError> {
        let p = path.as_ref();
        let bytes = tokio::fs::read(p)
            .await
            .map_err(|e| EbookError::Io(format!("Async read failed for {}: {}", p.display(), e)))?;
        let filename = p.file_stem().and_then(|s| s.to_str()).unwrap_or("eBook");
        Book::from_bytes_with_title(&bytes, filename)
    }

    /// Asynchronously load an eBook by reading bytes via any async reader.
    /// Returns a parsed `Book` or a descriptive error.
    pub async fn from_bytes_async(bytes: Vec<u8>) -> Result<Book, EbookError> {
        tokio::task::spawn_blocking(move || Book::from_bytes(&bytes))
            .await
            .map_err(|e| EbookError::Custom(format!("Async book parse task failed: {}", e)))?
    }

    /// Asynchronously lazy-load a section by index.
    /// Parses and processes one section without blocking the async runtime.
    pub async fn load_section_lazy_async(
        book: std::sync::Arc<Book>,
        index: usize,
    ) -> Result<super::super::section::Section, String> {
        tokio::task::spawn_blocking(move || book.load_section_lazy(index))
            .await
            .map_err(|e| format!("Async section load task failed: {}", e))?
    }
}
