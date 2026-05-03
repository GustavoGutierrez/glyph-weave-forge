# GlyphWeaveForge integration guide

This document explains how to integrate `glyphweaveforge` into another Rust project,
what was validated against the published crate, and which Markdown features are
currently supported.

## Important scope note

`glyphweaveforge` is a **library crate**, not a standalone CLI tool.

To use it from another project, create your own small binary and call the public
builder API (`Forge`) directly.

## Installation

Minimal renderer only:

```toml
[dependencies]
glyphweaveforge = "0.1.5"
```

Typst renderer enabled:

```toml
[dependencies]
glyphweaveforge = { version = "0.1.5", features = ["renderer-typst"] }
```

## Recommended usage

For rich PDF output, prefer the Typst backend explicitly:

```rust
use glyphweaveforge::{BuiltInTheme, Forge, PageSize, RenderBackendSelection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Forge::new()
        .from_path("README.md")
        .to_file("README.pdf")
        .with_backend(RenderBackendSelection::Typst)
        .with_page_size(PageSize::A4)
        .with_theme(BuiltInTheme::ScientificArticle)
        .convert()?;

    Ok(())
}
```

## In-memory conversion example

Use this path when integrating with web services, WASM, or custom upload flows.

```rust
use std::fs;

use glyphweaveforge::{BuiltInTheme, Forge, PageSize, RenderBackendSelection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = r#"
# Memory example

Text with **bold**, *italic*, and `inline code`.

![Logo](logo.png)
"#;

    let pdf = Forge::new()
        .from_text(markdown)
        .to_memory()
        .with_backend(RenderBackendSelection::Typst)
        .with_page_size(PageSize::A4)
        .with_theme(BuiltInTheme::ScientificArticle)
        .with_resource_resolver(|href| {
            if href == "logo.png" {
                fs::read("./assets/logo.png")
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("resource not found: {href}"),
                ))
            }
        })
        .convert()?;

    let bytes = pdf.bytes.expect("expected in-memory PDF bytes");
    fs::write("memory-example.pdf", bytes)?;
    Ok(())
}
```

## Theme validation

The built-in themes are:

- `BuiltInTheme::Invoice`
- `BuiltInTheme::ScientificArticle`
- `BuiltInTheme::Professional`
- `BuiltInTheme::Engineering`
- `BuiltInTheme::Informational`

The Typst renderer applies theme-specific:

- body fonts
- heading fonts
- text colors
- link colors
- code background colors
- quote background colors
- margins

Example of iterating across themes:

```rust
use glyphweaveforge::{BuiltInTheme, Forge, PageSize, RenderBackendSelection};

fn all_themes() -> [(&'static str, BuiltInTheme); 5] {
    [
        ("invoice", BuiltInTheme::Invoice),
        ("scientific-article", BuiltInTheme::ScientificArticle),
        ("professional", BuiltInTheme::Professional),
        ("engineering", BuiltInTheme::Engineering),
        ("informational", BuiltInTheme::Informational),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for (name, theme) in all_themes() {
        Forge::new()
            .from_path("sample.md")
            .to_file(format!("{name}.pdf"))
            .with_backend(RenderBackendSelection::Typst)
            .with_page_size(PageSize::A4)
            .with_theme(theme)
            .convert()?;
    }

    Ok(())
}
```

## Feature compatibility matrix

### Validated in the published crate `0.1.5`

Validated from a separate consumer project depending on `glyphweaveforge = "0.1.5"`
from crates.io:

- UTF-8 text renders visibly
- headings render
- unordered and ordered lists render
- block quotes render
- fenced code blocks render
- inline code renders
- Markdown links render through the Typst backend
- standard Markdown images render and embed into the PDF
- basic HTML `<img>` tags resolve through the same image pipeline
- standard Markdown tables render as real tables
- built-in themes generate non-blank PDFs

### Limitations in the published crate `0.1.5`

The published `0.1.5` crate still has these limits:

- footnotes render as fallbacks
- Mermaid fences render as visible fallbacks
- math fences render as visible fallbacks
- broader raw HTML layout is not supported beyond basic `<img>` extraction

## Link support notes

Links are supported in the Typst backend through the public API. A generated Typst
source should contain entries like:

```typst
#link("https://example.com/docs")[site]
```

If you inspect extracted PDF text, you will usually see only the visible label, not the
visual styling. Link annotations are stored in the PDF structure.

## Italic and bold notes

The Typst backend emits emphasis and strong markup from the Markdown AST. For example:

```typst
This paragraph mixes *bold* and _italic_.
```

And it enables a Typst emphasis rule:

```typst
#show emph: set text(style: "italic")
```

PDF text extraction tools typically strip formatting, so visual emphasis may not be
obvious in extracted text even when it renders correctly in a viewer.

## Best practices

- Prefer `from_path(...)` when your Markdown contains local images.
- Prefer standard Markdown image syntax over raw HTML when possible.
- Prefer the Typst backend for styled output.
- If you use `from_text(...)` or `from_bytes(...)`, inject assets with
  `with_resource_resolver(...)`.
- Keep tables compact and semantic.
- Use Markdown links instead of HTML whenever possible.

## Troubleshooting blank PDFs

If your PDF appears blank in another project:

1. Ensure the dependency enables `renderer-typst`.
2. Call `.with_backend(RenderBackendSelection::Typst)` explicitly.
3. Prefer `from_path(...)` for documents with local assets.
4. If using in-memory input, provide a resolver with `with_resource_resolver(...)`.
5. Make sure your environment allows Typst font discovery. The current Typst backend
   uses system and embedded font discovery through `typst-as-lib`.

## Honest status summary

If you need **published crates.io `0.1.5` exactly as-is**, it is safe for:

- headings
- lists
- quotes
- code
- links
- Markdown images
- basic HTML `<img>` image extraction
- standard Markdown tables
- themed Typst PDFs that are not blank

If you need broader raw HTML support beyond `<img>`, that still requires future work.
