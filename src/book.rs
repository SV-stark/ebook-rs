use crate::Cfi;
use crate::annotations::AnnotationManager;
use crate::archive::EpubArchive;
use crate::deobfuscate::FontDeobfuscator;
use crate::layout::RenditionLayout;
use crate::locations::Locations;
use crate::metadata::{ManifestItem, Metadata, SpineItem};
use crate::nav::{Landmark, NavPoint, PageListItem, parse_landmarks, parse_nav_xhtml, parse_ncx};
use crate::opf::parse_opf;
use crate::search::{SearchEngine, SearchResult};
use crate::section::Section;
use std::collections::HashMap;
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
    pub media_overlays: HashMap<String, crate::media_overlay::MediaOverlayPackage>,
}

impl Book {
    /// Load an EPUB, KEPUB, MOBI, AZW3, FB2, LIT, CBZ, or CBR book from a file path.
    pub fn from_file(path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read ebook file {}: {}", path, e))?;
        let filename = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Comic Book");
        Self::from_bytes_with_title(&bytes, filename)
    }

    /// Open an EPUB, KEPUB, MOBI, AZW3, FB2, LIT, CBZ, or CBR ebook from an in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        Self::from_bytes_with_title(bytes, "eBook")
    }

    /// Open an eBook from an in-memory byte slice with a title fallback.
    pub fn from_bytes_with_title(bytes: &[u8], title_fallback: &str) -> Result<Self, String> {
        if bytes.starts_with(b"Rar!\x1a\x07\x00")
            || bytes.starts_with(b"Rar!\x1a\x07\x01\x00")
            || bytes.starts_with(b"Rar!\x1a\x07")
        {
            return Err("CBR (RAR format) is not supported in pure-Rust mode (RARv4/RARv5 detected). Please convert the file to CBZ (ZIP format).".to_string());
        }
        if bytes.starts_with(b"PK\x03\x04") {
            if let Ok(archive) = EpubArchive::from_bytes(bytes) {
                if archive.contains("META-INF/container.xml") {
                    if let Ok(book) = Self::from_archive(archive) {
                        return Ok(book);
                    }
                }
            }
            crate::cbz::CbzBook::parse(bytes, title_fallback)
        } else if let Ok(mobi) = crate::mobi::MobiBook::parse(bytes) {
            Ok(mobi)
        } else if let Ok(fb2) = crate::fb2::Fb2Book::parse(bytes) {
            Ok(fb2)
        } else if let Ok(lit) = crate::lit::LitBook::parse(bytes) {
            Ok(lit)
        } else if let Ok(cbz) = crate::cbz::CbzBook::parse(bytes, title_fallback) {
            Ok(cbz)
        } else {
            Err("Unsupported or corrupted eBook format".to_string())
        }
    }

    /// Internal builder from an initialized archive.
    fn from_archive(archive: EpubArchive) -> Result<Self, String> {
        // E8 Fix: Detect DRM encryption (ADEPT / LCP)
        if archive.contains("META-INF/rights.xml") || archive.contains("license.lcpl") {
            return Err("DRM protected eBook (ADEPT/LCP). Decryption keys are required to read encrypted content.".to_string());
        }

        let opf_path = archive.get_opf_path()?;
        let opf_xml = archive.read_string(&opf_path)?;
        let opf = parse_opf(&opf_xml, &opf_path)?;

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
        let mut media_overlays = HashMap::new();
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
        })
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
    pub fn manifest(&self) -> &HashMap<String, ManifestItem> {
        &self.opf.manifest
    }

    /// Table of Contents.
    pub fn toc(&self) -> &[NavPoint] {
        &self.toc
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

    /// Retrieve a section by spine index (applying pre-display hooks and automatic RTL dir="rtl" injection).
    pub fn get_section(&self, index: usize) -> Result<Section, String> {
        let mut section = self
            .sections
            .get(index)
            .cloned()
            .ok_or_else(|| format!("Section index out of bounds: {}", index))?;

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
}

fn extract_first_img_src(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    if let Some(img_idx) = lower.find("<img") {
        let rem = &html[img_idx..];
        let lower_rem = &lower[img_idx..];
        if let Some(src_idx) = lower_rem.find("src=\"") {
            let val_start = src_idx + 5;
            if let Some(end_idx) = rem[val_start..].find('"') {
                return Some(rem[val_start..val_start + end_idx].to_string());
            }
        }
    }
    None
}
