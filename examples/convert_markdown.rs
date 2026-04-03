use std::error::Error;
use std::path::PathBuf;

use glyphweaveforge::{BuiltInTheme, Forge, PageSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendArg {
    Minimal,
    #[cfg(feature = "renderer-typst")]
    Typst,
}

impl BackendArg {
    fn parse(value: Option<&str>) -> Result<Self, &'static str> {
        match value.unwrap_or("minimal") {
            "minimal" => Ok(Self::Minimal),
            #[cfg(feature = "renderer-typst")]
            "typst" => Ok(Self::Typst),
            #[cfg(not(feature = "renderer-typst"))]
            "typst" => Err("the 'typst' backend requires the 'renderer-typst' feature"),
            _ => Err("backend must be 'minimal' or 'typst'"),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = PathBuf::from(args.next().ok_or("missing input markdown path")?);
    let output = PathBuf::from(args.next().ok_or("missing output pdf path")?);
    let backend = BackendArg::parse(args.next().as_deref().and_then(|value| value.to_str()))?;

    let builder = Forge::new()
        .from_path(&input)
        .to_file(&output)
        .with_page_size(PageSize::A4)
        .with_theme(BuiltInTheme::ScientificArticle);

    #[cfg(feature = "renderer-typst")]
    let builder = match backend {
        BackendArg::Minimal => builder,
        BackendArg::Typst => builder.with_backend(glyphweaveforge::RenderBackendSelection::Typst),
    };

    #[cfg(not(feature = "renderer-typst"))]
    let builder = match backend {
        BackendArg::Minimal => builder,
    };

    let result = builder.convert()?;

    if let Some(written_path) = result.written_path {
        println!("{}", written_path.display());
    }

    Ok(())
}
