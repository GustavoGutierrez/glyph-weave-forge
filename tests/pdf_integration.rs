use glyphweaveforge::Forge;
#[cfg(feature = "renderer-typst")]
use glyphweaveforge::{LayoutMode, RenderBackendSelection};

fn pdf_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn unsupported_markdown_and_standard_semantics_survive_rendering() {
    let markdown = "# Title\n\n- *one*\n- **two**\n\n> quote [link](https://example.com)\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\n<img src=\"logo.png\" alt=\"HTML logo\" />";
    let result = Forge::new()
        .from_text(markdown)
        .to_memory()
        .with_resource_resolver(|path| {
            if path == "logo.png" {
                Ok(vec![1, 2, 3])
            } else {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"))
            }
        })
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("- *one*"));
    assert!(text.contains("- **two**"));
    assert!(text.contains("> quote"));
    assert!(text.contains("https://example.com"));
    assert!(text.contains("a | b"));
    assert!(text.contains("1 | 2"));
    assert!(text.contains("Image: HTML logo | png"));
    assert!(!text.contains("Unsupported table"));
}

#[test]
fn mermaid_fallback_is_feature_sensitive() {
    let result = Forge::new()
        .from_text("```mermaid\nflowchart TD\n```")
        .to_memory()
        .convert()
        .expect("conversion should succeed");
    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("[unsupported:mermaid]"));
    assert!(text.contains("flowchart TD"));
}

#[test]
fn footnote_math_and_code_fallbacks_remain_visible() {
    let markdown =
        "alpha[^1]\n\n[^1]: visible note\n\n```math\nx^2 + 1\n```\n\n```rust\nfn main() {}\n```";
    let result = Forge::new()
        .from_text(markdown)
        .to_memory()
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("Unsupported footnote fallback"));
    assert!(text.contains("visible note"));
    assert!(text.contains("[unsupported:math]"));
    assert!(text.contains("x^2 + 1"));
    assert!(text.contains("[code:rust]"));
    assert!(text.contains("fn main"));
}

#[cfg(feature = "renderer-typst")]
#[test]
fn single_page_layout_produces_valid_pdf() {
    let markdown = "# Heading\n\nA representative paragraph with some content.\n\n- First item\n- Second item\n- Third item\n";
    let result = Forge::new()
        .from_text(markdown)
        .to_memory()
        .with_backend(RenderBackendSelection::Typst)
        .with_layout_mode(LayoutMode::SinglePage)
        .convert();

    assert!(
        result.is_ok(),
        "single-page conversion should succeed: {result:?}"
    );
    let bytes = result.unwrap().bytes.expect("bytes should exist");
    assert!(!bytes.is_empty(), "output bytes should not be empty");
    assert!(
        bytes.starts_with(b"%PDF"),
        "output should be a valid PDF (starts with %PDF)"
    );
}
