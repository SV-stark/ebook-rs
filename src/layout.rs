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
    /// Deliver book CSS as an inlined `<style>` block (merged) instead of a base64
    /// `<link href="data:text/css;base64,...">`. Useful for renderers without data-URI support.
    InlinedStyle,
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
    /// Inject custom reader font family and font URL (F5 Fix) with CSS injection sanitization.
    pub fn set_custom_font(&mut self, font_family: &str, font_url_or_b64: &str) {
        // Sanitize font family name: allow alphanumeric, spaces, hyphens, and underscores only
        let safe_family: String = font_family
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
            .collect();

        // Sanitize font URL/Base64: reject control chars, quotes, braces, semicolons, and javascript: URIs
        let is_unsafe_url = font_url_or_b64
            .contains(['\'', '"', ';', '{', '}', '<', '>', '\n', '\r'])
            || font_url_or_b64
                .trim()
                .to_ascii_lowercase()
                .starts_with("javascript:");

        if !safe_family.is_empty() && !is_unsafe_url {
            self.custom_font_family = Some(safe_family);
            self.custom_font_url = Some(font_url_or_b64.trim().to_string());
        }
    }

    /// Generate dynamic CSS rules to inject into section HTML.
    ///
    /// Every interpolated value is either drawn from a fixed enum or sanitized, so the result
    /// cannot escape the `<style>` element it is embedded in.
    pub fn to_css_override(&self) -> String {
        let (bg, fg, link) = match self.theme {
            Theme::Light => ("#ffffff", "#1e293b", "#2563eb"),
            Theme::Dark => ("#0f172a", "#f8fafc", "#60a5fa"),
            Theme::Sepia => ("#fef3c7", "#451a03", "#b45309"),
            Theme::Solarized => ("#073642", "#839496", "#268bd2"),
            Theme::HighContrast => ("#000000", "#ffffff", "#ffff00"),
        };

        // Sanitize both the family and the URL. `set_custom_font` sanitizes on the way in, but
        // these are public fields and can also arrive via `Deserialize` (e.g. a zstd cache).
        let custom_family = self
            .custom_font_family
            .as_deref()
            .map(sanitize_font_stack)
            .filter(|f| !f.is_empty());
        let custom_url = self
            .custom_font_url
            .as_deref()
            .map(sanitize_css_url)
            .filter(|u| !u.is_empty());

        let font_rule = match (custom_family.as_deref(), custom_url.as_deref()) {
            (Some(fam), Some(url)) => {
                format!(
                    r#"
                @font-face {{
                    font-family: '{fam}';
                    src: url('{url}');
                }}
                "#
                )
            }
            _ => String::new(),
        };

        let active_font = {
            // Fall through the whole chain: configured custom family -> configured base family
            // -> sans-serif. Hard-coding the last step when the custom family was present but
            // sanitized away discarded the user's real `font_family`.
            let s = sanitize_font_stack(
                self.custom_font_family
                    .as_deref()
                    .filter(|f| !sanitize_font_stack(f).is_empty())
                    .unwrap_or(&self.font_family),
            );
            if s.is_empty() {
                "sans-serif".to_string()
            } else {
                s
            }
        };

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

/// Sanitize a font-family value for safe interpolation into a CSS declaration.
///
/// `RenditionLayout::font_family` is a public field and ends up inside a `body { ... }` rule
/// that the reader injects as an inline `<style>`. Without filtering, a value such as
/// `x; } </style><script>...</script>` breaks out of the style element -- and the reader CSP
/// allows `style-src 'unsafe-inline'`, making this a script-injection primitive.
fn sanitize_font_stack(input: &str) -> String {
    const MAX_LEN: usize = 200;
    let filtered: String = input
        .chars()
        .take(MAX_LEN)
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | ',' | '\'' | '.'))
        .collect();
    let trimmed = filtered.trim();
    if trimmed.is_empty() || trimmed.contains('{') || trimmed.contains('}') {
        String::new()
    } else {
        trimmed.to_string()
    }
}

/// Sanitize a font `src` value for interpolation into `url('...')`.
///
/// Only characters that could terminate the `url('…')` token or start another declaration are
/// rejected. `;` and `,` are safe inside a quoted URL, and rejecting them silently dropped every
/// `data:font/woff2;base64,…` URI — which is the documented shape of `custom_font_url` — so the
/// entire `@font-face` rule disappeared instead of being emitted.
fn sanitize_css_url(input: &str) -> String {
    // Base64 font payloads are large; a 4 KB cap discarded every real one.
    const MAX_LEN: usize = 1024 * 1024;
    let trimmed = input.trim();
    if trimmed.len() > MAX_LEN {
        return String::new();
    }
    // Reject anything that could terminate the `url('...')` string or start another declaration.
    if trimmed.contains('\'')
        || trimmed.contains('"')
        || trimmed.contains('{')
        || trimmed.contains('}')
        || trimmed.contains('<')
        || trimmed.contains('>')
        || trimmed.contains('\\')
        || trimmed.contains('\n')
        || trimmed.contains('\r')
        || trimmed.to_ascii_lowercase().starts_with("javascript:")
    {
        return String::new();
    }
    trimmed.to_string()
}
