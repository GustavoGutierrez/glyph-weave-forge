use glyphweaveforge::Forge;

fn pdf_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn unsupported_markdown_and_standard_semantics_survive_rendering() {
    let markdown = "# Title\n\n- *one*\n- **two**\n\n> quote [link](https://example.com)\n\n| a | b |\n| - | - |\n| 1 | 2 |";
    let result = Forge::new()
        .from_text(markdown)
        .to_memory()
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("- *one*"));
    assert!(text.contains("- **two**"));
    assert!(text.contains("> quote"));
    assert!(text.contains("https://example.com"));
    assert!(text.contains("Unsupported table fallback"));
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
