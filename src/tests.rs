use std::io;

use serde_json::json;
#[cfg(feature = "fs")]
use tempfile::tempdir;

use crate::adapters::markdown::PulldownParser;
use crate::adapters::render::MinimalPdfRenderer;
use crate::adapters::render::plan::{RenderCodeBlock, RenderElement, build_render_plan};
use crate::core::ports::{MarkdownParser, RenderBackend, RenderRequest};
use crate::core::ports::{ResolvedAsset, ResourceStatus};
use crate::core::{Block, Document, ForgeError, Inline};
#[cfg(feature = "fs")]
use crate::pipeline::output::derive_output_name;
use crate::{
    BuiltInTheme, Forge, LayoutMode, PageSize, PdfOutput, RenderBackendSelection, ThemeConfig,
};

fn pdf_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn parser_preserves_lists_quotes_links_and_emphasis() {
    let document = PulldownParser
        .parse("# Title\n\n- *one*\n- **two**\n\n> quoted [link](https://example.com)")
        .expect("parse should succeed");

    assert!(matches!(document.blocks[0], Block::Heading { .. }));
    assert!(matches!(
        document.blocks[1],
        Block::List { ordered: false, .. }
    ));
    assert!(matches!(document.blocks[2], Block::Quote { .. }));
}

#[cfg(feature = "fs")]
#[test]
fn output_names_remain_deterministic() {
    assert_eq!(
        derive_output_name(None, "guide").expect("derived name"),
        "guide.pdf"
    );
    assert_eq!(
        derive_output_name(Some("report"), "guide").expect("custom name"),
        "report.pdf"
    );
}

#[test]
fn convert_reports_missing_source_and_output() {
    let err = Forge::new()
        .to_memory()
        .convert()
        .expect_err("missing source");
    assert!(matches!(err, ForgeError::MissingSource));

    let err = Forge::new()
        .from_text("hello")
        .convert()
        .expect_err("missing output");
    assert!(matches!(err, ForgeError::MissingOutput));
}

#[test]
fn converts_text_to_memory_pdf() {
    let result = Forge::new()
        .from_text("# Hello\n\nWorld")
        .to_memory()
        .convert()
        .expect("conversion should succeed");

    let bytes = result.bytes.expect("bytes should exist");
    assert!(bytes.starts_with(b"%PDF-1.4"));
    assert!(pdf_text(&bytes).contains("Hello"));
}

#[cfg(feature = "fs")]
#[test]
fn custom_output_name_is_honored_for_directory_output() {
    let temp = tempdir().expect("tempdir should exist");
    let result = Forge::new()
        .from_text("named")
        .to_directory(temp.path())
        .with_output_file_name("final-report")
        .convert()
        .expect("directory output should succeed");

    let written = result.written_path.expect("written path should be present");
    assert_eq!(
        written.file_name().and_then(|v| v.to_str()),
        Some("final-report.pdf")
    );
}

#[test]
fn renderer_can_be_swapped_through_public_port() {
    struct StubRenderer;

    impl RenderBackend for StubRenderer {
        fn render(
            &self,
            _document: &crate::core::Document,
            _request: &RenderRequest,
        ) -> crate::Result<Vec<u8>> {
            Ok(b"stub-pdf".to_vec())
        }
    }

    let result = Forge::new()
        .from_text("hello")
        .to_memory()
        .with_renderer(StubRenderer)
        .convert()
        .expect("custom renderer should be used");

    assert_eq!(
        result,
        PdfOutput {
            bytes: Some(b"stub-pdf".to_vec()),
            #[cfg(feature = "fs")]
            written_path: None
        }
    );
}

#[test]
fn theme_page_size_and_layout_options_flow_into_pdf_text() {
    let result = Forge::new()
        .from_text("# Report")
        .to_memory()
        .with_theme_config(ThemeConfig {
            built_in: Some(BuiltInTheme::Engineering),
            custom_theme_json: Some(json!({"name": "custom-engineering"})),
        })
        .with_page_size(PageSize::Letter)
        .with_layout_mode(LayoutMode::SinglePage)
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("pdf bytes should exist"));
    assert!(text.contains("custom-engineering"));
    assert!(text.contains("215.9x279.4mm"));
    assert!(text.contains("SinglePage"));
}

#[test]
fn missing_asset_block_is_visible() {
    let result = Forge::new()
        .from_text("![Logo](missing.svg)")
        .to_memory()
        .convert()
        .expect("conversion should succeed with fallback");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("Missing image: Logo"));
}

#[test]
fn inline_images_use_custom_resolver() {
    let result = Forge::new()
        .from_text("before ![Logo](logo.png) after")
        .to_memory()
        .with_resource_resolver(|path| {
            if path == "logo.png" {
                Ok(vec![1, 2, 3])
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
            }
        })
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("Image: Logo | png | loaded through custom resource resolver"));
}

#[test]
fn default_mermaid_behavior_is_non_lossy_fallback() {
    let result = Forge::new()
        .from_text("```mermaid\ngraph TD\n```")
        .to_memory()
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("[unsupported:mermaid]"));
    assert!(text.contains("graph TD"));
}

#[test]
fn unsupported_markdown_becomes_visible_fallback() {
    let result = Forge::new()
        .from_text("| a | b |\n| - | - |\n| 1 | 2 |")
        .to_memory()
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("Unsupported table fallback"));
}

#[cfg(feature = "fs")]
#[test]
fn writes_pdf_to_directory_with_derived_name() {
    let temp = tempdir().expect("tempdir should exist");
    let markdown = temp.path().join("guide.md");
    std::fs::write(&markdown, "Directory output").expect("markdown file should be written");

    let result = Forge::new()
        .from_path(&markdown)
        .to_directory(temp.path())
        .convert()
        .expect("directory output should succeed");

    assert_eq!(
        result
            .written_path
            .and_then(|path| path.file_name().map(|v| v.to_owned()))
            .and_then(|v| v.to_str().map(ToOwned::to_owned)),
        Some("guide.pdf".to_owned())
    );
}

#[test]
fn minimal_renderer_accepts_document_port() {
    let document = crate::core::Document::new(vec![Block::Paragraph {
        content: vec![Inline::Text("hello".to_owned())],
    }]);
    let bytes = MinimalPdfRenderer
        .render(
            &document,
            &RenderRequest {
                source_name: "doc".to_owned(),
                page_size: PageSize::A4,
                layout_mode: LayoutMode::Paged,
                theme: ThemeConfig::default(),
            },
        )
        .expect("render should succeed");
    assert!(pdf_text(&bytes).contains("hello"));
}

#[test]
fn render_plan_emits_code_block_elements_with_language_and_lines() {
    let document = Document::new(vec![Block::Code {
        language: Some("rust".to_owned()),
        code: "fn main() {}\nprintln!(\"ok\");".to_owned(),
    }]);

    let plan = build_render_plan(
        &document,
        &RenderRequest {
            source_name: "code".to_owned(),
            page_size: PageSize::A4,
            layout_mode: LayoutMode::Paged,
            theme: ThemeConfig::default(),
        },
    )
    .expect("render plan should succeed");

    assert!(plan.elements.iter().any(|element| {
        matches!(
            element,
            RenderElement::CodeBlock(RenderCodeBlock {
                language,
                summary,
                lines,
                ..
            }) if language.as_deref() == Some("rust")
                && summary == "[code:rust]"
                && lines == &vec!["fn main() {}".to_owned(), "println!(\"ok\");".to_owned()]
        )
    }));
}

#[test]
fn render_plan_keeps_honest_fallback_markers_for_mermaid_and_math() {
    let document = Document::new(vec![
        Block::Code {
            language: Some("mermaid".to_owned()),
            code: "graph TD\nA-->B".to_owned(),
        },
        Block::Code {
            language: Some("math".to_owned()),
            code: "x^2 + 1".to_owned(),
        },
    ]);

    let plan = build_render_plan(
        &document,
        &RenderRequest {
            source_name: "fallbacks".to_owned(),
            page_size: PageSize::A4,
            layout_mode: LayoutMode::Paged,
            theme: ThemeConfig::default(),
        },
    )
    .expect("render plan should succeed");

    assert!(plan.elements.iter().any(|element| {
        matches!(
            element,
            RenderElement::CodeBlock(RenderCodeBlock {
                language,
                summary,
                lines,
                ..
            }) if language.as_deref() == Some("mermaid")
                && summary == "[unsupported:mermaid]"
                && lines == &vec!["graph TD".to_owned(), "A-->B".to_owned()]
        )
    }));
    assert!(plan.elements.iter().any(|element| {
        matches!(
            element,
            RenderElement::CodeBlock(RenderCodeBlock {
                language,
                summary,
                lines,
                ..
            }) if language.as_deref() == Some("math")
                && summary == "[unsupported:math]"
                && lines == &vec!["x^2 + 1".to_owned()]
        )
    }));
}

#[test]
fn render_plan_promotes_loaded_images_into_image_elements() {
    let document = Document::new(vec![Block::Image {
        alt: "Logo".to_owned(),
        asset: ResolvedAsset {
            original: "logo.png".to_owned(),
            format: Some("png"),
            bytes: Some(vec![1, 2, 3]),
            status: ResourceStatus::Loaded,
            message: "loaded through custom resource resolver".to_owned(),
            #[cfg(feature = "fs")]
            path: None,
        },
    }]);

    let plan = build_render_plan(
        &document,
        &RenderRequest {
            source_name: "image".to_owned(),
            page_size: PageSize::A4,
            layout_mode: LayoutMode::Paged,
            theme: ThemeConfig::default(),
        },
    )
    .expect("render plan should succeed");

    assert!(plan.elements.iter().any(|element| {
        matches!(
            element,
            RenderElement::Image(image)
                if image.alt == "Logo"
                    && image.format == "png"
                    && image.bytes == vec![1, 2, 3]
                    && image.message == "loaded through custom resource resolver"
        )
    }));
}

#[test]
fn explicit_minimal_backend_selection_keeps_text_visible() {
    let result = Forge::new()
        .from_text("# Hello")
        .to_memory()
        .with_backend(RenderBackendSelection::Minimal)
        .convert()
        .expect("minimal backend should render");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("Hello"));
}

#[test]
fn math_blocks_use_honest_visible_fallback_text() {
    let result = Forge::new()
        .from_text("```math\nx^2 + 1\n```")
        .to_memory()
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("[unsupported:math]"));
    assert!(text.contains("x^2 + 1"));
}

#[cfg(feature = "renderer-typst")]
#[test]
fn typst_backend_renders_pdf_bytes() {
    let result = Forge::new()
        .from_text("# Hello from Typst")
        .to_memory()
        .with_backend(RenderBackendSelection::Typst)
        .convert()
        .expect("typst backend should render");

    let bytes = result.bytes.expect("bytes should exist");
    assert!(bytes.starts_with(b"%PDF"));
    assert!(bytes.len() > 100);
}

#[test]
fn custom_page_size_must_be_positive() {
    let err = Forge::new()
        .from_text("invalid page size")
        .to_memory()
        .with_page_size(PageSize::Custom {
            width_mm: 0.0,
            height_mm: 100.0,
        })
        .convert()
        .expect_err("invalid page size should fail");

    assert!(matches!(
        err,
        ForgeError::InvalidConfiguration {
            field: "page_size",
            ..
        }
    ));
}

#[cfg(feature = "renderer-typst")]
#[test]
fn typst_backend_uses_memory_backed_images() {
    let result = Forge::new()
        .from_text("![Diagram](diagram.png)")
        .to_memory()
        .with_backend(RenderBackendSelection::Typst)
        .with_resource_resolver(|path| {
            if path == "diagram.png" {
                Ok(vec![
                    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49,
                    0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02,
                    0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44,
                    0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02,
                    0xFE, 0x0D, 0xEF, 0x46, 0xB8, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
                    0xAE, 0x42, 0x60, 0x82,
                ])
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
            }
        })
        .convert()
        .expect("typst backend should render memory-backed images");

    let bytes = result.bytes.expect("bytes should exist");
    assert!(bytes.starts_with(b"%PDF"));
    assert!(bytes.len() > 100);
}
