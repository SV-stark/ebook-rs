#![allow(clippy::collapsible_if, clippy::collapsible_match)]

pub mod analytics;
pub mod annotations;
pub mod archive;
pub mod book;
pub mod cbz;
pub mod cfi;
pub mod deobfuscate;
pub mod fb2;
pub mod footnote;
pub mod layout;
pub mod lit;
pub mod locations;
pub mod media_overlay;
pub mod metadata;
pub mod mobi;
pub mod nav;
#[cfg(feature = "opds")]
pub mod opds;
pub mod opf;
pub mod paginator;
pub mod sample_builder;
pub mod search;
pub mod section;
pub mod stream_zip;
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
pub use deobfuscate::FontDeobfuscator;
pub use fb2::Fb2Book;
pub use footnote::Footnote;
pub use layout::{
    AssetDeliveryStrategy, FlowMode, LayoutMode, RenditionLayout, SpreadMode, Theme,
    ViewportManagerConfig,
};
pub use lit::LitBook;
pub use locations::{LocationEntry, Locations};
pub use media_overlay::{
    MediaOverlayPackage, MediaOverlayParallel, MediaOverlaySequence, SmilAudioClip, SmilClock,
    SmilTextRef,
};
pub use metadata::{
    AccessibilityMetadata, GuideItem, ManifestItem, Metadata, PageProgressionDirection, SpineItem,
};
pub use mobi::{MobiBook, decompress_palmdoc};
pub use nav::{Landmark, NavPoint, PageListItem};
#[cfg(feature = "opds")]
pub use opds::{OpdsEntry, OpdsFeed, OpdsLink};
pub use paginator::{PageRange, ReflowPaginator, SectionPageMap};
pub use sample_builder::generate_sample_epub;
pub use search::{SearchEngine, SearchResult};
pub use section::Section;
pub use stream_zip::{ZipEntryLocation, ZipHeaderReader};
pub use webpub::{WebpubLink, WebpubManifest, WebpubMetadata};

#[cfg(feature = "server")]
pub use server::ReaderServer;

#[cfg(feature = "wasm")]
pub use wasm::WasmBook;
