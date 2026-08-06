use crate::section::Section;
use serde::{Deserialize, Serialize};

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
}

/// Deterministic, DOM-free Reflow Paginator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflowPaginator {
    pub font_size_px: u32,
    pub line_height: f32,
    pub viewport_width_px: u32,
    pub viewport_height_px: u32,
    pub margin_px: u32,
}

impl Default for ReflowPaginator {
    fn default() -> Self {
        Self {
            font_size_px: 16,
            line_height: 1.6,
            viewport_width_px: 800,
            viewport_height_px: 600,
            margin_px: 32,
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

        let char_width = self.font_size_px as f32 * 0.55;
        let line_height_px = self.font_size_px as f32 * self.line_height;

        let chars_per_line = (usable_width / char_width).max(10.0) as usize;
        let lines_per_page = (usable_height / line_height_px).max(4.0) as usize;

        let chars_per_page = (chars_per_line * lines_per_page).max(100);

        let total_chars = text.chars().count();
        if total_chars == 0 {
            return SectionPageMap {
                total_pages: 1,
                page_ranges: vec![PageRange {
                    page_number: 1,
                    start_char: 0,
                    end_char: 0,
                }],
            };
        }

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
        }
    }

    /// Calculate virtual page map for a given section.
    pub fn paginate_section(&self, section: &Section) -> SectionPageMap {
        self.paginate_text(&section.plain_text)
    }
}
