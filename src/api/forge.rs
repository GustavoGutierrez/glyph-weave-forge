use std::io;
#[cfg(feature = "fs")]
use std::path::Path;

use crate::adapters::resources::ClosureResolver;
use crate::api::options::{
    ConvertOptions, MarkdownSource, OutputTarget, PdfOutput, RenderBackendSelection,
};
use crate::composition::ensure_renderer;
use crate::core::ports::{RenderBackend, ResourceResolver};
use crate::core::{BuiltInTheme, LayoutMode, PageSize, ThemeConfig};
use crate::core::{ForgeError, Result};
use crate::pipeline::convert::convert;

/// Main entrypoint for Markdown to PDF conversion.
pub struct Forge<'a> {
    options: ConvertOptions<'a>,
}

impl<'a> Default for Forge<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Forge<'a> {
    /// Creates a new builder with pragmatic defaults.
    pub fn new() -> Self {
        Self {
            options: ConvertOptions::default(),
        }
    }

    /// Replaces the current options wholesale.
    pub fn with_options(mut self, options: ConvertOptions<'a>) -> Self {
        self.options = options;
        self
    }

    /// Reads markdown from a file path.
    #[cfg(feature = "fs")]
    pub fn from_path(mut self, path: &'a Path) -> Self {
        self.options.source = Some(MarkdownSource::Path(path));
        self
    }

    /// Reads markdown from UTF-8 text.
    pub fn from_text(mut self, text: &'a str) -> Self {
        self.options.source = Some(MarkdownSource::Text(text));
        self
    }

    /// Reads markdown from UTF-8 bytes.
    pub fn from_bytes(mut self, bytes: &'a [u8]) -> Self {
        self.options.source = Some(MarkdownSource::Bytes(bytes));
        self
    }

    /// Writes the generated PDF to a specific file.
    #[cfg(feature = "fs")]
    pub fn to_file(mut self, path: &'a Path) -> Self {
        self.options.output = Some(OutputTarget::File(path));
        self
    }

    /// Writes the generated PDF into a directory.
    #[cfg(feature = "fs")]
    pub fn to_directory(mut self, path: &'a Path) -> Self {
        self.options.output = Some(OutputTarget::Directory(path));
        self
    }

    /// Returns the generated PDF as bytes in memory.
    pub fn to_memory(mut self) -> Self {
        self.options.output = Some(OutputTarget::Memory);
        self
    }

    /// Overrides the output file name used by directory outputs.
    pub fn with_output_file_name(mut self, file_name: &'a str) -> Self {
        self.options.output_file_name = Some(file_name);
        self
    }

    /// Sets the target page size.
    pub fn with_page_size(mut self, page_size: PageSize) -> Self {
        self.options.page_size = page_size;
        self
    }

    /// Sets the target layout mode.
    pub fn with_layout_mode(mut self, layout_mode: LayoutMode) -> Self {
        self.options.layout_mode = layout_mode;
        self
    }

    /// Selects a built-in theme.
    pub fn with_theme(mut self, theme: BuiltInTheme) -> Self {
        self.options.theme.built_in = Some(theme);
        self
    }

    /// Applies a full theme configuration.
    pub fn with_theme_config(mut self, theme: ThemeConfig) -> Self {
        self.options.theme = theme;
        self
    }

    /// Selects one of the built-in renderer backends.
    pub fn with_backend(mut self, backend: RenderBackendSelection) -> Self {
        self.options.backend = backend;
        self.options.renderer = None;
        self
    }

    /// Registers a typed resource resolver implementation.
    pub fn with_resource_adapter<R>(mut self, resolver: R) -> Self
    where
        R: ResourceResolver + 'static,
    {
        self.options.resource_resolver = Some(Box::new(resolver));
        self
    }

    /// Registers a closure-based resource resolver.
    pub fn with_resource_resolver<F>(self, resolver: F) -> Self
    where
        F: Fn(&str) -> io::Result<Vec<u8>> + Send + Sync + 'static,
    {
        self.with_resource_adapter(ClosureResolver::new(resolver))
    }

    /// Replaces the render backend.
    pub fn with_renderer<R>(mut self, renderer: R) -> Self
    where
        R: RenderBackend + 'static,
    {
        self.options.renderer = Some(Box::new(renderer));
        self
    }

    /// Converts the configured markdown source into a PDF.
    pub fn convert(mut self) -> Result<PdfOutput> {
        if self.options.source.is_none() {
            return Err(ForgeError::MissingSource);
        }
        if self.options.output.is_none() {
            return Err(ForgeError::MissingOutput);
        }
        ensure_renderer(&mut self.options);
        convert(&self.options)
    }
}
