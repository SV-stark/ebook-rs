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
}

impl Book {
    /// Load an EPUB book from a file path.
    pub fn from_file(path: &str) -> Result<Self, String> {
        let archive = EpubArchive::open(path)?;
        Self::from_archive(archive)
    }

    /// Open an EPUB, MOBI, or AZW3 ebook from an in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() >= 68 && &bytes[60..68] == b"BOOKMOBI"
            || (bytes.len() >= 68 && &bytes[60..68] == b"TEXtREDR")
        {
            crate::mobi::MobiBook::parse(bytes)
        } else if bytes.starts_with(b"PK\x03\x04") {
            let archive = EpubArchive::from_bytes(bytes)?;
            Self::from_archive(archive)
        } else {
            // Attempt MOBI/AZW3 fallback parsing first, then EPUB archive fallback
            if let Ok(mobi_book) = crate::mobi::MobiBook::parse(bytes) {
                Ok(mobi_book)
            } else {
                let archive = EpubArchive::from_bytes(bytes)?;
                Self::from_archive(archive)
            }
        }
    }

    /// Internal builder from an initialized archive.
    fn from_archive(archive: EpubArchive) -> Result<Self, String> {
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

    /// Retrieve cover image bytes and mime type.
    pub fn cover_image(&self) -> Option<(Vec<u8>, &'static str)> {
        if let Some(ref href) = self.opf.metadata.cover_href {
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

    /// Retrieve a section by spine index (applying pre-display hooks).
    pub fn get_section(&self, index: usize) -> Result<Section, String> {
        let mut section = self
            .sections
            .get(index)
            .cloned()
            .ok_or_else(|| format!("Section index out of bounds: {}", index))?;

        // Apply registered before_display hooks
        for hook in &self.before_display_hooks {
            hook(&mut section.processed_html, &section.full_path);
        }

        Ok(section)
    }

    /// Retrieve section by relative href string.
    pub fn get_section_by_href(&self, href: &str) -> Result<Section, String> {
        let clean = href.trim();
        let target = clean.split('#').next().unwrap_or(clean);

        for section in &self.sections {
            if section.href == target
                || section.full_path == target
                || section.href.ends_with(target)
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
