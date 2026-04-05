use typst::layout::PagedDocument;
use typst_as_lib::TypstEngine;
use typst_as_lib::typst_kit_options::TypstKitFontOptions;

use crate::adapters::render::plan::{ThemeProfile, page_spec, resolve_theme};
use crate::core::ports::{RenderBackend, RenderRequest, ResolvedAsset, ResourceStatus};
use crate::core::{Block, Document, ForgeError, Inline, LayoutMode, Result, TableAlignment};

#[derive(Debug, Default)]
pub struct TypstPdfRenderer;

impl RenderBackend for TypstPdfRenderer {
    fn render(&self, document: &Document, request: &RenderRequest) -> Result<Vec<u8>> {
        let page_size = page_spec(request.page_size);
        let theme = resolve_theme(&request.theme);
        let mut assets = TypstAssetLibrary::default();
        let source = build_typst_source(
            document,
            page_size.width_mm,
            page_size.height_mm,
            &theme,
            request.layout_mode,
            &mut assets,
        )?;

        if let Ok(path) = std::env::var("GLYPHWEAVEFORGE_DEBUG_TYPST_PATH") {
            let _ = std::fs::write(path, &source);
        }

        let engine = TypstEngine::builder()
            .main_file(source)
            .search_fonts_with(
                TypstKitFontOptions::default()
                    .include_system_fonts(true)
                    .include_embedded_fonts(true),
            )
            .with_static_file_resolver(assets.files())
            .build();
        let warned = engine.compile::<PagedDocument>();
        let document = warned.output.map_err(|error| ForgeError::TypstCompile {
            message: error.to_string(),
        })?;

        typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
            ForgeError::TypstExport {
                message: format!("{errors:?}"),
            }
        })
    }
}

#[derive(Debug, Default)]
struct TypstAssetLibrary {
    files: Vec<(String, Vec<u8>)>,
}

impl TypstAssetLibrary {
    fn register(&mut self, image: &ResolvedAsset) -> Result<String> {
        if image.status != ResourceStatus::Loaded {
            return Err(ForgeError::TypstAsset {
                target: image.original.clone(),
                message: image.message.clone(),
            });
        }

        let Some(bytes) = image.bytes.as_ref() else {
            return Err(ForgeError::TypstAsset {
                target: image.original.clone(),
                message: "resolved image bytes were empty".to_owned(),
            });
        };

        let extension = match image.format.unwrap_or("bin") {
            "jpeg" => "jpg",
            other => other,
        };
        let path = format!("assets/image-{}.{}", self.files.len(), extension);
        self.files.push((path.clone(), bytes.clone()));
        Ok(path)
    }

    fn files(&self) -> Vec<(&str, &[u8])> {
        self.files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
            .collect()
    }
}

fn build_typst_source(
    document: &Document,
    page_width_mm: f32,
    page_height_mm: f32,
    theme: &ThemeProfile,
    layout_mode: LayoutMode,
    assets: &mut TypstAssetLibrary,
) -> Result<String> {
    let mut source = String::new();
    let height_directive = match layout_mode {
        LayoutMode::SinglePage => "auto".to_owned(),
        LayoutMode::Paged => format!("{page_height_mm:.1}mm"),
    };
    source.push_str(&format!(
        "#set page(width: {page_width_mm:.1}mm, height: {height_directive}, margin: {margin:.1}mm)\n",
        margin = theme.margin_mm
    ));
    source.push_str(&format!(
        "#set text(font: \"{}\", size: {:.2}pt, fill: rgb(\"{}\"), lang: \"es\")\n",
        escape_typst_string(&theme.body_font),
        theme.body_font_size_pt,
        theme.body_color
    ));
    source.push_str("#set par(justify: true, leading: 0.68em)\n");
    source.push_str("#set raw(theme: auto)\n");
    source.push_str(&format!(
        "#show heading.where(level: 1): set text(font: \"{}\", size: {:.2}pt, weight: \"bold\", fill: rgb(\"{}\"))\n",
        escape_typst_string(&theme.heading_font),
        theme.body_font_size_pt * 2.15,
        theme.heading_color
    ));
    source.push_str("#show heading.where(level: 1): set block(above: 1.5em, below: 0.6em)\n");
    source.push_str(&format!(
        "#show heading.where(level: 2): set text(font: \"{}\", size: {:.2}pt, weight: \"bold\", fill: rgb(\"{}\"))\n",
        escape_typst_string(&theme.heading_font),
        theme.body_font_size_pt * 1.7,
        theme.heading_color
    ));
    source.push_str("#show heading.where(level: 2): set block(above: 1.1em, below: 0.6em)\n");
    source.push_str(&format!(
        "#show heading.where(level: 3): set text(font: \"{}\", size: {:.2}pt, weight: \"bold\", fill: rgb(\"{}\"))\n",
        escape_typst_string(&theme.heading_font),
        theme.body_font_size_pt * 1.4,
        theme.heading_color
    ));
    source.push_str("#show heading.where(level: 3): set block(above: 1.1em, below: 0.4em)\n");
    source.push_str(&format!(
        "#show strong: set text(weight: \"bold\", fill: rgb(\"{}\"))\n",
        theme.heading_color
    ));
    source.push_str("#show emph: set text(style: \"italic\")\n");
    source.push_str(&format!(
        "#show table.cell.where(y: 0): set text(weight: \"bold\", fill: rgb(\"{}\"))\n",
        theme.heading_color
    ));
    source.push_str(&format!(
        "#show link: set text(fill: rgb(\"{}\"))\n\n",
        theme.accent_color
    ));

    for block in &document.blocks {
        source.push_str(&render_block(block, theme, assets)?);
    }

    Ok(source)
}

fn render_block(
    block: &Block,
    theme: &ThemeProfile,
    assets: &mut TypstAssetLibrary,
) -> Result<String> {
    match block {
        Block::Heading { level, content } => render_heading(*level, content, theme, assets),
        Block::Paragraph { content } => {
            Ok(format!("{}\n\n", render_markup(content, theme, assets)?))
        }
        Block::List { ordered, items } => render_list(*ordered, items, theme, assets),
        Block::Table {
            alignments,
            headers,
            rows,
        } => render_table_block(alignments, headers, rows, theme, assets),
        Block::Quote { content } => Ok(format!(
            "#block(fill: rgb(\"{}\"), inset: 10pt, radius: 6pt, width: 100%)[{}]\n\n",
            theme.quote_background,
            render_markup(content, theme, assets)?
        )),
        Block::Code { language, code } => Ok(render_code_block(language.as_deref(), code, theme)),
        Block::Image { alt, asset } => render_image_block(alt, asset, assets),
        Block::MissingAsset {
            alt,
            target,
            message,
        } => Ok(render_notice_block(
            &format!("Missing image: {alt} ({target})"),
            message,
            theme,
        )),
        Block::Unsupported { kind, raw } => Ok(render_notice_block(
            &format!("Unsupported {kind}"),
            raw,
            theme,
        )),
        Block::ThematicBreak => Ok(format!(
            "#line(length: 100%, stroke: 0.6pt + rgb(\"{}\"))\n\n",
            theme.muted_color
        )),
    }
}

fn render_heading(
    level: u8,
    content: &[Inline],
    theme: &ThemeProfile,
    assets: &mut TypstAssetLibrary,
) -> Result<String> {
    let level = level.clamp(1, 6) as usize;
    Ok(format!(
        "{} {}\n\n",
        "=".repeat(level),
        render_markup(content, theme, assets)?
    ))
}

fn render_list(
    ordered: bool,
    items: &[Vec<Inline>],
    theme: &ThemeProfile,
    assets: &mut TypstAssetLibrary,
) -> Result<String> {
    let mut out = String::new();
    for item in items {
        let marker = if ordered { "+" } else { "-" };
        out.push_str(marker);
        out.push(' ');
        out.push_str(&render_markup(item, theme, assets)?);
        out.push('\n');
    }
    out.push('\n');
    Ok(out)
}

fn render_table_block(
    alignments: &[TableAlignment],
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    theme: &ThemeProfile,
    assets: &mut TypstAssetLibrary,
) -> Result<String> {
    let column_count = headers
        .len()
        .max(rows.iter().map(|row| row.len()).max().unwrap_or(0));
    if column_count == 0 {
        return Ok(String::new());
    }

    let mut out = format!(
        "#table(columns: {column_count}, stroke: 0.5pt + rgb(\"{}\"), inset: 8pt, fill: (_, y) => if y == 0 {{ rgb(\"{}\") }} else if calc.rem(y, 2) == 1 {{ rgb(\"{}\") }} else {{ white }},\n",
        theme.muted_color, theme.quote_background, theme.code_background,
    );

    if !headers.is_empty() {
        out.push_str("  table.header");
        for (index, header) in headers.iter().enumerate() {
            out.push_str(&render_table_cell(
                header,
                alignments
                    .get(index)
                    .copied()
                    .unwrap_or(TableAlignment::None),
                theme,
                assets,
            )?);
        }
        out.push_str(",\n");
    }

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            out.push_str("  ");
            out.push_str(&render_table_cell(
                cell,
                alignments
                    .get(index)
                    .copied()
                    .unwrap_or(TableAlignment::None),
                theme,
                assets,
            )?);
            out.push_str(",\n");
        }

        for index in row.len()..column_count {
            out.push_str("  ");
            out.push_str(&render_table_cell(
                &[],
                alignments
                    .get(index)
                    .copied()
                    .unwrap_or(TableAlignment::None),
                theme,
                assets,
            )?);
            out.push_str(",\n");
        }
    }

    out.push_str(")\n\n");
    Ok(out)
}

fn render_code_block(language: Option<&str>, code: &str, theme: &ThemeProfile) -> String {
    let label = escape_markup_text(language.unwrap_or("text"));
    format!(
        "#block(above: 0.5em, below: 0.8em, width: 100%)[#text(size: {:.2}pt, fill: rgb(\"{}\"))[{}]]\n#block(fill: rgb(\"{}\"), inset: 10pt, radius: 6pt, width: 100%)[```{}\n{}\n```]\n\n",
        (theme.code_font_size_pt - 0.5).max(7.5),
        theme.muted_color,
        label,
        theme.code_background,
        label,
        code,
    )
}

fn render_image_block(
    alt: &str,
    asset: &ResolvedAsset,
    assets: &mut TypstAssetLibrary,
) -> Result<String> {
    let path = assets.register(asset)?;
    Ok(format!(
        "#figure(image(\"{}\", width: 92%), caption: [{}])\n\n",
        escape_typst_string(&path),
        escape_markup_text(alt)
    ))
}

fn render_notice_block(title: &str, body: &str, theme: &ThemeProfile) -> String {
    format!(
        "#block(fill: rgb(\"{}\"), inset: 10pt, radius: 6pt, width: 100%)[*{}* {}]\n\n",
        theme.quote_background,
        escape_markup_text(title),
        escape_markup_text(body)
    )
}

fn render_table_cell(
    content: &[Inline],
    alignment: TableAlignment,
    theme: &ThemeProfile,
    assets: &mut TypstAssetLibrary,
) -> Result<String> {
    let body = render_markup(content, theme, assets)?;
    let cell = if body.is_empty() {
        "[]".to_owned()
    } else {
        format!("[{body}]")
    };

    Ok(match alignment {
        TableAlignment::None | TableAlignment::Left => cell,
        TableAlignment::Center => format!("[#align(center){cell}]"),
        TableAlignment::Right => format!("[#align(right){cell}]"),
    })
}

fn render_markup(
    inlines: &[Inline],
    theme: &ThemeProfile,
    assets: &mut TypstAssetLibrary,
) -> Result<String> {
    let mut out = String::new();
    for inline in inlines {
        out.push_str(&render_inline(inline, theme, assets)?);
    }
    Ok(out)
}

fn render_inline(
    inline: &Inline,
    theme: &ThemeProfile,
    assets: &mut TypstAssetLibrary,
) -> Result<String> {
    match inline {
        Inline::Text(text) => Ok(escape_markup_text(text)),
        Inline::Code(text) => Ok(format!(
            "#box(fill: rgb(\"{}\"), inset: (x: 0.22em, y: 0.08em), radius: 2pt)[`{}`]",
            theme.code_background,
            escape_code_markup(text)
        )),
        Inline::Emphasis(children) => Ok(format!("_{}_", render_markup(children, theme, assets)?)),
        Inline::Strong(children) => Ok(format!("*{}*", render_markup(children, theme, assets)?)),
        Inline::Link { label, target } => Ok(format!(
            "#link(\"{}\")[{}]",
            escape_typst_string(target),
            render_markup(label, theme, assets)?
        )),
        Inline::Image { alt, .. } => Ok(format!("[image: {}]", escape_markup_text(alt))),
        Inline::ResolvedImage { alt, asset } => {
            if asset.status == ResourceStatus::Loaded {
                let path = assets.register(asset)?;
                Ok(format!(
                    "#image(\"{}\", height: 1.2em)",
                    escape_typst_string(&path)
                ))
            } else {
                Ok(escape_markup_text(alt))
            }
        }
        Inline::SoftBreak => Ok(" ".to_owned()),
        Inline::HardBreak => Ok(" \\\n".to_owned()),
    }
}

fn escape_markup_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '#' | '[' | ']' | '*' | '_' | '`' | '$' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            '\n' | '\r' => escaped.push(' '),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn escape_code_markup(text: &str) -> String {
    text.replace('`', "\\`")
}

fn escape_typst_string(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => {}
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
