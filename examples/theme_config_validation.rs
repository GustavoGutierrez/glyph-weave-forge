use std::error::Error;
use std::path::PathBuf;

use glyphweaveforge::{BuiltInTheme, Forge, PageSize, ThemeConfig};
use serde_json::json;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = PathBuf::from(args.next().ok_or("missing input markdown path")?);
    let output = PathBuf::from(args.next().ok_or("missing output pdf path")?);
    let heading_scale: f64 = args
        .next()
        .as_deref()
        .and_then(|value| value.to_str())
        .ok_or("missing heading scale")?
        .parse()?;

    let builder = Forge::new()
        .from_path(&input)
        .to_file(&output)
        .with_page_size(PageSize::A4)
        .with_theme_config(ThemeConfig {
            built_in: Some(BuiltInTheme::Engineering),
            custom_theme_json: Some(json!({
                "name": format!("engineering-heading-scale-{heading_scale}"),
                "body_font_size_pt": 10.0,
                "code_font_size_pt": 8.5,
                "heading_scale": heading_scale,
                "margin_mm": 14.0
            })),
        });

    #[cfg(feature = "renderer-typst")]
    let builder = builder.with_backend(glyphweaveforge::RenderBackendSelection::Typst);

    let result = builder.convert()?;

    if let Some(written_path) = result.written_path {
        println!("{}", written_path.display());
    }

    Ok(())
}
