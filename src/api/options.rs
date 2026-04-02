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
pub struct ConvertOptions<'a> {
    pub source: Option<MarkdownSource<'a>>,
    pub output: Option<OutputTarget<'a>>,
    pub output_file_name: Option<&'a str>,
    pub page_size: PageSize,
    pub layout_mode: LayoutMode,
    pub theme: ThemeConfig,
    pub backend: RenderBackendSelection,
    pub resource_resolver: Option<Box<dyn ResourceResolver>>,
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
    pub bytes: Option<Vec<u8>>,
    #[cfg(feature = "fs")]
    pub written_path: Option<PathBuf>,
}
