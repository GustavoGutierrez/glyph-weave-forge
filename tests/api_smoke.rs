use std::io;

#[cfg(feature = "renderer-typst")]
use glyphweaveforge::RenderBackendSelection;
use glyphweaveforge::{BuiltInTheme, Forge, LayoutMode, PageSize, ThemeConfig};
use serde_json::json;
#[cfg(feature = "fs")]
use tempfile::tempdir;

fn pdf_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn cargo_manifest() -> &'static str {
    include_str!("../Cargo.toml")
}

fn crate_docs_source() -> &'static str {
    include_str!("../README.md")
}

fn crate_root_docs() -> &'static str {
    include_str!("../src/lib.rs")
}

fn agents_guidance() -> &'static str {
    include_str!("../AGENTS.md")
}

#[cfg(feature = "fs")]
#[test]
fn path_bytes_and_text_inputs_work() {
    let temp = tempdir().expect("tempdir should exist");
    let path = temp.path().join("doc.md");
    std::fs::write(&path, "# Title").expect("markdown should be written");

    let path_pdf = Forge::new()
        .from_path(&path)
        .to_memory()
        .convert()
        .expect("path conversion");
    let bytes_pdf = Forge::new()
        .from_bytes(b"Bytes source")
        .to_memory()
        .convert()
        .expect("bytes conversion");
    let text_pdf = Forge::new()
        .from_text("Text source")
        .to_memory()
        .convert()
        .expect("text conversion");

    assert!(pdf_text(&path_pdf.bytes.expect("path bytes")).contains("Title"));
    assert!(pdf_text(&bytes_pdf.bytes.expect("bytes bytes")).contains("Bytes source"));
    assert!(pdf_text(&text_pdf.bytes.expect("text bytes")).contains("Text source"));
}

#[cfg(feature = "fs")]
#[test]
fn file_and_directory_outputs_work() {
    let temp = tempdir().expect("tempdir should exist");
    let file_output = temp.path().join("out.pdf");
    Forge::new()
        .from_text("file output")
        .to_file(&file_output)
        .convert()
        .expect("file output");
    assert!(file_output.exists());

    let markdown = temp.path().join("guide.md");
    std::fs::write(&markdown, "directory output").expect("markdown should be written");
    let result = Forge::new()
        .from_path(&markdown)
        .to_directory(temp.path())
        .convert()
        .expect("dir output");
    assert!(result.written_path.expect("written path").exists());
}

#[test]
fn theme_and_page_configuration_are_visible() {
    let result = Forge::new()
        .from_text("# Report")
        .to_memory()
        .with_theme_config(ThemeConfig {
            built_in: Some(BuiltInTheme::Engineering),
            custom_theme_json: Some(json!({"name": "integration-theme"})),
        })
        .with_page_size(PageSize::Legal)
        .with_layout_mode(LayoutMode::SinglePage)
        .convert()
        .expect("conversion should succeed");

    let text = pdf_text(&result.bytes.expect("bytes should exist"));
    assert!(text.contains("integration-theme"));
    assert!(text.contains("215.9x355.6mm"));
}

#[test]
fn readme_primary_builder_flow_matches_public_api() {
    let pdf = Forge::new()
        .from_text("# Hello\n\nWorld")
        .to_memory()
        .convert()
        .expect("README example should succeed");

    let text = pdf_text(&pdf.bytes.expect("bytes should exist"));
    assert!(text.contains("Hello"));
    assert!(text.contains("World"));
}

#[test]
fn manifest_keeps_required_publication_metadata_honest() {
    let manifest = cargo_manifest();

    for required_field in [
        "description = ",
        "license = \"Apache-2.0\"",
        "rust-version = \"1.85\"",
        "repository = \"https://github.com/GustavoGutierrez/glyph-weave-forge\"",
        "readme = \"README.md\"",
        "documentation = \"https://docs.rs/glyphweaveforge\"",
        "keywords = [\"markdown\", \"pdf\", \"converter\", \"typst\"]",
        "categories = [\"text-processing\", \"rendering\"]",
    ] {
        assert!(
            manifest.contains(required_field),
            "manifest should contain {required_field}"
        );
    }

    assert!(!manifest.contains("browser"));
    assert!(!manifest.contains("latex"));
    assert!(!manifest.contains("/home/"));
}

#[test]
fn public_docs_describe_real_features_and_limitations() {
    let readme = crate_docs_source();
    let crate_docs = crate_root_docs();

    for section in [
        "## Installation",
        "## Main builder flow",
        "## Supported inputs and outputs",
        "## Backend selection",
        "## Feature matrix",
        "## Limitations",
    ] {
        assert!(readme.contains(section), "README should contain {section}");
    }

    for truthful_claim in [
        "renderer-typst",
        "renderer-minimal",
        "with_resource_resolver",
        "does **not** add real Mermaid rendering",
        "does **not** add real math layout",
        "Advanced Markdown such as tables and footnotes is exposed through visible fallback text",
    ] {
        assert!(
            readme.contains(truthful_claim),
            "README should contain {truthful_claim}"
        );
    }

    assert!(crate_docs.contains("include_str!(\"../README.md\")"));
}

#[test]
fn public_artifacts_do_not_embed_local_machine_paths() {
    for public_text in [cargo_manifest(), crate_docs_source(), crate_root_docs()] {
        for forbidden in [
            "/home/",
            "/Users/",
            "C:\\\\",
            ".config/opencode",
            "/.cargo/",
            "meridian",
        ] {
            assert!(
                !public_text.contains(forbidden),
                "public artifact should not contain {forbidden}"
            );
        }
    }
}

#[test]
fn agents_file_documents_security_and_publication_hygiene() {
    let agents = agents_guidance();

    for expected in [
        "api -> pipeline -> core -> adapters",
        "Public-facing crate documentation must stay in **English**.",
        "Never publish secrets, tokens, credentials, private keys, or local environment details.",
        "Do not add absolute filesystem paths",
        "cargo publish --dry-run",
    ] {
        assert!(
            agents.contains(expected),
            "AGENTS.md should contain {expected}"
        );
    }
}

#[test]
fn missing_resources_and_custom_resolvers_have_visible_output() {
    let missing = Forge::new()
        .from_text("![Missing](nope.png)")
        .to_memory()
        .convert()
        .expect("missing image should still render");
    assert!(
        pdf_text(&missing.bytes.expect("bytes should exist")).contains("Missing image: Missing")
    );

    let resolved = Forge::new()
        .from_text("![Logo](logo.png)")
        .to_memory()
        .with_resource_resolver(|path| {
            if path == "logo.png" {
                Ok(vec![1, 2, 3])
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
            }
        })
        .convert()
        .expect("resolved image should render");
    assert!(pdf_text(&resolved.bytes.expect("bytes should exist")).contains("Image: Logo | png"));
}

#[cfg(not(feature = "fs"))]
#[test]
fn no_fs_builds_still_support_injected_resource_resolvers() {
    let resolved = Forge::new()
        .from_text("![Logo](logo.png)")
        .to_memory()
        .with_resource_resolver(|path| {
            if path == "logo.png" {
                Ok(vec![1, 2, 3])
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
            }
        })
        .convert()
        .expect("resolved image should render without fs");

    assert!(pdf_text(&resolved.bytes.expect("bytes should exist")).contains("Image: Logo | png"));
}

#[cfg(feature = "renderer-typst")]
#[test]
fn typst_backend_works_with_memory_only_input_and_output() {
    let result = Forge::new()
        .from_text("memory only typst backend")
        .to_memory()
        .with_backend(RenderBackendSelection::Typst)
        .convert()
        .expect("typst backend should succeed without fs");

    let bytes = result.bytes.expect("bytes should exist");
    assert!(bytes.starts_with(b"%PDF"));
}
