use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    Reflowable,
    PrePaginated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlowMode {
    Paginated,
    Scrolled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpreadMode {
    Auto,
    None,
    Double,
    Single,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    Sepia,
    Solarized,
    HighContrast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenditionLayout {
    pub layout_mode: LayoutMode,
    pub flow_mode: FlowMode,
    pub spread_mode: SpreadMode,
    pub theme: Theme,
    pub font_family: String,
    pub font_size_px: u32,
    pub line_height: f32,
    pub margin_px: u32,
}

impl Default for RenditionLayout {
    fn default() -> Self {
        Self {
            layout_mode: LayoutMode::Reflowable,
            flow_mode: FlowMode::Paginated,
            spread_mode: SpreadMode::Auto,
            theme: Theme::Light,
            font_family: "Inter, system-ui, -apple-system, sans-serif".to_string(),
            font_size_px: 16,
            line_height: 1.6,
            margin_px: 32,
        }
    }
}

impl RenditionLayout {
    /// Generate dynamic CSS rules to inject into section HTML.
    pub fn to_css_override(&self) -> String {
        let (bg, fg, link) = match self.theme {
            Theme::Light => ("#ffffff", "#1e293b", "#2563eb"),
            Theme::Dark => ("#0f172a", "#f8fafc", "#60a5fa"),
            Theme::Sepia => ("#fef3c7", "#451a03", "#b45309"),
            Theme::Solarized => ("#073642", "#839496", "#268bd2"),
            Theme::HighContrast => ("#000000", "#ffffff", "#ffff00"),
        };

        format!(
            r#"
            :root {{
                --reader-bg: {};
                --reader-fg: {};
                --reader-link: {};
            }}
            body {{
                background-color: var(--reader-bg) !important;
                color: var(--reader-fg) !important;
                font-family: {} !important;
                font-size: {}px !important;
                line-height: {} !important;
                padding: {}px !important;
                margin: 0 auto !important;
                max-width: 850px !important;
                box-sizing: border-box !important;
                word-wrap: break-word !important;
            }}
            a {{
                color: var(--reader-link) !important;
            }}
            img, svg, video {{
                max-width: 100% !important;
                height: auto !important;
            }}
            "#,
            bg, fg, link, self.font_family, self.font_size_px, self.line_height, self.margin_px
        )
    }
}
