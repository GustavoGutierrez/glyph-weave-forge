#[cfg(not(feature = "fs"))]
use std::io;

use glyphweaveforge::Forge;

#[cfg(feature = "renderer-minimal")]
#[test]
fn renderer_minimal_feature_exposes_minimal_backend() {
    use glyphweaveforge::RenderBackendSelection;

    let result = Forge::new()
        .from_text("renderer minimal")
        .to_memory()
        .with_backend(RenderBackendSelection::Minimal)
        .convert()
        .expect("minimal backend should succeed");

    assert!(result.bytes.is_some());
}

#[test]
fn memory_only_conversion_is_always_available() {
    let result = Forge::new()
        .from_text("feature matrix")
        .to_memory()
        .convert()
        .expect("memory conversion should succeed");

    assert!(result.bytes.is_some());
}

#[cfg(not(feature = "fs"))]
#[test]
fn memory_only_builds_keep_custom_resource_resolvers() {
    let result = Forge::new()
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
        .expect("resolver-backed conversion should succeed");

    assert!(
        String::from_utf8_lossy(&result.bytes.expect("bytes should exist"))
            .contains("Image: Logo | png")
    );
}

#[cfg(feature = "fs")]
#[test]
fn fs_feature_exposes_path_and_directory_helpers() {
    use tempfile::tempdir;

    let temp = tempdir().expect("tempdir should exist");
    let markdown = temp.path().join("matrix.md");
    std::fs::write(&markdown, "matrix fs").expect("markdown should be written");

    let result = Forge::new()
        .from_path(&markdown)
        .to_directory(temp.path())
        .convert()
        .expect("fs-backed conversion should succeed");

    assert!(
        result
            .written_path
            .expect("written path should exist")
            .exists()
    );
}

#[cfg(feature = "renderer-typst")]
#[test]
fn renderer_typst_feature_exposes_typst_backend() {
    use glyphweaveforge::RenderBackendSelection;

    let result = Forge::new()
        .from_text("renderer typst")
        .to_memory()
        .with_backend(RenderBackendSelection::Typst)
        .convert()
        .expect("typst backend should succeed");

    let bytes = result.bytes.expect("bytes should exist");
    assert!(bytes.starts_with(b"%PDF"));
}

#[cfg(feature = "typst")]
#[test]
fn typst_alias_feature_keeps_typst_backend_selectable() {
    use glyphweaveforge::RenderBackendSelection;

    let result = Forge::new()
        .from_text("typst alias")
        .to_memory()
        .with_backend(RenderBackendSelection::Typst)
        .convert()
        .expect("typst alias feature should enable typst backend");

    let bytes = result.bytes.expect("bytes should exist");
    assert!(bytes.starts_with(b"%PDF"));
}
