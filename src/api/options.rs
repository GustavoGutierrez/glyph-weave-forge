#[cfg(feature = "fs")]
use std::path::{Path, PathBuf};

use crate::core::ports::{RenderBackend, ResourceResolver};
use crate::core::{BuiltInTheme, LayoutMode, PageSize, ThemeConfig};

/// Supported markdown input sources.
pub enum MarkdownSource<'a> {
    /// Read markdown from a filesystem path.
    #[cfg(feature = "fs")]
    Path(&'a Path),
    /// Read markdown from UTF-8 text.
    Text(&'a str),
    /// Read markdown from UTF-8 bytes.
    Bytes(&'a [u8]),
}

/// Supported PDF output targets.
pub enum OutputTarget<'a> {
    /// Write the final PDF to an explicit file path.
    #[cfg(feature = "fs")]
    File(&'a Path),
    /// Write the final PDF into a directory using a derived or explicit file name.
    #[cfg(feature = "fs")]
    Directory(&'a Path),
    /// Return the final PDF bytes in memory.
    Memory,
    #[doc(hidden)]
    __Lifetime(std::marker::PhantomData<&'a ()>),
}

/// Built-in renderer backends available through the composition layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RenderBackendSelection {
    /// The lightweight native-compatible fallback backend.
    #[default]
    Minimal,
    /// The optional Typst-backed backend.
    #[cfg(feature = "renderer-typst")]
    Typst,
}

/// Conversion options used by the builder and pipeline.
///
/// Most callers should use [`crate::Forge`] directly. This type is public so the
/// full builder state can be inspected, created ahead of time, or replaced via
/// [`crate::Forge::with_options`].
pub struct ConvertOptions<'a> {
    /// Markdown input source.
    pub source: Option<MarkdownSource<'a>>,
    /// PDF output target.
    pub output: Option<OutputTarget<'a>>,
    /// Optional file name override for directory outputs.
    pub output_file_name: Option<&'a str>,
    /// Requested page size.
    pub page_size: PageSize,
    /// Requested layout mode.
    pub layout_mode: LayoutMode,
    /// Theme configuration, including optional JSON overrides.
    pub theme: ThemeConfig,
    /// Selected built-in backend.
    pub backend: RenderBackendSelection,
    /// Optional resource resolver adapter.
    pub resource_resolver: Option<Box<dyn ResourceResolver>>,
    /// Optional renderer override.
    pub renderer: Option<Box<dyn RenderBackend>>,
}

impl<'a> Default for ConvertOptions<'a> {
    fn default() -> Self {
        Self {
            source: None,
            output: None,
            output_file_name: None,
            page_size: PageSize::A4,
            layout_mode: LayoutMode::Paged,
            theme: ThemeConfig {
                built_in: Some(BuiltInTheme::Professional),
                custom_theme_json: None,
            },
            backend: RenderBackendSelection::Minimal,
            resource_resolver: None,
            renderer: None,
        }
    }
}

/// Result of a conversion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PdfOutput {
    /// In-memory PDF bytes, present for [`OutputTarget::Memory`].
    pub bytes: Option<Vec<u8>>,
    #[cfg(feature = "fs")]
    /// Written path, present for file and directory outputs when `fs` is enabled.
    pub written_path: Option<PathBuf>,
}
