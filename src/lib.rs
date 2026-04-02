#![doc(html_root_url = "https://docs.rs/glyphweaveforge/latest/glyphweaveforge/")]
#![doc = "GlyphWeaveForge is a Markdown-to-PDF library with explicit adapter boundaries.\n\n"]
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
pub use core::ports::{RenderBackend, ResolvedAsset, ResourceResolver, ResourceStatus};
pub use core::{BuiltInTheme, LayoutMode, PageSize, ThemeConfig};
pub use core::{ForgeError, Result};

#[cfg(test)]
mod tests;
