#[cfg(feature = "fs")]
use std::path::PathBuf;

use crate::core::{Document, Result};
use crate::core::{LayoutMode, PageSize, ThemeConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceStatus {
    Loaded,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAsset {
    pub original: String,
    pub format: Option<&'static str>,
    pub bytes: Option<Vec<u8>>,
    pub status: ResourceStatus,
    pub message: String,
    #[cfg(feature = "fs")]
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
    pub base_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub source_name: String,
    pub page_size: PageSize,
    pub layout_mode: LayoutMode,
    pub theme: ThemeConfig,
}

pub trait MarkdownParser: Send + Sync {
    fn parse(&self, markdown: &str) -> Result<Document>;
}

pub trait ResourceResolver: Send + Sync {
    fn resolve(&self, href: &str, ctx: &ResourceContext) -> Result<ResolvedAsset>;
}

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
