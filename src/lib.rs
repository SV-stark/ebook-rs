#![allow(clippy::collapsible_if, clippy::collapsible_match)]

pub mod analytics;
pub mod annotations;
pub mod archive;
pub mod book;
pub mod cbz;
pub mod cfi;
pub mod citation;
pub mod deobfuscate;
pub mod dom;
pub mod fb2;
pub mod fingerprint;
pub mod footnote;
pub mod layout;
pub mod lcp;
pub mod lit;
pub mod locations;
pub mod media_overlay;
pub mod metadata;
pub mod mobi;
pub mod nav;
pub mod odt;
#[cfg(feature = "opds")]
pub mod opds;
pub mod opf;
pub mod paginator;
pub mod pdf;
pub mod sample_builder;
pub mod search;
pub mod section;
pub mod stream_zip;
pub mod treesitter;
pub mod txt;
pub mod ffi;
pub mod rag;
pub mod validator;
pub mod wasm;
pub mod webpub;

#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub mod web_ui;

pub use analytics::ReadingAnalytics;
pub use annotations::{Annotation, AnnotationManager, AnnotationType};
pub use archive::{EpubArchive, HttpRangeRequest};
pub use book::Book;
pub use cbz::CbzBook;
pub use cfi::{Cfi, CfiDomTarget, CfiOffset, CfiPath, CfiStep};
pub use citation::{CitationExporter, CitationStyle};
pub use deobfuscate::FontDeobfuscator;
pub use dom::{DomNode, EbookDomTree, decode_bytes_with_encoding, sanitize_and_repair_xml};
pub use fb2::Fb2Book;
pub use fingerprint::{BookFingerprint, FingerprintGenerator};
pub use footnote::Footnote;
pub use layout::{
    AssetDeliveryStrategy, FlowMode, LayoutMode, RenditionLayout, SpreadMode, SyntheticSpread,
    Theme, ViewportManagerConfig, WritingMode,
};
pub use lcp::{LcpDecryptor, LcpLicense, LcpRights, LcpUser};
pub use lit::LitBook;
pub use locations::{LocationEntry, Locations, LocatorLocations, ReadiumLocator};
pub use media_overlay::{
    MediaOverlayPackage, MediaOverlayParallel, MediaOverlaySequence, SmilAudioClip, SmilClock,
    SmilTextRef,
};
pub use metadata::{
    AccessibilityMetadata, GuideItem, ManifestItem, Metadata, PageProgressionDirection, SpineItem,
};
pub use mobi::{MobiBook, decompress_palmdoc};
pub use nav::{Landmark, NavPoint, NavPointFlat, PageListItem, TocSearchResult};
pub use odt::OdtBook;
#[cfg(feature = "opds")]
pub use opds::{OpdsEntry, OpdsFeed, OpdsLink};
pub use paginator::{PageRange, ReflowPaginator, SectionPageMap};
pub use pdf::PdfBook;
pub use rag::{RagChunk, RagChunkConfig, RagChunker};
pub use sample_builder::generate_sample_epub;
pub use search::{SearchEngine, SearchResult};
pub use section::{Section, TtsWordToken};
pub use stream_zip::{ZipEntryLocation, ZipHeaderReader};
pub use treesitter::{ExtractedCodeBlock, SyntaxNodeInfo, TreeSitterEngine};
pub use txt::TxtBook;
pub use validator::{
    EpubValidator, UniversalEpub3Exporter, ValidationError, ValidationReport, ValidationSeverity,
};
pub use webpub::{WebpubLink, WebpubManifest, WebpubMetadata};

#[cfg(feature = "server")]
pub use server::ReaderServer;

#[cfg(feature = "wasm")]
pub use wasm::WasmBook;
