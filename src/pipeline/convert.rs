use std::borrow::Cow;
#[cfg(feature = "fs")]
use std::path::{Path, PathBuf};

use crate::adapters::markdown::PulldownParser;
use crate::api::{ConvertOptions, MarkdownSource, PdfOutput};
use crate::composition::default_resource_resolver;
use crate::core::ports::{
    MarkdownParser, RenderRequest, ResourceContext, ResourceResolver, ResourceStatus,
};
use crate::core::{Block, Document, ForgeError, Inline, Result};
use crate::math::preprocess_math_blocks;
use crate::pipeline::output::write_output;

#[derive(Debug, Clone)]
struct NormalizedInput {
    markdown: String,
    #[cfg(feature = "fs")]
    base_dir: Option<PathBuf>,
    source_name: String,
}

pub fn convert(options: &ConvertOptions<'_>) -> Result<PdfOutput> {
    let source = options.source.as_ref().ok_or(ForgeError::MissingSource)?;
    let output = options.output.as_ref().ok_or(ForgeError::MissingOutput)?;
    let normalized = normalize_source(source)?;
    let parser = PulldownParser;
    let mut document = parser.parse(&normalized.markdown)?;
    let resolver = if let Some(resolver) = options.resource_resolver.as_deref() {
        Some(resolver)
    } else {
        default_resource_resolver()
    };
    #[cfg(feature = "fs")]
    let resource_context = ResourceContext {
        base_dir: normalized.base_dir.clone(),
    };
    #[cfg(not(feature = "fs"))]
    let resource_context = ResourceContext {};
    resolve_document_resources(&mut document, resolver, &resource_context)?;
    let renderer = options
        .renderer
        .as_deref()
        .ok_or_else(|| ForgeError::Render {
            message: "no render backend configured".to_owned(),
        })?;
    let pdf_bytes = renderer.render(
        &document,
        &RenderRequest {
            source_name: normalized.source_name.clone(),
            page_size: options.page_size,
            layout_mode: options.layout_mode,
            theme: options.theme.clone(),
        },
    )?;
    write_output(
        output,
        options.output_file_name,
        &normalized.source_name,
        pdf_bytes,
    )
}

fn normalize_source(source: &MarkdownSource<'_>) -> Result<NormalizedInput> {
    match source {
        #[cfg(feature = "fs")]
        MarkdownSource::Path(path) => normalize_path(path),
        MarkdownSource::Text(text) => Ok(normalize_text(Cow::Borrowed(text), "document")),
        MarkdownSource::Bytes(bytes) => {
            let text = std::str::from_utf8(bytes)?;
            Ok(normalize_text(Cow::Borrowed(text), "document"))
        }
    }
}

#[cfg(feature = "fs")]
fn normalize_path(path: &Path) -> Result<NormalizedInput> {
    let raw = std::fs::read_to_string(path).map_err(|source| ForgeError::InputRead {
        path: path.to_path_buf(),
        source,
    })?;
    let base_dir = path.parent().map(Path::to_path_buf);
    let source_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("document")
        .to_owned();
    Ok(normalize_text(Cow::Owned(raw), &source_name).with_base_dir(base_dir))
}

fn normalize_text(markdown: Cow<'_, str>, source_name: &str) -> NormalizedInput {
    let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let preprocessed = preprocess_math_blocks(&normalized);
    NormalizedInput {
        markdown: preprocessed,
        source_name: source_name.to_owned(),
        #[cfg(feature = "fs")]
        base_dir: None,
    }
}

#[cfg(feature = "fs")]
impl NormalizedInput {
    fn with_base_dir(mut self, base_dir: Option<PathBuf>) -> Self {
        self.base_dir = base_dir;
        self
    }
}

fn resolve_document_resources(
    document: &mut Document,
    resolver: Option<&dyn ResourceResolver>,
    ctx: &ResourceContext,
) -> Result<()> {
    for block in &mut document.blocks {
        match block {
            Block::Paragraph { content } => {
                if let [Inline::Image { alt, target }] = content.as_slice() {
                    let asset = resolve_asset(target, resolver, ctx)?;
                    *block = if asset.status == ResourceStatus::Loaded {
                        Block::Image {
                            alt: alt.clone(),
                            asset,
                        }
                    } else {
                        Block::MissingAsset {
                            alt: alt.clone(),
                            target: target.clone(),
                            message: asset.message,
                        }
                    };
                } else {
                    for inline in content {
                        if let Inline::Image { alt, target } = inline {
                            let asset = resolve_asset(target, resolver, ctx)?;
                            *inline = Inline::ResolvedImage {
                                alt: alt.clone(),
                                asset,
                            };
                        }
                    }
                }
            }
            Block::Quote { .. } | Block::Heading { .. } | Block::List { .. } => {
                resolve_nested_block(block, resolver, ctx)?
            }
            _ => {}
        }
    }
    Ok(())
}

fn resolve_nested_block(
    block: &mut Block,
    resolver: Option<&dyn ResourceResolver>,
    ctx: &ResourceContext,
) -> Result<()> {
    match block {
        Block::Heading { content, .. } | Block::Quote { content } => {
            resolve_inlines(content, resolver, ctx)
        }
        Block::List { items, .. } => {
            for item in items {
                resolve_inlines(item, resolver, ctx)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn resolve_inlines(
    content: &mut Vec<Inline>,
    resolver: Option<&dyn ResourceResolver>,
    ctx: &ResourceContext,
) -> Result<()> {
    for inline in content {
        match inline {
            Inline::Image { alt, target } => {
                let asset = resolve_asset(target, resolver, ctx)?;
                *inline = Inline::ResolvedImage {
                    alt: alt.clone(),
                    asset,
                };
            }
            Inline::Emphasis(children) | Inline::Strong(children) => {
                resolve_inlines(children, resolver, ctx)?;
            }
            Inline::Link { label, .. } => resolve_inlines(label, resolver, ctx)?,
            _ => {}
        }
    }
    Ok(())
}

fn resolve_asset(
    target: &str,
    resolver: Option<&dyn ResourceResolver>,
    ctx: &ResourceContext,
) -> Result<crate::core::ports::ResolvedAsset> {
    match resolver {
        Some(resolver) => resolver.resolve(target, ctx),
        None => Ok(crate::core::ports::ResolvedAsset::missing(
            target,
            "resource resolver unavailable for this source",
        )),
    }
}
