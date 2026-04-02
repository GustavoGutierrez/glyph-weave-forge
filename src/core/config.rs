use serde_json::Value;

/// Output pagination strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Paginate lines across pages.
    Paged,
    /// Keep all lines in a single page payload.
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
    Custom { width_mm: f32, height_mm: f32 },
}

/// Predefined visual themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInTheme {
    Invoice,
    ScientificArticle,
    Professional,
    Engineering,
    Informational,
}

/// Theme selection and optional JSON overrides.
#[derive(Debug, Clone, Default)]
pub struct ThemeConfig {
    pub built_in: Option<BuiltInTheme>,
    pub custom_theme_json: Option<Value>,
}
