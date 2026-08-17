use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticSpread {
    pub left_index: usize,
    pub right_index: Option<usize>,
    pub combined_html: String,
    pub width: f64,
    pub height: f64,
}

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
pub struct ViewportManagerConfig {
    pub preload_count: usize,
    pub continuous: bool,
    pub intersection_observer: bool,
}

impl Default for ViewportManagerConfig {
    fn default() -> Self {
        Self {
            preload_count: 2,
            continuous: true,
            intersection_observer: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetDeliveryStrategy {
    InlinedBase64,
    ResourceStream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WritingMode {
    #[default]
    HorizontalLtr,
    HorizontalRtl,
    VerticalRl,
    VerticalLr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenditionLayout {
    pub layout_mode: LayoutMode,
    pub flow_mode: FlowMode,
    pub spread_mode: SpreadMode,
    pub writing_mode: WritingMode,
    pub theme: Theme,
    pub font_family: String,
    pub font_size_px: u32,
    pub line_height: f32,
    pub margin_px: u32,
    pub allow_scripted_content: bool,
    pub viewport_config: ViewportManagerConfig,
    pub asset_delivery: AssetDeliveryStrategy,
    pub custom_font_family: Option<String>,
    pub custom_font_url: Option<String>,
}

impl Default for RenditionLayout {
    fn default() -> Self {
        Self {
            layout_mode: LayoutMode::Reflowable,
            flow_mode: FlowMode::Paginated,
            spread_mode: SpreadMode::Auto,
            writing_mode: WritingMode::HorizontalLtr,
            theme: Theme::Light,
            font_family: "Inter, system-ui, -apple-system, sans-serif".to_string(),
            font_size_px: 16,
            line_height: 1.6,
            margin_px: 32,
            allow_scripted_content: false,
            viewport_config: ViewportManagerConfig::default(),
            asset_delivery: AssetDeliveryStrategy::InlinedBase64,
            custom_font_family: None,
            custom_font_url: None,
        }
    }
}

impl RenditionLayout {
    /// Inject custom reader font family and font URL (F5 Fix).
    pub fn set_custom_font(&mut self, font_family: &str, font_url_or_b64: &str) {
        self.custom_font_family = Some(font_family.to_string());
        self.custom_font_url = Some(font_url_or_b64.to_string());
    }

    /// Generate dynamic CSS rules to inject into section HTML.
    pub fn to_css_override(&self) -> String {
        let (bg, fg, link) = match self.theme {
            Theme::Light => ("#ffffff", "#1e293b", "#2563eb"),
            Theme::Dark => ("#0f172a", "#f8fafc", "#60a5fa"),
            Theme::Sepia => ("#fef3c7", "#451a03", "#b45309"),
            Theme::Solarized => ("#073642", "#839496", "#268bd2"),
            Theme::HighContrast => ("#000000", "#ffffff", "#ffff00"),
        };

        let font_rule =
            if let (Some(fam), Some(url)) = (&self.custom_font_family, &self.custom_font_url) {
                format!(
                    r#"
                @font-face {{
                    font-family: '{}';
                    src: url('{}');
                }}
                "#,
                    fam, url
                )
            } else {
                "".to_string()
            };

        let active_font = self
            .custom_font_family
            .as_deref()
            .unwrap_or(&self.font_family);

        let mode_css = match self.writing_mode {
            WritingMode::HorizontalLtr => "direction: ltr;",
            WritingMode::HorizontalRtl => "direction: rtl;",
            WritingMode::VerticalRl => {
                "writing-mode: vertical-rl; -webkit-writing-mode: vertical-rl;"
            }
            WritingMode::VerticalLr => {
                "writing-mode: vertical-lr; -webkit-writing-mode: vertical-lr;"
            }
        };

        format!(
            r#"
            {}
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
                {}
            }}
            a {{
                color: var(--reader-link) !important;
            }}
            img, svg, video {{
                max-width: 100% !important;
                height: auto !important;
            }}
            "#,
            font_rule,
            bg,
            fg,
            link,
            active_font,
            self.font_size_px,
            self.line_height,
            self.margin_px,
            mode_css
        )
    }

    /// Calculate Fixed Layout (FXL) scale factor and CSS transform matrix string
    /// given target page dimensions (vp_width, vp_height) and screen container bounds.
    pub fn compute_fxl_scale(
        &self,
        vp_width: f64,
        vp_height: f64,
        container_width: f64,
        container_height: f64,
    ) -> Option<(f64, String)> {
        if vp_width <= 0.0 || vp_height <= 0.0 || container_width <= 0.0 || container_height <= 0.0
        {
            return None;
        }

        let scale_w = container_width / vp_width;
        let scale_h = container_height / vp_height;
        let scale = scale_w.min(scale_h);

        let css_transform = format!(
            "width: {}px; height: {}px; transform: scale({}); transform-origin: 0 0;",
            vp_width, vp_height, scale
        );

        Some((scale, css_transform))
    }
}
