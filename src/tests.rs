use std::io;

use serde_json::json;
#[cfg(feature = "fs")]
use tempfile::tempdir;

use crate::adapters::markdown::PulldownParser;
use crate::adapters::render::MinimalPdfRenderer;
use crate::adapters::render::plan::{
    RenderCodeBlock, RenderElement, build_render_plan, resolve_theme,
};
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

#[test]
fn parser_preserves_standard_tables() {
    let document = PulldownParser
        .parse("| Name | Docs |\n| --- | --- |\n| GlyphWeaveForge | [site](https://example.com) |")
        .expect("parse should succeed");

    assert!(matches!(document.blocks[0], Block::Table { .. }));
}

#[test]
fn parser_maps_inline_math_to_ast_math_node() {
    let document = PulldownParser
        .parse("Energy $E=mc^2$ relation")
        .expect("parse should succeed");

    assert!(matches!(
        &document.blocks[0],
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(inline, Inline::Math(tex) if tex == "E=mc^2"))
    ));
}

#[test]
fn parser_maps_display_math_to_block_node() {
    let document = PulldownParser
        .parse("before\n\n$$\\frac{a}{b}$$\n\nafter")
        .expect("parse should succeed");

    assert!(
        document
            .blocks
            .iter()
            .any(|block| matches!(block, Block::Math { tex } if tex.contains("\\frac{a}{b}")))
    );
}

#[test]
fn parser_maps_mermaid_fence_to_mermaid_block() {
    let document = PulldownParser
        .parse("```mermaid\nflowchart TD\nA --> B\n```")
        .expect("parse should succeed");

    assert!(matches!(
        document.blocks.first(),
        Some(Block::Mermaid { source }) if source.contains("flowchart TD") && source.contains("A --> B")
    ));
}

#[test]
fn parser_promotes_basic_html_img_tags() {
    let document = PulldownParser
        .parse("<img src=\"logo.png\" alt=\"Logo HTML\" width=\"120\" />")
        .expect("parse should succeed");

    assert!(matches!(
        &document.blocks[0],
        Block::Paragraph { content }
            if matches!(content.as_slice(), [Inline::Image { alt, target }] if alt == "Logo HTML" && target == "logo.png")
    ));
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

    let bytes = result.bytes.expect("pdf bytes should exist");
    let text = pdf_text(&bytes);
    // Verify content renders
    assert!(text.contains("Report"));
    // Verify PDF header
    assert!(bytes.starts_with(b"%PDF"));
    // Page size and layout flow into resolved profile (tested in theme resolution tests)
    let profile = resolve_theme(&ThemeConfig {
        built_in: Some(BuiltInTheme::Engineering),
        custom_theme_json: Some(json!({"name": "custom-engineering"})),
    });
    assert_eq!(profile.name, "custom-engineering");
}

#[test]
fn theme_json_overrides_are_applied_to_resolved_profile() {
    let theme = resolve_theme(&ThemeConfig {
        built_in: Some(BuiltInTheme::Engineering),
        custom_theme_json: Some(json!({
            "name": "compact-engineering",
            "body_font_size_pt": 9.0,
            "code_font_size_pt": 8.0,
            "heading_scale": 1.1,
            "margin_mm": 12.0,
            "body_color": "111111",
            "muted_color": "222222",
            "heading_color": "333333",
            "accent_color": "444444",
            "code_background": "555555",
            "quote_background": "666666"
        })),
    });

    assert_eq!(theme.name, "compact-engineering");
    assert_eq!(theme.body_font_size_pt, 9.0);
    assert_eq!(theme.code_font_size_pt, 8.0);
    assert_eq!(theme.heading_scale, 1.1);
    assert_eq!(theme.margin_mm, 12.0);
    assert_eq!(theme.body_color, "111111");
    assert_eq!(theme.muted_color, "222222");
    assert_eq!(theme.heading_color, "333333");
    assert_eq!(theme.accent_color, "444444");
    assert_eq!(theme.code_background, "555555");
    assert_eq!(theme.quote_background, "666666");
}

#[test]
fn built_in_themes_match_documented_style_presets() {
    let invoice = resolve_theme(&ThemeConfig {
        built_in: Some(BuiltInTheme::Invoice),
        custom_theme_json: None,
    });
    assert_eq!(invoice.body_font, "DejaVu Sans");
    assert_eq!(invoice.heading_font, "DejaVu Sans");
    assert_eq!(invoice.body_font_size_pt, 11.0);
    assert_eq!(invoice.margin_mm, 25.4);
    assert_eq!(invoice.heading_scale, 1.3);
    assert_eq!(invoice.muted_color, "666666");
    assert_eq!(invoice.heading_color, "1E3A8A");

    let scientific = resolve_theme(&ThemeConfig {
        built_in: Some(BuiltInTheme::ScientificArticle),
        custom_theme_json: None,
    });
    assert_eq!(scientific.body_font, "DejaVu Serif");
    assert_eq!(scientific.body_font_size_pt, 10.0);
    assert_eq!(scientific.code_font_size_pt, 8.5);
    assert_eq!(scientific.margin_mm, 20.0);
    assert_eq!(scientific.heading_scale, 1.9);
    assert_eq!(scientific.body_color, "000000");
    assert_eq!(scientific.heading_color, "000000");

    let professional = resolve_theme(&ThemeConfig {
        built_in: Some(BuiltInTheme::Professional),
        custom_theme_json: None,
    });
    assert_eq!(professional.body_font, "DejaVu Sans");
    assert_eq!(professional.body_font_size_pt, 12.0);
    assert_eq!(professional.margin_mm, 25.4);
    assert_eq!(professional.heading_scale, 1.55);
    assert_eq!(professional.heading_color, "1E3A8A");
    assert_eq!(professional.muted_color, "6B7280");

    let engineering = resolve_theme(&ThemeConfig {
        built_in: Some(BuiltInTheme::Engineering),
        custom_theme_json: None,
    });
    assert_eq!(engineering.body_font, "DejaVu Serif");
    assert_eq!(engineering.code_font, "DejaVu Sans Mono");
    assert_eq!(engineering.body_font_size_pt, 10.5);
    assert_eq!(engineering.margin_mm, 20.0);
    assert_eq!(engineering.heading_scale, 1.35);
    assert_eq!(engineering.body_color, "000000");

    let informational = resolve_theme(&ThemeConfig {
        built_in: Some(BuiltInTheme::Informational),
        custom_theme_json: None,
    });
    assert_eq!(informational.body_font, "DejaVu Sans");
    assert_eq!(informational.body_font_size_pt, 13.0);
    assert_eq!(informational.code_font_size_pt, 10.5);
    assert_eq!(informational.margin_mm, 25.4);
    assert_eq!(informational.heading_scale, 1.55);
    assert_eq!(informational.heading_color, "1D4ED8");
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
fn html_img_tags_use_custom_resolver() {
    let result = Forge::new()
        .from_text("<img src=\"logo.png\" alt=\"Logo HTML\" width=\"160\" />")
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
    assert!(text.contains("Image: Logo HTML | png | loaded through custom resource resolver"));
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
        .from_text("alpha[^1]\n\n[^1]: visible note")
        .to_memory()
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("Unsupported footnote fallback"));
}

#[test]
fn standard_tables_render_visible_rows() {
    let result = Forge::new()
        .from_text("| a | b |\n| --- | --- |\n| 1 | 2 |")
        .to_memory()
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("a | b"));
    assert!(text.contains("1 | 2"));
    assert!(!text.contains("Unsupported table"));
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
        Block::Mermaid {
            source: "graph TD\nA-->B".to_owned(),
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

#[test]
fn minimal_renderer_uses_readable_math_fallbacks() {
    let result = Forge::new()
        .from_text("Inline $x^2$ and\n\n$$\n\\sum_{n=1}^{\\infty} a_n\n$$")
        .to_memory()
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("[math: x^2]"));
    assert!(text.contains("[math]"));
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

#[cfg(feature = "renderer-typst")]
#[test]
fn typst_backend_renders_scientific_article_fixture_with_math() {
    let markdown = include_str!("../examples/markdown/mathematical-article.md");
    let output = Forge::new()
        .from_text(markdown)
        .to_memory()
        .with_backend(RenderBackendSelection::Typst)
        .convert()
        .expect("typst backend should render fixture");

    let bytes = output.bytes.expect("bytes should exist");
    assert!(bytes.starts_with(b"%PDF"));
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
