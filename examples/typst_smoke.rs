#[cfg(feature = "renderer-typst")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    use typst::layout::PagedDocument;
    use typst_as_lib::TypstEngine;
    use typst_as_lib::typst_kit_options::TypstKitFontOptions;

    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("missing output pdf path")?;

    let source = r#"
#set page(paper: "a4", margin: 20mm)
#set text(font: "DejaVu Serif", size: 11pt)

= Hola mundo

Este es un párrafo normal con acentos: Bogotá, información, número 123.

- Lista con guiones
- Ítems normales

_Itálicas_ y *negritas* también funcionan.
"#;

    let engine = TypstEngine::builder()
        .main_file(source)
        .search_fonts_with(
            TypstKitFontOptions::default()
                .include_system_fonts(true)
                .include_embedded_fonts(true),
        )
        .build();
    let warned = engine.compile::<PagedDocument>();
    let document = warned
        .output
        .map_err(|error| format!("typst compile failed: {error}"))?;
    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|errors| format!("typst pdf export failed: {errors:?}"))?;
    std::fs::write(output, pdf)?;
    Ok(())
}

#[cfg(not(feature = "renderer-typst"))]
fn main() {
    eprintln!("this example requires the 'renderer-typst' feature");
    std::process::exit(1);
}
