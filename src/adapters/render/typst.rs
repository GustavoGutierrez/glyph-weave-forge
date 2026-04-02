use typst::layout::PagedDocument;
use typst_as_lib::TypstEngine;

use crate::adapters::render::plan::{
    RenderCodeBlock, RenderElement, RenderImage, RenderLine, build_render_plan,
};
use crate::core::ports::{RenderBackend, RenderRequest};
use crate::core::{Document, ForgeError, Result};

#[derive(Debug, Default)]
pub struct TypstPdfRenderer;

impl RenderBackend for TypstPdfRenderer {
    fn render(&self, document: &Document, request: &RenderRequest) -> Result<Vec<u8>> {
        let plan = build_render_plan(document, request)?;
        let assets = TypstAssetLibrary::from_elements(&plan.elements)?;
        let source = build_typst_source(
            plan.page_size.width_mm,
            plan.page_size.height_mm,
            plan.theme.margin_mm,
            &plan.elements,
            &assets,
        );
        let engine = TypstEngine::builder()
            .main_file(("main.typ", source))
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
    fn from_elements(elements: &[RenderElement]) -> Result<Self> {
        let mut files = Vec::new();
        for element in elements {
            if let RenderElement::Image(image) = element {
                if image.bytes.is_empty() {
                    return Err(ForgeError::TypstAsset {
                        target: image.original.clone(),
                        message: "resolved image bytes were empty".to_owned(),
                    });
                }
                let path = asset_path(files.len(), image);
                files.push((path, image.bytes.clone()));
            }
        }
        Ok(Self { files })
    }

    fn path_for(&self, index: usize) -> Option<&str> {
        self.files.get(index).map(|(path, _)| path.as_str())
    }

    fn files(&self) -> Vec<(&str, &[u8])> {
        self.files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
            .collect()
    }
}

fn build_typst_source(
    page_width_mm: f32,
    page_height_mm: f32,
    margin_mm: f32,
    elements: &[RenderElement],
    assets: &TypstAssetLibrary,
) -> String {
    let mut source = format!(
        "#set page(width: {page_width_mm:.1}mm, height: {page_height_mm:.1}mm, margin: {margin_mm:.1}mm)\n#set par(justify: false)\n\n"
    );
    let mut image_index = 0usize;
    for element in elements {
        match element {
            RenderElement::Line(line) => push_line(&mut source, line),
            RenderElement::CodeBlock(block) => push_code_block(&mut source, block),
            RenderElement::Image(image) => {
                if let Some(path) = assets.path_for(image_index) {
                    source.push_str(&format!(
                        "#image(\"{}\", width: 100%)\n",
                        escape_typst_string(path)
                    ));
                    source.push_str(&format!(
                        "#par([{}])\n#par([{}])\n\n",
                        escape_typst_text(&image.alt),
                        escape_typst_text(&image.message)
                    ));
                } else {
                    push_line(
                        &mut source,
                        &RenderLine {
                            text: format!("[Missing image asset: {}]", image.original),
                            font_size_pt: 11.0,
                        },
                    );
                }
                image_index += 1;
            }
        }
    }
    source
}

fn push_line(source: &mut String, line: &RenderLine) {
    if line.text.is_empty() {
        source.push_str("#v(0.8em)\n");
        return;
    }

    source.push_str(&format!(
        "#set text(size: {:.2}pt)\n#par([{}])\n\n",
        line.font_size_pt,
        escape_typst_text(&line.text)
    ));
}

fn push_code_block(source: &mut String, block: &RenderCodeBlock) {
    source.push_str(&format!(
        "#set text(size: {:.2}pt, font: \"DejaVu Sans Mono\")\n#par([{}])\n",
        block.font_size_pt,
        escape_typst_text(&block.summary)
    ));

    for line in &block.lines {
        source.push_str(&format!("#par([{}])\n", escape_typst_text(line)));
    }

    source.push('\n');
}

fn asset_path(index: usize, image: &RenderImage) -> String {
    let extension = match image.format {
        "jpeg" => "jpg",
        other => other,
    };
    format!("assets/image-{index}.{extension}")
}

fn escape_typst_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '[' => escaped.push_str("\\["),
            ']' => escaped.push_str("\\]"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn escape_typst_string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}
