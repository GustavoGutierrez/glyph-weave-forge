use serde_json::Value;

use crate::core::ports::{RenderRequest, ResolvedAsset, ResourceStatus};
use crate::core::{
    Block, BuiltInTheme, Document, ForgeError, Inline, LayoutMode, PageSize, Result, ThemeConfig,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThemeProfile {
    pub(crate) name: String,
    pub(crate) body_font_size_pt: f32,
    pub(crate) code_font_size_pt: f32,
    pub(crate) heading_scale: f32,
    pub(crate) margin_mm: f32,
    pub(crate) body_font: String,
    pub(crate) heading_font: String,
    pub(crate) code_font: String,
    pub(crate) body_color: String,
    pub(crate) muted_color: String,
    pub(crate) heading_color: String,
    pub(crate) accent_color: String,
    pub(crate) code_background: String,
    pub(crate) quote_background: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PageSpec {
    pub(crate) width_mm: f32,
    pub(crate) height_mm: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RenderPlan {
    pub(crate) theme: ThemeProfile,
    pub(crate) page_size: PageSpec,
    pub(crate) elements: Vec<RenderElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RenderElement {
    Line(RenderLine),
    CodeBlock(RenderCodeBlock),
    Image(RenderImage),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RenderCodeBlock {
    pub(crate) language: Option<String>,
    pub(crate) summary: String,
    pub(crate) lines: Vec<String>,
    pub(crate) font_size_pt: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RenderPage {
    pub(crate) lines: Vec<RenderLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RenderLine {
    pub(crate) text: String,
    pub(crate) font_size_pt: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RenderImage {
    pub(crate) alt: String,
    pub(crate) original: String,
    pub(crate) format: &'static str,
    pub(crate) bytes: Vec<u8>,
    pub(crate) message: String,
}

pub(crate) fn build_render_plan(
    document: &Document,
    request: &RenderRequest,
) -> Result<RenderPlan> {
    validate_render_request(request)?;

    let theme = resolve_theme(&request.theme);
    let page_size = page_spec(request.page_size);
    let mut elements = Vec::new();
    elements.extend(document_to_elements(document, &theme));

    Ok(RenderPlan {
        theme,
        page_size,
        elements,
    })
}

pub(crate) fn render_lines(plan: &RenderPlan) -> Vec<RenderLine> {
    plan.elements
        .iter()
        .flat_map(|element| match element {
            RenderElement::Line(line) => vec![line.clone()],
            RenderElement::CodeBlock(block) => {
                let mut lines = vec![RenderLine {
                    text: block.summary.clone(),
                    font_size_pt: block.font_size_pt,
                }];
                lines.extend(block.lines.iter().map(|line| RenderLine {
                    text: format!("    {line}"),
                    font_size_pt: block.font_size_pt,
                }));
                lines.push(blank_line(plan.theme.body_font_size_pt));
                lines
            }
            RenderElement::Image(image) => vec![
                RenderLine {
                    text: format!(
                        "[Image: {} | {} | {}]",
                        image.alt, image.format, image.message
                    ),
                    font_size_pt: plan.theme.body_font_size_pt,
                },
                blank_line(plan.theme.body_font_size_pt),
            ],
        })
        .collect()
}

pub(crate) fn paginate(
    lines: &[RenderLine],
    page_size: PageSpec,
    layout_mode: LayoutMode,
    margin_mm: f32,
) -> Vec<RenderPage> {
    if layout_mode == LayoutMode::SinglePage {
        return vec![RenderPage {
            lines: lines.to_vec(),
        }];
    }
    let usable_height_mm = (page_size.height_mm - (margin_mm * 2.0)).max(20.0);
    let line_capacity = (usable_height_mm / 5.0).floor().max(1.0) as usize;
    lines
        .chunks(line_capacity)
        .map(|chunk| RenderPage {
            lines: chunk.to_vec(),
        })
        .collect()
}

pub(crate) fn page_spec(page_size: PageSize) -> PageSpec {
    match page_size {
        PageSize::A4 => PageSpec {
            width_mm: 210.0,
            height_mm: 297.0,
        },
        PageSize::Letter => PageSpec {
            width_mm: 215.9,
            height_mm: 279.4,
        },
        PageSize::Legal => PageSpec {
            width_mm: 215.9,
            height_mm: 355.6,
        },
        PageSize::Custom {
            width_mm,
            height_mm,
        } => PageSpec {
            width_mm,
            height_mm,
        },
    }
}

fn validate_render_request(request: &RenderRequest) -> Result<()> {
    if let PageSize::Custom {
        width_mm,
        height_mm,
    } = request.page_size
        && (width_mm <= 0.0 || height_mm <= 0.0)
    {
        return Err(ForgeError::InvalidConfiguration {
            field: "page_size",
            message: format!("custom page size must be positive, got {width_mm}x{height_mm}mm"),
        });
    }
    Ok(())
}

fn document_to_elements(document: &Document, theme: &ThemeProfile) -> Vec<RenderElement> {
    let mut elements = Vec::new();
    for block in &document.blocks {
        match block {
            Block::Heading { level, content } => {
                elements.push(RenderElement::Line(RenderLine {
                    text: inline_text(content),
                    font_size_pt: (theme.body_font_size_pt * theme.heading_scale)
                        - ((*level as f32 - 1.0) * 1.2),
                }));
                elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
            }
            Block::Paragraph { content } => {
                for line in inline_text(content).split('\n') {
                    elements.push(RenderElement::Line(RenderLine {
                        text: line.to_owned(),
                        font_size_pt: theme.body_font_size_pt,
                    }));
                }
                elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
            }
            Block::List { ordered, items } => {
                for (index, item) in items.iter().enumerate() {
                    let marker = if *ordered {
                        format!("{}. ", index + 1)
                    } else {
                        "- ".to_owned()
                    };
                    elements.push(RenderElement::Line(RenderLine {
                        text: format!("{marker}{}", inline_text(item)),
                        font_size_pt: theme.body_font_size_pt,
                    }));
                }
                elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
            }
            Block::Table { headers, rows, .. } => {
                render_table_block(&mut elements, headers, rows, theme);
            }
            Block::Quote { content } => {
                for line in inline_text(content).split('\n') {
                    elements.push(RenderElement::Line(RenderLine {
                        text: format!("> {line}"),
                        font_size_pt: theme.body_font_size_pt,
                    }));
                }
                elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
            }
            Block::Code { language, code } => {
                render_code_block(&mut elements, language.as_deref(), code, theme)
            }
            Block::Mermaid { source } => {
                elements.push(RenderElement::CodeBlock(RenderCodeBlock {
                    language: Some("mermaid".to_owned()),
                    summary: "[unsupported:mermaid]".to_owned(),
                    lines: source.lines().map(ToOwned::to_owned).collect(),
                    font_size_pt: theme.code_font_size_pt,
                }));
            }
            Block::Math { tex } => {
                elements.push(RenderElement::Line(RenderLine {
                    text: format!("[math] {tex}"),
                    font_size_pt: theme.body_font_size_pt,
                }));
                elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
            }
            Block::Image { alt, asset } => {
                if let Some(image) = render_image(alt, asset) {
                    elements.push(RenderElement::Image(image));
                    elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
                } else {
                    elements.push(RenderElement::Line(RenderLine {
                        text: image_summary(alt, asset),
                        font_size_pt: theme.body_font_size_pt,
                    }));
                    elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
                }
            }
            Block::MissingAsset {
                alt,
                target,
                message,
            } => {
                elements.push(RenderElement::Line(RenderLine {
                    text: format!("[Missing image: {alt} ({target}) | {message}]"),
                    font_size_pt: theme.body_font_size_pt,
                }));
                elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
            }
            Block::Unsupported { kind, raw } => {
                elements.push(RenderElement::Line(RenderLine {
                    text: format!("[Unsupported {kind} fallback] {raw}"),
                    font_size_pt: theme.body_font_size_pt,
                }));
                elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
            }
            Block::ThematicBreak => {
                elements.push(RenderElement::Line(RenderLine {
                    text: "----------------------------------------".to_owned(),
                    font_size_pt: theme.body_font_size_pt,
                }));
                elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
            }
        }
    }
    elements
}

fn render_code_block(
    elements: &mut Vec<RenderElement>,
    language: Option<&str>,
    code: &str,
    theme: &ThemeProfile,
) {
    let (language, summary) = match language {
        Some("math") => (Some("math".to_owned()), "[unsupported:math]".to_owned()),
        Some(language) => (Some(language.to_owned()), format!("[code:{language}]")),
        None => (None, "[code:text]".to_owned()),
    };

    elements.push(RenderElement::CodeBlock(RenderCodeBlock {
        language,
        summary,
        lines: code.lines().map(ToOwned::to_owned).collect(),
        font_size_pt: theme.code_font_size_pt,
    }));
}

fn render_table_block(
    elements: &mut Vec<RenderElement>,
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    theme: &ThemeProfile,
) {
    if !headers.is_empty() {
        elements.push(RenderElement::Line(RenderLine {
            text: join_table_row(headers),
            font_size_pt: theme.body_font_size_pt,
        }));
        elements.push(RenderElement::Line(RenderLine {
            text: headers
                .iter()
                .map(|cell| "-".repeat(inline_text(cell).len().max(3)))
                .collect::<Vec<_>>()
                .join(" | "),
            font_size_pt: theme.code_font_size_pt,
        }));
    }

    for row in rows {
        elements.push(RenderElement::Line(RenderLine {
            text: join_table_row(row),
            font_size_pt: theme.body_font_size_pt,
        }));
    }

    elements.push(RenderElement::Line(blank_line(theme.body_font_size_pt)));
}

fn join_table_row(row: &[Vec<Inline>]) -> String {
    row.iter()
        .map(|cell| inline_text(cell))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn render_image(alt: &str, asset: &ResolvedAsset) -> Option<RenderImage> {
    if asset.status != ResourceStatus::Loaded {
        return None;
    }
    let format = asset.format?;
    let bytes = asset.bytes.clone()?;
    Some(RenderImage {
        alt: alt.to_owned(),
        original: asset.original.clone(),
        format,
        bytes,
        message: asset.message.clone(),
    })
}

fn inline_text(content: &[Inline]) -> String {
    let mut out = String::new();
    for inline in content {
        match inline {
            Inline::Text(text) => out.push_str(text),
            Inline::Code(text) => {
                out.push('`');
                out.push_str(text);
                out.push('`');
            }
            Inline::Math(text) => {
                out.push_str("[math: ");
                out.push_str(text);
                out.push(']');
            }
            Inline::Emphasis(children) => {
                out.push('*');
                out.push_str(&inline_text(children));
                out.push('*');
            }
            Inline::Strong(children) => {
                out.push_str("**");
                out.push_str(&inline_text(children));
                out.push_str("**");
            }
            Inline::Link { label, target } => {
                out.push('[');
                out.push_str(&inline_text(label));
                out.push_str("](");
                out.push_str(target);
                out.push(')');
            }
            Inline::Image { alt, target } => {
                out.push_str(&format!("![{alt}]({target})"));
            }
            Inline::ResolvedImage { alt, asset } => out.push_str(&image_summary(alt, asset)),
            Inline::SoftBreak | Inline::HardBreak => out.push('\n'),
        }
    }
    out
}

fn image_summary(alt: &str, resource: &ResolvedAsset) -> String {
    match resource.status {
        ResourceStatus::Loaded => {
            let format = resource.format.unwrap_or("binary");
            format!("[Image: {alt} | {format} | {}]", resource.message)
        }
        ResourceStatus::Missing => format!("[Missing image: {alt} | {}]", resource.message),
    }
}

fn blank_line(font_size_pt: f32) -> RenderLine {
    RenderLine {
        text: String::new(),
        font_size_pt,
    }
}

pub(crate) fn resolve_theme(theme: &ThemeConfig) -> ThemeProfile {
    let built_in = theme.built_in.unwrap_or(BuiltInTheme::Professional);
    let mut profile = match built_in {
        BuiltInTheme::Invoice => ThemeProfile {
            name: "invoice".to_owned(),
            body_font_size_pt: 11.0,
            code_font_size_pt: 9.5,
            heading_scale: 1.3,
            margin_mm: 25.4,
            body_font: "DejaVu Sans".to_owned(),
            heading_font: "DejaVu Sans".to_owned(),
            code_font: "DejaVu Sans Mono".to_owned(),
            body_color: "000000".to_owned(),
            muted_color: "666666".to_owned(),
            heading_color: "1E3A8A".to_owned(),
            accent_color: "1E3A8A".to_owned(),
            code_background: "F8FAFC".to_owned(),
            quote_background: "F1F5F9".to_owned(),
        },
        BuiltInTheme::ScientificArticle => ThemeProfile {
            name: "scientific-article".to_owned(),
            body_font_size_pt: 10.0,
            code_font_size_pt: 8.5,
            heading_scale: 1.9,
            margin_mm: 20.0,
            body_font: "DejaVu Serif".to_owned(),
            heading_font: "DejaVu Serif".to_owned(),
            code_font: "DejaVu Sans Mono".to_owned(),
            body_color: "000000".to_owned(),
            muted_color: "444444".to_owned(),
            heading_color: "000000".to_owned(),
            accent_color: "222222".to_owned(),
            code_background: "F5F5F5".to_owned(),
            quote_background: "FAFAFA".to_owned(),
        },
        BuiltInTheme::Professional => ThemeProfile {
            name: "professional".to_owned(),
            body_font_size_pt: 12.0,
            code_font_size_pt: 9.5,
            heading_scale: 1.55,
            margin_mm: 25.4,
            body_font: "DejaVu Sans".to_owned(),
            heading_font: "DejaVu Sans".to_owned(),
            code_font: "DejaVu Sans Mono".to_owned(),
            body_color: "111827".to_owned(),
            muted_color: "6B7280".to_owned(),
            heading_color: "1E3A8A".to_owned(),
            accent_color: "2563EB".to_owned(),
            code_background: "F7FAFC".to_owned(),
            quote_background: "EDF2F7".to_owned(),
        },
        BuiltInTheme::Engineering => ThemeProfile {
            name: "engineering".to_owned(),
            body_font_size_pt: 10.5,
            code_font_size_pt: 9.0,
            heading_scale: 1.35,
            margin_mm: 20.0,
            body_font: "DejaVu Serif".to_owned(),
            heading_font: "DejaVu Serif".to_owned(),
            code_font: "DejaVu Sans Mono".to_owned(),
            body_color: "000000".to_owned(),
            muted_color: "4B5563".to_owned(),
            heading_color: "111827".to_owned(),
            accent_color: "374151".to_owned(),
            code_background: "F3F4F6".to_owned(),
            quote_background: "F9FAFB".to_owned(),
        },
        BuiltInTheme::Informational => ThemeProfile {
            name: "informational".to_owned(),
            body_font_size_pt: 13.0,
            code_font_size_pt: 10.5,
            heading_scale: 1.55,
            margin_mm: 25.4,
            body_font: "DejaVu Sans".to_owned(),
            heading_font: "DejaVu Sans".to_owned(),
            code_font: "DejaVu Sans Mono".to_owned(),
            body_color: "1F2937".to_owned(),
            muted_color: "64748B".to_owned(),
            heading_color: "1D4ED8".to_owned(),
            accent_color: "0EA5E9".to_owned(),
            code_background: "EFF6FF".to_owned(),
            quote_background: "F0F9FF".to_owned(),
        },
    };
    if let Some(custom) = theme.custom_theme_json.as_ref() {
        apply_json_overrides(&mut profile, custom);
    }
    profile
}

fn apply_json_overrides(profile: &mut ThemeProfile, json: &Value) {
    if let Some(name) = json.get("name").and_then(Value::as_str) {
        profile.name = name.to_owned();
    }
    if let Some(size) = json.get("body_font_size_pt").and_then(Value::as_f64) {
        profile.body_font_size_pt = size as f32;
    }
    if let Some(size) = json.get("code_font_size_pt").and_then(Value::as_f64) {
        profile.code_font_size_pt = size as f32;
    }
    if let Some(scale) = json.get("heading_scale").and_then(Value::as_f64) {
        profile.heading_scale = scale as f32;
    }
    if let Some(margin) = json.get("margin_mm").and_then(Value::as_f64) {
        profile.margin_mm = margin as f32;
    }
    if let Some(color) = json.get("body_color").and_then(Value::as_str) {
        profile.body_color = color.to_owned();
    }
    if let Some(color) = json.get("muted_color").and_then(Value::as_str) {
        profile.muted_color = color.to_owned();
    }
    if let Some(color) = json.get("heading_color").and_then(Value::as_str) {
        profile.heading_color = color.to_owned();
    }
    if let Some(color) = json.get("accent_color").and_then(Value::as_str) {
        profile.accent_color = color.to_owned();
    }
    if let Some(color) = json.get("code_background").and_then(Value::as_str) {
        profile.code_background = color.to_owned();
    }
    if let Some(color) = json.get("quote_background").and_then(Value::as_str) {
        profile.quote_background = color.to_owned();
    }
}
