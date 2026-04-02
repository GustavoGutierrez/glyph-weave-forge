use serde_json::Value;

/// Output pagination strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Paginate rendered lines across pages.
    Paged,
    /// Keep all rendered lines in a single page payload.
    SinglePage,
}

/// Target page size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    /// ISO A4.
    A4,
    /// North American letter.
    Letter,
    /// North American legal.
    Legal,
    /// Caller-defined size in millimeters.
    ///
    /// Both values must be greater than zero.
    Custom { width_mm: f32, height_mm: f32 },
}

/// Predefined visual themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInTheme {
    /// Preset intended for invoices and compact business documents.
    Invoice,
    /// Preset intended for article-like documents.
    ScientificArticle,
    /// General-purpose default preset.
    Professional,
    /// Preset tuned for technical and engineering-style output.
    Engineering,
    /// Preset for straightforward informational content.
    Informational,
}

/// Theme selection and optional JSON overrides.
#[derive(Debug, Clone, Default)]
pub struct ThemeConfig {
    /// Optional built-in preset. When omitted, rendering falls back to `Professional`.
    pub built_in: Option<BuiltInTheme>,
    /// Optional JSON overrides for theme profile fields such as `name`,
    /// `body_font_size_pt`, `code_font_size_pt`, `heading_scale`, and `margin_mm`.
    pub custom_theme_json: Option<Value>,
}
