use crate::annotations::AnnotationManager;
use crate::archive::EpubArchive;
use crate::cfi::Cfi;
use crate::layout::RenditionLayout;
use crate::locations::Locations;
use crate::metadata::{GuideItem, Metadata, SpineItem};
use crate::nav::{parse_nav_xhtml, parse_ncx, NavPoint};
use crate::opf::{parse_container_xml, parse_opf, OpfPackage};
use crate::search::{SearchEngine, SearchResult};
use crate::section::Section;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Core Book struct representing an EPUB publication.
pub struct Book {
    pub archive: EpubArchive,
    pub opf: OpfPackage,
    pub toc: Vec<NavPoint>,
    pub sections: Vec<Section>,
    pub locations: Locations,
    pub annotations: AnnotationManager,
    pub layout: RenditionLayout,
}

impl Book {
    /// Load an EPUB book from in-memory ZIP raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let archive = EpubArchive::from_bytes(bytes)?;

        // 1. Locate OPF rootfile path from META-INF/container.xml
        let container_xml = archive.read_string("META-INF/container.xml")?;
        let opf_path = parse_container_xml(&container_xml)?;

        // 2. Parse OPF document
        let opf_xml = archive.read_string(&opf_path)?;
        let opf = parse_opf(&opf_xml, &opf_path)?;

        // 3. Parse Table of Contents (NCX or NAV XHTML)
        let mut toc = Vec::new();
        if let Some(ref nav_id) = opf.nav_item_id {
            if let Some(nav_item) = opf.manifest.get(nav_id) {
                if let Ok(nav_html) = archive.read_string(&nav_item.full_path) {
                    if let Ok(parsed_toc) = parse_nav_xhtml(&nav_html, &nav_item.full_path) {
                        toc = parsed_toc;
                    }
                }
            }
        }

        if toc.is_empty() {
            if let Some(ref toc_id) = opf.toc_item_id {
                if let Some(toc_item) = opf.manifest.get(toc_id) {
                    if let Ok(ncx_xml) = archive.read_string(&toc_item.full_path) {
                        if let Ok(parsed_toc) = parse_ncx(&ncx_xml, &toc_item.full_path) {
                            toc = parsed_toc;
                        }
                    }
                }
            }
        }

        // Fallback: check any .ncx file in archive if toc is still empty
        if toc.is_empty() {
            for path in archive.list_files() {
                if path.ends_with(".ncx") {
                    if let Ok(ncx_xml) = archive.read_string(&path) {
                        if let Ok(parsed_toc) = parse_ncx(&ncx_xml, &path) {
                            toc = parsed_toc;
                            break;
                        }
                    }
                }
            }
        }

        // 4. Load all spine sections into memory and process resources
        let mut sections = Vec::with_capacity(opf.spine.len());
        let mut locations = Locations::new(150);

        for item in &opf.spine {
            match Section::new(
                item.index,
                item.idref.clone(),
                item.href.clone(),
                item.href.clone(),
                &archive,
            ) {
                Ok(section) => {
                    locations.add_spine_section(section.index, &section.plain_text);
                    sections.push(section);
                }
                Err(err) => {
                    eprintln!("⚠️ Failed to load section {}: {}", item.href, err);
                }
            }
        }
        locations.finalize();

        Ok(Self {
            archive,
            opf,
            toc,
            sections,
            locations,
            annotations: AnnotationManager::new(),
            layout: RenditionLayout::default(),
        })
    }

    /// Load an EPUB book from a file system path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        Self::from_bytes(&bytes)
    }

    /// Get book metadata.
    pub fn metadata(&self) -> &Metadata {
        &self.opf.metadata
    }

    /// Get manifest items.
    pub fn manifest(&self) -> &HashMap<String, crate::metadata::ManifestItem> {
        &self.opf.manifest
    }

    /// Get spine itemrefs list.
    pub fn spine(&self) -> &[SpineItem] {
        &self.opf.spine
    }

    /// Get Table of Contents.
    pub fn toc(&self) -> &[NavPoint] {
        &self.toc
    }

    /// Get landmarks / guide references.
    pub fn landmarks(&self) -> &[GuideItem] {
        &self.opf.guide
    }

    /// Retrieve cover image bytes and mime type.
    pub fn cover_image(&self) -> Option<(Vec<u8>, &'static str)> {
        if let Some(ref href) = self.opf.metadata.cover_href {
            if let Ok(bytes) = self.archive.read_bytes(href) {
                let mime = EpubArchive::get_mime_type(href);
                return Some((bytes, mime));
            }
        }
        None
    }

    /// Retrieve section by spine index (0..N).
    pub fn get_section(&self, index: usize) -> Result<&Section, String> {
        self.sections
            .get(index)
            .ok_or_else(|| format!("Spine index out of bounds: {}", index))
    }

    /// Retrieve section by href or full path.
    pub fn get_section_by_href(&self, href: &str) -> Result<&Section, String> {
        let clean = crate::archive::normalize_path(href);
        let target = clean.split('#').next().unwrap_or(&clean);

        for section in &self.sections {
            if section.href == target
                || section.full_path == target
                || section.href.ends_with(target)
            {
                return Ok(section);
            }
        }
        Err(format!("Section not found for href: {}", href))
    }

    /// Retrieve section for a given CFI string.
    pub fn get_section_by_cfi(&self, cfi_str: &str) -> Result<&Section, String> {
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
