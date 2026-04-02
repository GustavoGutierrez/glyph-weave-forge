use crate::adapters::render::plan::{
    PageSpec, RenderPage, build_render_plan, paginate, render_lines,
};
use crate::core::ports::{RenderBackend, RenderRequest};
use crate::core::{Document, Result};

#[derive(Debug, Default)]
pub struct MinimalPdfRenderer;

impl RenderBackend for MinimalPdfRenderer {
    fn render(&self, document: &Document, request: &RenderRequest) -> Result<Vec<u8>> {
        let plan = build_render_plan(document, request)?;
        let lines = render_lines(&plan);
        let pages = paginate(
            &lines,
            plan.page_size,
            request.layout_mode,
            plan.theme.margin_mm,
        );
        Ok(MinimalPdfWriter::from_pages(&request.source_name, plan.page_size, &pages).finish())
    }
}

struct MinimalPdfWriter {
    objects: Vec<Vec<u8>>,
}

impl MinimalPdfWriter {
    fn from_pages(title: &str, page_size: PageSpec, pages: &[RenderPage]) -> Self {
        let mut writer = Self {
            objects: Vec::new(),
        };
        let info_id =
            writer.push_object(format!("<< /Title ({}) >>", escape_pdf_text(title)).into_bytes());
        let pages_id = writer.reserve_object();
        let font_id = writer.push_object(
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_vec(),
        );
        let mut page_ids = Vec::new();
        for page in pages {
            let content_id = writer.push_stream_object(build_content_stream(
                page.lines
                    .iter()
                    .map(|line| (line.text.as_str(), line.font_size_pt)),
            ));
            let page_id = writer.push_object(
                format!(
                    "<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 {font_id} 0 R >> >> /Contents {content_id} 0 R >>",
                    mm_to_points(page_size.width_mm),
                    mm_to_points(page_size.height_mm)
                )
                .into_bytes(),
            );
            page_ids.push(page_id);
        }
        writer.replace_object(
            pages_id,
            format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>",
                page_ids
                    .iter()
                    .map(|id| format!("{id} 0 R"))
                    .collect::<Vec<_>>()
                    .join(" "),
                page_ids.len()
            )
            .into_bytes(),
        );
        let catalog_id =
            writer.push_object(format!("<< /Type /Catalog /Pages {pages_id} 0 R >>").into_bytes());
        let _ = (info_id, catalog_id);
        writer
    }

    fn reserve_object(&mut self) -> usize {
        self.objects.push(Vec::new());
        self.objects.len()
    }

    fn push_object(&mut self, content: Vec<u8>) -> usize {
        self.objects.push(content);
        self.objects.len()
    }

    fn replace_object(&mut self, id: usize, content: Vec<u8>) {
        self.objects[id - 1] = content;
    }

    fn push_stream_object(&mut self, stream: Vec<u8>) -> usize {
        let mut object = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
        object.extend_from_slice(&stream);
        object.extend_from_slice(b"\nendstream");
        self.push_object(object)
    }

    fn finish(self) -> Vec<u8> {
        let info_id = 1usize;
        let catalog_id = self.objects.len();
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0usize];
        for (index, object) in self.objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            pdf.extend_from_slice(object);
            pdf.extend_from_slice(b"\nendobj\n");
        }
        let xref_offset = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", self.objects.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets.iter().skip(1) {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Info {info_id} 0 R /Root {catalog_id} 0 R >>\nstartxref\n{xref_offset}\n%%EOF",
                self.objects.len() + 1
            )
            .as_bytes(),
        );
        pdf
    }
}

fn build_content_stream<'a>(lines: impl Iterator<Item = (&'a str, f32)>) -> Vec<u8> {
    let mut stream = Vec::new();
    let mut y = 800.0f32;
    for (text, font_size) in lines {
        let safe_text = escape_pdf_text(text);
        stream.extend_from_slice(
            format!("BT\n/F1 {font_size:.2} Tf\n1 0 0 1 40 {y:.2} Tm\n({safe_text}) Tj\nET\n")
                .as_bytes(),
        );
        y -= (font_size + 6.0).max(12.0);
    }
    stream
}

fn escape_pdf_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '(' => escaped.push_str("\\("),
            ')' => escaped.push_str("\\)"),
            '\\' => escaped.push_str("\\\\"),
            '\n' | '\r' => escaped.push(' '),
            _ if ch.is_ascii() && !ch.is_ascii_control() => escaped.push(ch),
            _ => escaped.push('?'),
        }
    }
    escaped
}

fn mm_to_points(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}
