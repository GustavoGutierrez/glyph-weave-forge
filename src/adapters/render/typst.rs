#[cfg(feature = "mermaid")]
use std::collections::HashMap;

use typst::layout::PagedDocument;
use typst_as_lib::TypstEngine;
use typst_as_lib::typst_kit_options::TypstKitFontOptions;

use crate::adapters::render::plan::{ThemeProfile, page_spec, resolve_theme};
use crate::core::ports::{RenderBackend, RenderRequest, ResolvedAsset, ResourceStatus};
use crate::core::{Block, Document, ForgeError, Inline, LayoutMode, Result, TableAlignment};
use crate::math::latex_to_typst_math;

#[derive(Debug, Default)]
pub struct TypstPdfRenderer;

impl RenderBackend for TypstPdfRenderer {
    fn render(&self, document: &Document, request: &RenderRequest) -> Result<Vec<u8>> {
        let page_size = page_spec(request.page_size);
        let theme = resolve_theme(&request.theme);
        let mut assets = TypstAssetLibrary::default();
        #[cfg(feature = "mermaid")]
        let mut mermaid = MermaidRenderContext::new(RustMermaidRunner);
        let source = build_typst_source(
            document,
            page_size.width_mm,
            page_size.height_mm,
            &theme,
            request.layout_mode,
            &mut assets,
            #[cfg(feature = "mermaid")]
            &mut mermaid,
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

    fn register_bytes(&mut self, extension: &str, bytes: Vec<u8>, prefix: &str) -> String {
        let path = format!("assets/{prefix}-{}.{}", self.files.len(), extension);
        self.files.push((path.clone(), bytes));
        path
    }
}

fn build_typst_source(
    document: &Document,
    page_width_mm: f32,
    page_height_mm: f32,
    theme: &ThemeProfile,
    layout_mode: LayoutMode,
    assets: &mut TypstAssetLibrary,
    #[cfg(feature = "mermaid")] mermaid: &mut MermaidRenderContext<RustMermaidRunner>,
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
    let h1_size = heading_size_pt(theme, 1);
    let h2_size = heading_size_pt(theme, 2);
    let h3_size = heading_size_pt(theme, 3);
    source.push_str(&format!(
        "#show heading.where(level: 1): set text(font: \"{}\", size: {:.2}pt, weight: \"bold\", fill: rgb(\"{}\"))\n",
        escape_typst_string(&theme.heading_font),
        h1_size,
        theme.heading_color
    ));
    source.push_str("#show heading.where(level: 1): set block(above: 1.5em, below: 0.6em)\n");
    source.push_str(&format!(
        "#show heading.where(level: 2): set text(font: \"{}\", size: {:.2}pt, weight: \"bold\", fill: rgb(\"{}\"))\n",
        escape_typst_string(&theme.heading_font),
        h2_size,
        theme.heading_color
    ));
    source.push_str("#show heading.where(level: 2): set block(above: 1.1em, below: 0.6em)\n");
    source.push_str(&format!(
        "#show heading.where(level: 3): set text(font: \"{}\", size: {:.2}pt, weight: \"bold\", fill: rgb(\"{}\"))\n",
        escape_typst_string(&theme.heading_font),
        h3_size,
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
        source.push_str(&render_block(
            block,
            theme,
            assets,
            #[cfg(feature = "mermaid")]
            mermaid,
        )?);
    }

    Ok(source)
}

fn heading_size_pt(theme: &ThemeProfile, level: u8) -> f32 {
    let step = 0.25f32;
    let multiplier = match level {
        1 => theme.heading_scale,
        2 => theme.heading_scale - step,
        _ => theme.heading_scale - (step * 2.0),
    }
    .max(1.0);

    theme.body_font_size_pt * multiplier
}

fn render_block(
    block: &Block,
    theme: &ThemeProfile,
    assets: &mut TypstAssetLibrary,
    #[cfg(feature = "mermaid")] mermaid: &mut MermaidRenderContext<RustMermaidRunner>,
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
        Block::Mermaid { source } => render_mermaid_block(
            source,
            theme,
            assets,
            #[cfg(feature = "mermaid")]
            mermaid,
        ),
        Block::Math { tex } => match latex_to_typst_math(tex) {
            Ok(converted) => Ok(format!("$ {converted} $\n\n")),
            Err(error) => Ok(render_notice_block(
                "Math conversion error",
                &format!("{error}; original: {tex}"),
                theme,
            )),
        },
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

fn render_mermaid_block(
    source: &str,
    theme: &ThemeProfile,
    assets: &mut TypstAssetLibrary,
    #[cfg(feature = "mermaid")] mermaid: &mut MermaidRenderContext<RustMermaidRunner>,
) -> Result<String> {
    #[cfg(feature = "mermaid")]
    {
        match mermaid.render(source) {
            MermaidRenderOutcome::RenderedSvg(svg) => {
                let path = assets.register_bytes("svg", svg.clone(), "mermaid");
                Ok(format!(
                    "#figure(image(\"{}\", width: 95%), caption: [Mermaid diagram])\n\n",
                    escape_typst_string(&path)
                ))
            }
            MermaidRenderOutcome::Fallback(message) => {
                Ok(render_notice_block(
                    "Unsupported mermaid",
                    &format!("[unsupported:mermaid] {message}\\n{source}"),
                    theme,
                ))
            }
        }
    }

    #[cfg(not(feature = "mermaid"))]
    {
        Ok(render_notice_block(
            "Unsupported mermaid",
            &format!("[unsupported:mermaid] {source}"),
            theme,
        ))
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
        Inline::Math(tex) => match latex_to_typst_math(tex) {
            Ok(converted) => Ok(format!("${converted}$")),
            Err(error) => Ok(format!(
                "#box(fill: rgb(\"{}\"), inset: 0.2em, radius: 3pt)[*Math conversion error:* {}]",
                theme.quote_background,
                escape_markup_text(&error.to_string())
            )),
        },
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

#[cfg(feature = "mermaid")]
#[derive(Debug, Clone)]
enum MermaidRenderOutcome {
    RenderedSvg(Vec<u8>),
    Fallback(String),
}

#[cfg(feature = "mermaid")]
trait MermaidRunner {
    fn render(&self, source: &str) -> Result<Vec<u8>>;
}

#[cfg(feature = "mermaid")]
#[derive(Debug)]
struct RustMermaidRunner;

#[cfg(feature = "mermaid")]
impl MermaidRunner for RustMermaidRunner {
    fn render(&self, source: &str) -> Result<Vec<u8>> {
        render_mermaid_svg(source)
            .map(|svg| svg.into_bytes())
            .map_err(|error| ForgeError::Render { message: error })
    }
}

#[cfg(feature = "mermaid")]
struct MermaidRenderContext<R: MermaidRunner> {
    runner: R,
    cache: HashMap<u64, MermaidRenderOutcome>,
}

#[cfg(feature = "mermaid")]
impl<R: MermaidRunner> MermaidRenderContext<R> {
    fn new(runner: R) -> Self {
        Self {
            runner,
            cache: HashMap::new(),
        }
    }

    fn render(&mut self, source: &str) -> &MermaidRenderOutcome {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let key = hasher.finish();

        self.cache
            .entry(key)
            .or_insert_with(|| match self.runner.render(source) {
                Ok(svg) => MermaidRenderOutcome::RenderedSvg(svg),
                Err(error) => MermaidRenderOutcome::Fallback(format!(
                    "Rust Mermaid subset renderer error: {error}"
                )),
            })
    }
}

#[cfg(feature = "mermaid")]
fn render_mermaid_svg(source: &str) -> std::result::Result<String, String> {
    let lines: Vec<String> = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if lines.is_empty() {
        return Err("empty mermaid source".to_owned());
    }
    let header = lines[0].to_lowercase();
    if header.starts_with("flowchart") || header.starts_with("graph") {
        return Ok(render_flowchart_svg(&lines));
    }
    if header.starts_with("sequencediagram") {
        return Ok(render_sequence_svg(&lines));
    }
    if header.starts_with("statediagram") {
        return Ok(render_state_svg(&lines));
    }
    if header.starts_with("classdiagram") {
        return Ok(render_class_svg(&lines));
    }
    if header.starts_with("erdiagram") {
        return Ok(render_er_svg(&lines));
    }
    if header.starts_with("gitgraph") {
        return Ok(render_git_graph_svg(&lines));
    }
    if header.starts_with("gantt") {
        return Ok(render_gantt_svg(&lines));
    }
    Err(format!("unsupported mermaid diagram type: {}", lines[0]))
}

#[cfg(feature = "mermaid")]
fn svg_frame(width: i32, height: i32, body: String) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\"><rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"#FFFFFF\"/>{body}</svg>"
    )
}

#[cfg(feature = "mermaid")]
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(feature = "mermaid")]
fn render_flowchart_svg(lines: &[String]) -> String {
    let mut nodes: Vec<String> = Vec::new();
    let mut edges: Vec<(String, String, Option<String>)> = Vec::new();
    for line in lines.iter().skip(1) {
        if let Some((from, rest)) = line.split_once("-->") {
            let from = from.trim().to_owned();
            let mut label = None;
            let to = if let Some((lbl, to)) = rest.split_once(':') {
                label = Some(lbl.trim().trim_matches('|').to_owned());
                to.trim().to_owned()
            } else {
                rest.trim().to_owned()
            };
            if !nodes.contains(&from) {
                nodes.push(from.clone());
            }
            if !nodes.contains(&to) {
                nodes.push(to.clone());
            }
            edges.push((from, to, label));
        }
    }
    if nodes.is_empty() {
        nodes.push("diagram".to_owned());
    }
    let width = 980;
    let height = 120 + (nodes.len() as i32 * 72);
    let mut body = String::from(
        "<defs><marker id=\"arr\" markerWidth=\"10\" markerHeight=\"7\" refX=\"9\" refY=\"3.5\" orient=\"auto\"><polygon points=\"0 0, 10 3.5, 0 7\" fill=\"#375A7F\"/></marker></defs>",
    );
    let mut positions = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        let x = 100 + ((i as i32 % 2) * 420);
        let y = 50 + ((i as i32 / 2) * 84);
        positions.insert(node.clone(), (x, y));
        body.push_str(&format!("<rect x=\"{x}\" y=\"{y}\" width=\"320\" height=\"52\" rx=\"8\" fill=\"#EDF2F7\" stroke=\"#4A6FA5\"/><text x=\"{}\" y=\"{}\" fill=\"#1F2937\" font-size=\"16\" font-family=\"Arial\" text-anchor=\"middle\">{}</text>", x + 160, y + 31, xml_escape(node)));
    }
    for (from, to, label) in edges {
        if let (Some((fx, fy)), Some((tx, ty))) = (positions.get(&from), positions.get(&to)) {
            let x1 = fx + 320;
            let y1 = fy + 26;
            let x2 = *tx;
            let y2 = ty + 26;
            body.push_str(&format!("<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#375A7F\" stroke-width=\"2\" marker-end=\"url(#arr)\"/>"));
            if let Some(label) = label {
                body.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"#4B5563\" font-size=\"13\" font-family=\"Arial\" text-anchor=\"middle\">{}</text>", (x1 + x2) / 2, (y1 + y2) / 2 - 6, xml_escape(&label)));
            }
        }
    }
    svg_frame(width, height.max(180), body)
}

#[cfg(feature = "mermaid")]
fn render_sequence_svg(lines: &[String]) -> String {
    let mut participants: Vec<String> = Vec::new();
    let mut messages: Vec<(String, String, String, bool)> = Vec::new();
    for line in lines.iter().skip(1) {
        if line.starts_with("participant") {
            let name = line.split_whitespace().last().unwrap_or("Actor").to_owned();
            if !participants.contains(&name) {
                participants.push(name);
            }
        } else if let Some((lhs, msg)) = line.split_once(':') {
            let dashed = lhs.contains("-->>");
            let arrow = if dashed { "-->>" } else { "->>" };
            if let Some((from, to)) = lhs.split_once(arrow) {
                let from = from.trim().to_owned();
                let to = to.trim().to_owned();
                if !participants.contains(&from) {
                    participants.push(from.clone());
                }
                if !participants.contains(&to) {
                    participants.push(to.clone());
                }
                messages.push((from, to, msg.trim().to_owned(), dashed));
            }
        }
    }
    let width = 180 + (participants.len() as i32 * 220);
    let height = 160 + (messages.len() as i32 * 64);
    let mut body = String::from(
        "<defs><marker id=\"arr\" markerWidth=\"10\" markerHeight=\"7\" refX=\"9\" refY=\"3.5\" orient=\"auto\"><polygon points=\"0 0, 10 3.5, 0 7\" fill=\"#375A7F\"/></marker></defs>",
    );
    let mut xmap = HashMap::new();
    for (i, p) in participants.iter().enumerate() {
        let x = 120 + (i as i32 * 220);
        xmap.insert(p.clone(), x);
        body.push_str(&format!("<rect x=\"{}\" y=\"24\" width=\"120\" height=\"34\" rx=\"6\" fill=\"#E5EDF7\" stroke=\"#4A6FA5\"/><text x=\"{}\" y=\"46\" text-anchor=\"middle\" font-size=\"14\" fill=\"#1F2937\">{}</text><line x1=\"{}\" y1=\"58\" x2=\"{}\" y2=\"{}\" stroke=\"#9CA3AF\" stroke-dasharray=\"6 5\"/>", x - 60, x, xml_escape(p), x, x, height - 20));
    }
    for (i, (from, to, msg, dashed)) in messages.iter().enumerate() {
        let y = 92 + (i as i32 * 56);
        if let (Some(x1), Some(x2)) = (xmap.get(from), xmap.get(to)) {
            let dash = if *dashed {
                " stroke-dasharray=\"6 4\""
            } else {
                ""
            };
            body.push_str(&format!("<line x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" stroke=\"#375A7F\" stroke-width=\"2\" marker-end=\"url(#arr)\"{dash}/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"13\" fill=\"#374151\">{}</text>", x1, x2, (x1 + x2) / 2, y - 8, xml_escape(msg)));
        }
    }
    svg_frame(width.max(600), height.max(220), body)
}

#[cfg(feature = "mermaid")]
fn render_state_svg(lines: &[String]) -> String {
    render_transition_svg(lines, "State Diagram")
}
#[cfg(feature = "mermaid")]
fn render_class_svg(lines: &[String]) -> String {
    render_transition_svg(lines, "Class Diagram")
}
#[cfg(feature = "mermaid")]
fn render_er_svg(lines: &[String]) -> String {
    render_transition_svg(lines, "ER Diagram")
}

#[cfg(feature = "mermaid")]
fn render_transition_svg(lines: &[String], title: &str) -> String {
    let mut transitions = Vec::new();
    for line in lines.iter().skip(1) {
        if let Some((a, b)) = line.split_once("-->") {
            transitions.push((a.trim().to_owned(), b.trim().to_owned()));
        } else if let Some((a, b)) = line.split_once("||--") {
            transitions.push((a.trim().to_owned(), b.trim().to_owned()));
        }
    }
    let width = 960;
    let height = 170 + transitions.len() as i32 * 60;
    let mut body = format!(
        "<text x=\"40\" y=\"36\" font-size=\"18\" fill=\"#1F2937\" font-family=\"Arial\">{}</text>",
        xml_escape(title)
    );
    for (i, (a, b)) in transitions.iter().enumerate() {
        let y = 70 + (i as i32 * 56);
        body.push_str(&format!("<rect x=\"40\" y=\"{}\" width=\"290\" height=\"38\" rx=\"8\" fill=\"#EDF2F7\" stroke=\"#4A6FA5\"/><text x=\"185\" y=\"{}\" text-anchor=\"middle\" font-size=\"14\" fill=\"#1F2937\">{}</text><line x1=\"330\" y1=\"{}\" x2=\"620\" y2=\"{}\" stroke=\"#375A7F\" stroke-width=\"2\"/><polygon points=\"620,{} 610,{} 610,{}\" fill=\"#375A7F\"/><rect x=\"620\" y=\"{}\" width=\"290\" height=\"38\" rx=\"8\" fill=\"#F8FAFC\" stroke=\"#4A6FA5\"/><text x=\"765\" y=\"{}\" text-anchor=\"middle\" font-size=\"14\" fill=\"#1F2937\">{}</text>", y, y + 24, xml_escape(a), y + 19, y + 19, y + 19, y + 14, y + 24, y, y + 24, xml_escape(b)));
    }
    svg_frame(width, height.max(220), body)
}

#[cfg(feature = "mermaid")]
fn render_git_graph_svg(lines: &[String]) -> String {
    let commits: Vec<String> = lines
        .iter()
        .skip(1)
        .filter(|l| l.starts_with("commit"))
        .map(|l| {
            l.replacen("commit id:", "", 1)
                .trim()
                .trim_matches('"')
                .to_owned()
        })
        .collect();
    let width = 980;
    let height = 220;
    let mut body = String::from(
        "<line x1=\"70\" y1=\"120\" x2=\"900\" y2=\"120\" stroke=\"#4A6FA5\" stroke-width=\"3\"/>",
    );
    for (i, commit) in commits.iter().enumerate() {
        let x = 90 + (i as i32 * 150);
        body.push_str(&format!("<circle cx=\"{x}\" cy=\"120\" r=\"14\" fill=\"#375A7F\"/><text x=\"{x}\" y=\"96\" text-anchor=\"middle\" font-size=\"12\" fill=\"#1F2937\">{}</text>", xml_escape(commit)));
    }
    svg_frame(width, height, body)
}

#[cfg(feature = "mermaid")]
fn render_gantt_svg(lines: &[String]) -> String {
    let mut tasks = Vec::new();
    let mut section = String::new();
    for line in lines.iter().skip(1) {
        if line.starts_with("section ") {
            section = line.replacen("section ", "", 1).trim().to_owned();
        } else if line.contains(':') {
            let name = line.split(':').next().unwrap_or(line).trim().to_owned();
            tasks.push((section.clone(), name));
        }
    }
    let width = 980;
    let height = 120 + tasks.len() as i32 * 44;
    let mut body = String::new();
    for (i, (section, task)) in tasks.iter().enumerate() {
        let y = 60 + i as i32 * 40;
        let x = 260 + ((i as i32 % 4) * 110);
        let w = 220;
        body.push_str(&format!("<text x=\"20\" y=\"{}\" font-size=\"12\" fill=\"#6B7280\">{}</text><text x=\"20\" y=\"{}\" font-size=\"14\" fill=\"#1F2937\">{}</text><rect x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"24\" rx=\"5\" fill=\"#4A6FA5\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#FFFFFF\">{}</text>", y - 10, xml_escape(section), y + 5, xml_escape(task), y - 14, x + (w/2), y + 2, xml_escape(task)));
    }
    svg_frame(width, height.max(220), body)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "mermaid")]
    use std::sync::{Arc, Mutex};

    use serde_json::json;

    use crate::adapters::render::plan::{page_spec, resolve_theme};
    use crate::core::{Block, BuiltInTheme, Document, Inline, LayoutMode, PageSize, ThemeConfig};

    use super::{TypstAssetLibrary, build_typst_source, heading_size_pt};

    fn source_for(document: &Document, mode: LayoutMode) -> String {
        let page = page_spec(PageSize::A4);
        let theme = resolve_theme(&ThemeConfig::default());
        let mut assets = TypstAssetLibrary::default();
        #[cfg(feature = "mermaid")]
        let mut mermaid = super::MermaidRenderContext::new(super::RustMermaidRunner);

        build_typst_source(
            document,
            page.width_mm,
            page.height_mm,
            &theme,
            mode,
            &mut assets,
            #[cfg(feature = "mermaid")]
            &mut mermaid,
        )
        .expect("source should render")
    }

    #[test]
    fn heading_size_uses_theme_heading_scale() {
        let compact = resolve_theme(&ThemeConfig {
            built_in: Some(BuiltInTheme::Engineering),
            custom_theme_json: Some(json!({"body_font_size_pt": 10.0, "heading_scale": 1.2})),
        });
        let large = resolve_theme(&ThemeConfig {
            built_in: Some(BuiltInTheme::Engineering),
            custom_theme_json: Some(json!({"body_font_size_pt": 10.0, "heading_scale": 2.0})),
        });

        assert_eq!(heading_size_pt(&compact, 1), 12.0);
        assert_eq!(heading_size_pt(&large, 1), 20.0);
        assert!(heading_size_pt(&large, 1) > heading_size_pt(&large, 2));
        assert!(heading_size_pt(&large, 2) > heading_size_pt(&large, 3));
    }

    #[test]
    fn typst_source_changes_when_heading_scale_changes() {
        let document = Document::new(vec![]);
        let page = page_spec(PageSize::A4);
        let compact = resolve_theme(&ThemeConfig {
            built_in: Some(BuiltInTheme::Engineering),
            custom_theme_json: Some(json!({"body_font_size_pt": 10.0, "heading_scale": 1.2})),
        });
        let large = resolve_theme(&ThemeConfig {
            built_in: Some(BuiltInTheme::Engineering),
            custom_theme_json: Some(json!({"body_font_size_pt": 10.0, "heading_scale": 2.0})),
        });

        #[cfg(feature = "mermaid")]
        let mut mermaid_compact = super::MermaidRenderContext::new(super::RustMermaidRunner);
        let compact_source = build_typst_source(
            &document,
            page.width_mm,
            page.height_mm,
            &compact,
            LayoutMode::Paged,
            &mut TypstAssetLibrary::default(),
            #[cfg(feature = "mermaid")]
            &mut mermaid_compact,
        )
        .expect("compact source should render");
        #[cfg(feature = "mermaid")]
        let mut mermaid_large = super::MermaidRenderContext::new(super::RustMermaidRunner);
        let large_source = build_typst_source(
            &document,
            page.width_mm,
            page.height_mm,
            &large,
            LayoutMode::Paged,
            &mut TypstAssetLibrary::default(),
            #[cfg(feature = "mermaid")]
            &mut mermaid_large,
        )
        .expect("large source should render");

        assert_ne!(compact_source, large_source);
        assert!(compact_source.contains("size: 12.00pt"));
        assert!(large_source.contains("size: 20.00pt"));
    }

    #[test]
    fn typst_source_emits_real_math_delimiters() {
        let document = Document::new(vec![
            Block::Paragraph {
                content: vec![
                    Inline::Text("Eq: ".to_owned()),
                    Inline::Math("\\alpha^2 + \\beta^2".to_owned()),
                ],
            },
            Block::Math {
                tex: "\\frac{1}{2}\\pi r^2".to_owned(),
            },
        ]);
        let source = source_for(&document, LayoutMode::Paged);

        assert!(source.contains('$'));
        assert!(source.contains("frac(1, 2)"));
        assert!(!source.contains("```math"));
        assert!(!source.contains("\\$"));
    }

    #[cfg(not(feature = "mermaid"))]
    #[test]
    fn typst_source_emits_mermaid_fallback_without_feature() {
        let document = Document::new(vec![Block::Mermaid {
            source: "flowchart TD\nA-->B".to_owned(),
        }]);
        let source = source_for(&document, LayoutMode::Paged);
        assert!(source.contains("[unsupported:mermaid]"));
        assert!(source.contains("flowchart TD"));
    }

    #[cfg(feature = "mermaid")]
    #[test]
    fn mermaid_renderer_caches_identical_diagrams() {
        #[derive(Clone)]
        struct FakeRunner {
            calls: Arc<Mutex<Vec<&'static str>>>,
        }

        impl super::MermaidRunner for FakeRunner {
            fn render(&self, _source: &str) -> crate::Result<Vec<u8>> {
                let marker = "svg";
                self.calls.lock().expect("mutex").push(marker);
                Ok(b"<svg/>".to_vec())
            }
        }

        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut context = super::MermaidRenderContext::new(FakeRunner {
            calls: calls.clone(),
        });

        let _ = context.render("flowchart TD\nA-->B");
        let _ = context.render("flowchart TD\nA-->B");

        let calls = calls.lock().expect("mutex");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "svg");
    }

    #[cfg(feature = "mermaid")]
    #[test]
    fn mermaid_subset_renderer_emits_svg_for_supported_types() {
        let inputs = [
            "flowchart LR\nA --> B",
            "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: hello",
            "stateDiagram-v2\n[*] --> Running\nRunning --> [*]",
            "classDiagram\nFoo --> Bar",
            "erDiagram\nA ||--o{ B : owns",
            "gitGraph\ncommit id: \"one\"",
            "gantt\nsection Build\nTask: p1, 2026-05-01, 1d",
        ];

        for input in inputs {
            let svg = super::render_mermaid_svg(input).expect("supported diagram should render");
            assert!(svg.starts_with("<svg"));
            assert!(svg.contains("</svg>"));
            assert!(!svg.contains("[unsupported:mermaid]"));
        }
    }

    #[cfg(feature = "mermaid")]
    #[test]
    fn mermaid_subset_renderer_is_deterministic() {
        let src = "flowchart LR\nA --> B\nB --> C";
        let a = super::render_mermaid_svg(src).expect("first render");
        let b = super::render_mermaid_svg(src).expect("second render");
        assert_eq!(a, b);
    }
}
