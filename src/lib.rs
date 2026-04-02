#![doc(html_root_url = "https://docs.rs/glyphweaveforge/latest/glyphweaveforge/")]
#![doc = "GlyphWeaveForge is a Markdown-to-PDF library with explicit adapter boundaries.\n\nThe public API is centered on `Forge`, with stable builder-style configuration for source selection, output selection, page settings, theme selection, backend selection, and adapter injection.\n\n"]
#![doc = include_str!("../README.md")]

mod adapters;
mod api;
mod composition;
mod core;
mod math;
mod pipeline;

pub use api::{
    ConvertOptions, Forge, MarkdownSource, OutputTarget, PdfOutput, RenderBackendSelection,
};
pub use core::Document;
pub use core::ports::{RenderBackend, ResolvedAsset, ResourceResolver, ResourceStatus};
pub use core::ports::{RenderRequest, ResourceContext};
pub use core::{BuiltInTheme, LayoutMode, PageSize, ThemeConfig};
pub use core::{ForgeError, Result};

#[cfg(test)]
mod tests;
