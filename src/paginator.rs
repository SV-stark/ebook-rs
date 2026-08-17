use crate::section::Section;
use serde::{Deserialize, Serialize};

pub use crate::layout::WritingMode;

/// Reading progression direction for page navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TextDirection {
    #[default]
    Ltr,
    Rtl,
}

/// Page range mapping character offsets to virtual page numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRange {
    pub page_number: usize,
    pub start_char: usize,
    pub end_char: usize,
}

/// Paginated section map containing page counts and character break intervals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionPageMap {
    pub total_pages: usize,
    pub page_ranges: Vec<PageRange>,
    pub writing_mode: WritingMode,
    pub is_rtl: bool,
}

/// Deterministic, DOM-free Reflow Paginator with full CJK vertical writing and RTL pagination support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflowPaginator {
    pub font_size_px: u32,
    pub line_height: f32,
    pub viewport_width_px: u32,
    pub viewport_height_px: u32,
    pub margin_px: u32,
    pub writing_mode: WritingMode,
    pub direction: TextDirection,
}

impl Default for ReflowPaginator {
    fn default() -> Self {
        Self {
            font_size_px: 16,
            line_height: 1.6,
            viewport_width_px: 800,
            viewport_height_px: 600,
            margin_px: 32,
            writing_mode: WritingMode::HorizontalLtr,
            direction: TextDirection::Ltr,
        }
    }
}

impl ReflowPaginator {
    pub fn new(
        font_size_px: u32,
        line_height: f32,
        viewport_width_px: u32,
        viewport_height_px: u32,
        margin_px: u32,
    ) -> Self {
        Self {
            font_size_px,
            line_height,
            viewport_width_px,
            viewport_height_px,
            margin_px,
            writing_mode: WritingMode::HorizontalLtr,
            direction: TextDirection::Ltr,
        }
    }

    /// Builder method to configure writing mode.
    pub fn with_writing_mode(mut self, mode: WritingMode) -> Self {
        self.direction = match mode {
            WritingMode::HorizontalRtl | WritingMode::VerticalRl => TextDirection::Rtl,
            WritingMode::HorizontalLtr | WritingMode::VerticalLr => TextDirection::Ltr,
        };
        self.writing_mode = mode;
        self
    }

    /// Builder method to configure reading direction.
    pub fn with_direction(mut self, direction: TextDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Whether current paginator layout is vertical.
    pub fn is_vertical(&self) -> bool {
        matches!(
            self.writing_mode,
            WritingMode::VerticalRl | WritingMode::VerticalLr
        )
    }

    /// Whether current paginator layout is right-to-left.
    pub fn is_rtl(&self) -> bool {
        matches!(self.direction, TextDirection::Rtl)
            || matches!(
                self.writing_mode,
                WritingMode::HorizontalRtl | WritingMode::VerticalRl
            )
    }

    /// CSS properties string for reader styling matching this paginator configuration.
    pub fn css_properties(&self) -> String {
        match self.writing_mode {
            WritingMode::HorizontalLtr => {
                "writing-mode: horizontal-tb; direction: ltr; text-align: left;".to_string()
            }
            WritingMode::HorizontalRtl => {
                "writing-mode: horizontal-tb; direction: rtl; text-align: right;".to_string()
            }
            WritingMode::VerticalRl => {
                "writing-mode: vertical-rl; text-orientation: upright; direction: rtl;".to_string()
            }
            WritingMode::VerticalLr => {
                "writing-mode: vertical-lr; text-orientation: upright; direction: ltr;".to_string()
            }
        }
    }

    /// Calculate virtual reflow page breaks for plain text content.
    pub fn paginate_text(&self, text: &str) -> SectionPageMap {
        let usable_width = self
            .viewport_width_px
            .saturating_sub(self.margin_px * 2)
            .max(200) as f32;
        let usable_height = self
            .viewport_height_px
            .saturating_sub(self.margin_px * 2)
            .max(200) as f32;

        let total_chars = text.chars().count();
        if total_chars == 0 {
            return SectionPageMap {
                total_pages: 1,
                page_ranges: vec![PageRange {
                    page_number: 1,
                    start_char: 0,
                    end_char: 0,
                }],
                writing_mode: self.writing_mode,
                is_rtl: self.is_rtl(),
            };
        }

        let is_cjk_dominant = {
            let cjk_count = text.chars().take(200).filter(|c| is_cjk_char(*c)).count();
            let total_checked = text.chars().take(200).count();
            total_checked > 0 && (cjk_count as f32 / total_checked as f32) > 0.3
        };

        let chars_per_page = match self.writing_mode {
            WritingMode::VerticalRl | WritingMode::VerticalLr => {
                // In vertical mode: columns advance horizontally, characters flow top-to-bottom.
                let char_height = self.font_size_px as f32 * 1.05;
                let col_spacing_px = self.font_size_px as f32 * self.line_height;

                let chars_per_col = (usable_height / char_height).max(6.0) as usize;
                let cols_per_page = (usable_width / col_spacing_px).max(4.0) as usize;
                (chars_per_col * cols_per_page).max(80)
            }
            WritingMode::HorizontalLtr | WritingMode::HorizontalRtl => {
                let char_width = if is_cjk_dominant {
                    self.font_size_px as f32 * 0.95
                } else {
                    self.font_size_px as f32 * 0.55
                };
                let line_height_px = self.font_size_px as f32 * self.line_height;

                let chars_per_line = (usable_width / char_width).max(10.0) as usize;
                let lines_per_page = (usable_height / line_height_px).max(4.0) as usize;
                (chars_per_line * lines_per_page).max(100)
            }
        };

        let total_pages = total_chars.div_ceil(chars_per_page);
        let mut page_ranges = Vec::with_capacity(total_pages);

        for p in 0..total_pages {
            let start = p * chars_per_page;
            let end = ((p + 1) * chars_per_page).min(total_chars);
            page_ranges.push(PageRange {
                page_number: p + 1,
                start_char: start,
                end_char: end,
            });
        }

        SectionPageMap {
            total_pages,
            page_ranges,
            writing_mode: self.writing_mode,
            is_rtl: self.is_rtl(),
        }
    }

    /// Calculate virtual page map for a given section.
    pub fn paginate_section(&self, section: &Section) -> SectionPageMap {
        self.paginate_text(&section.plain_text)
    }
}

/// Helper function to detect CJK ideographs and kana characters.
#[inline]
pub fn is_cjk_char(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Unified Ideographs Extension A
        | '\u{3040}'..='\u{309F}' // Hiragana
        | '\u{30A0}'..='\u{30FF}' // Katakana
        | '\u{31F0}'..='\u{31FF}' // Katakana Phonetic Extensions
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        | '\u{3000}'..='\u{303F}' // CJK Symbols and Punctuation
        | '\u{FF00}'..='\u{FFEF}' // Halfwidth and Fullwidth Forms
    )
}
