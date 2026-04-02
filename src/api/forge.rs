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
///
/// `Forge` is a fluent builder. Configure a markdown source, configure an output
/// target, optionally adjust page/theme/backend settings, then call [`convert`](Self::convert).
///
/// # Examples
///
/// ```rust
/// use glyphweaveforge::Forge;
///
/// let pdf = Forge::new()
///     .from_text("# Hello")
///     .to_memory()
///     .convert()
///     .expect("conversion should succeed");
///
/// assert!(pdf.bytes.expect("bytes should exist").starts_with(b"%PDF"));
/// ```
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
    ///
    /// Defaults:
    /// - backend: [`RenderBackendSelection::Minimal`]
    /// - page size: [`PageSize::A4`]
    /// - layout mode: [`LayoutMode::Paged`]
    /// - theme: [`BuiltInTheme::Professional`]
    /// - source/output: unset until configured
    pub fn new() -> Self {
        Self {
            options: ConvertOptions::default(),
        }
    }

    /// Replaces the current options wholesale.
    ///
    /// This is useful when you want to construct [`ConvertOptions`] separately and
    /// then hand them to the builder in one step.
    pub fn with_options(mut self, options: ConvertOptions<'a>) -> Self {
        self.options = options;
        self
    }

    /// Reads markdown from a file path.
    ///
    /// When the `fs` feature is enabled, relative assets referenced from the markdown
    /// can also be resolved from the markdown file's parent directory.
    #[cfg(feature = "fs")]
    pub fn from_path(mut self, path: &'a Path) -> Self {
        self.options.source = Some(MarkdownSource::Path(path));
        self
    }

    /// Reads markdown from UTF-8 text.
    ///
    /// Text inputs use `document` as their synthetic source name when deriving
    /// directory output file names.
    pub fn from_text(mut self, text: &'a str) -> Self {
        self.options.source = Some(MarkdownSource::Text(text));
        self
    }

    /// Reads markdown from UTF-8 bytes.
    ///
    /// Returns [`ForgeError::InvalidUtf8`] from [`convert`](Self::convert) if the bytes are not valid UTF-8.
    pub fn from_bytes(mut self, bytes: &'a [u8]) -> Self {
        self.options.source = Some(MarkdownSource::Bytes(bytes));
        self
    }

    /// Writes the generated PDF to a specific file.
    ///
    /// File outputs return [`PdfOutput`] with `bytes = None` and `written_path = Some(path)`.
    #[cfg(feature = "fs")]
    pub fn to_file(mut self, path: &'a Path) -> Self {
        self.options.output = Some(OutputTarget::File(path));
        self
    }

    /// Writes the generated PDF into a directory.
    ///
    /// The final file name is derived from the source name unless overridden with
    /// [`with_output_file_name`](Self::with_output_file_name).
    #[cfg(feature = "fs")]
    pub fn to_directory(mut self, path: &'a Path) -> Self {
        self.options.output = Some(OutputTarget::Directory(path));
        self
    }

    /// Returns the generated PDF as bytes in memory.
    ///
    /// Memory outputs return [`PdfOutput`] with `bytes = Some(...)`.
    pub fn to_memory(mut self) -> Self {
        self.options.output = Some(OutputTarget::Memory);
        self
    }

    /// Overrides the output file name used by directory outputs.
    ///
    /// If the provided name does not end with `.pdf`, the extension is appended automatically.
    pub fn with_output_file_name(mut self, file_name: &'a str) -> Self {
        self.options.output_file_name = Some(file_name);
        self
    }

    /// Sets the target page size.
    ///
    /// Built-in sizes are [`PageSize::A4`], [`PageSize::Letter`], and [`PageSize::Legal`].
    /// Custom sizes use millimeters.
    pub fn with_page_size(mut self, page_size: PageSize) -> Self {
        self.options.page_size = page_size;
        self
    }

    /// Sets the target layout mode.
    ///
    /// [`LayoutMode::Paged`] paginates lines across multiple pages.
    /// [`LayoutMode::SinglePage`] keeps all rendered lines in one page payload.
    pub fn with_layout_mode(mut self, layout_mode: LayoutMode) -> Self {
        self.options.layout_mode = layout_mode;
        self
    }

    /// Selects a built-in theme.
    ///
    /// Built-in themes are `Invoice`, `ScientificArticle`, `Professional`,
    /// `Engineering`, and `Informational`.
    pub fn with_theme(mut self, theme: BuiltInTheme) -> Self {
        self.options.theme.built_in = Some(theme);
        self
    }

    /// Applies a full theme configuration.
    ///
    /// Use this when you want to combine a built-in preset with JSON overrides such
    /// as `name`, `body_font_size_pt`, `code_font_size_pt`, `heading_scale`, or `margin_mm`.
    pub fn with_theme_config(mut self, theme: ThemeConfig) -> Self {
        self.options.theme = theme;
        self
    }

    /// Selects one of the built-in renderer backends.
    ///
    /// The default selection is [`RenderBackendSelection::Minimal`].
    /// [`RenderBackendSelection::Typst`] is available when the `renderer-typst`
    /// feature is enabled.
    ///
    /// Calling this clears any custom renderer previously injected through
    /// [`with_renderer`](Self::with_renderer).
    pub fn with_backend(mut self, backend: RenderBackendSelection) -> Self {
        self.options.backend = backend;
        self.options.renderer = None;
        self
    }

    /// Registers a typed resource resolver implementation.
    ///
    /// This is the lower-level extension point for custom asset loading.
    pub fn with_resource_adapter<R>(mut self, resolver: R) -> Self
    where
        R: ResourceResolver + 'static,
    {
        self.options.resource_resolver = Some(Box::new(resolver));
        self
    }

    /// Registers a closure-based resource resolver.
    ///
    /// The closure receives each referenced `href` and should return the resolved
    /// bytes or an `io::Error`. `NotFound` errors become visible missing-asset
    /// fallbacks instead of hard conversion failures.
    pub fn with_resource_resolver<F>(self, resolver: F) -> Self
    where
        F: Fn(&str) -> io::Result<Vec<u8>> + Send + Sync + 'static,
    {
        self.with_resource_adapter(ClosureResolver::new(resolver))
    }

    /// Replaces the render backend.
    ///
    /// This overrides the built-in backend selection and uses the provided public
    /// [`RenderBackend`] implementation directly.
    pub fn with_renderer<R>(mut self, renderer: R) -> Self
    where
        R: RenderBackend + 'static,
    {
        self.options.renderer = Some(Box::new(renderer));
        self
    }

    /// Converts the configured markdown source into a PDF.
    ///
    /// # Errors
    ///
    /// Returns:
    /// - [`ForgeError::MissingSource`] when no source was configured.
    /// - [`ForgeError::MissingOutput`] when no output target was configured.
    /// - source, resource, configuration, render, or output errors that occur during the pipeline.
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
