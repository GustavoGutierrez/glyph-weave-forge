#[cfg(feature = "fs")]
use std::path::PathBuf;

use crate::core::{Document, Result};
use crate::core::{LayoutMode, PageSize, ThemeConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceStatus {
    /// The referenced asset was successfully loaded.
    Loaded,
    /// The referenced asset could not be loaded and should render as a visible fallback.
    Missing,
}

/// Resolved asset payload returned by a [`ResourceResolver`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAsset {
    /// Original target as referenced from Markdown.
    pub original: String,
    /// Best-effort detected image format from the file extension.
    pub format: Option<&'static str>,
    /// Loaded bytes when the asset is available.
    pub bytes: Option<Vec<u8>>,
    /// Resolution status.
    pub status: ResourceStatus,
    /// Human-readable resolver message used by fallback rendering.
    pub message: String,
    #[cfg(feature = "fs")]
    /// Filesystem path when the asset came from disk.
    pub path: Option<PathBuf>,
}

impl ResolvedAsset {
    pub fn missing(target: &str, message: impl Into<String>) -> Self {
        Self {
            original: target.to_owned(),
            format: detect_format(target),
            bytes: None,
            status: ResourceStatus::Missing,
            message: message.into(),
            #[cfg(feature = "fs")]
            path: None,
        }
    }

    #[cfg(feature = "fs")]
    pub fn loaded(
        target: &str,
        bytes: Vec<u8>,
        message: impl Into<String>,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            original: target.to_owned(),
            format: detect_format(target),
            bytes: Some(bytes),
            status: ResourceStatus::Loaded,
            message: message.into(),
            path,
        }
    }

    #[cfg(not(feature = "fs"))]
    pub fn loaded(target: &str, bytes: Vec<u8>, message: impl Into<String>) -> Self {
        Self {
            original: target.to_owned(),
            format: detect_format(target),
            bytes: Some(bytes),
            status: ResourceStatus::Loaded,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResourceContext {
    #[cfg(feature = "fs")]
    /// Base directory for relative filesystem asset lookup.
    pub base_dir: Option<PathBuf>,
}

/// Render input passed to a [`RenderBackend`].
#[derive(Debug, Clone)]
pub struct RenderRequest {
    /// Source name derived from the markdown input.
    pub source_name: String,
    /// Requested page size.
    pub page_size: PageSize,
    /// Requested layout mode.
    pub layout_mode: LayoutMode,
    /// Requested theme configuration.
    pub theme: ThemeConfig,
}

/// Port for markdown parsing adapters used by the conversion pipeline.
pub trait MarkdownParser: Send + Sync {
    fn parse(&self, markdown: &str) -> Result<Document>;
}

/// Port for asset/resource resolution adapters.
///
/// Resolvers receive the raw href from Markdown and a [`ResourceContext`]. They can
/// return [`ResolvedAsset`] values with either [`ResourceStatus::Loaded`] or
/// [`ResourceStatus::Missing`]. Missing assets remain visible in output instead of
/// being silently dropped.
pub trait ResourceResolver: Send + Sync {
    fn resolve(&self, href: &str, ctx: &ResourceContext) -> Result<ResolvedAsset>;
}

/// Port for PDF rendering adapters.
///
/// Custom renderers can be injected through [`crate::Forge::with_renderer`].
pub trait RenderBackend: Send + Sync {
    fn render(&self, document: &Document, request: &RenderRequest) -> Result<Vec<u8>>;
}

fn detect_format(resource_path: &str) -> Option<&'static str> {
    let extension = std::path::Path::new(resource_path)
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();

    match extension.as_str() {
        "svg" => Some("svg"),
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpeg"),
        "gif" => Some("gif"),
        _ => None,
    }
}
