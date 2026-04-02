![GlyphWeaveForge logo](logo.png)

# GlyphWeaveForge

GlyphWeaveForge converts Markdown into PDF through a small Rust pipeline with explicit boundaries:

`api -> pipeline -> core -> adapters`

The crate ships a lightweight built-in renderer by default and can optionally use Typst without changing the public builder API.

## Installation

```toml
[dependencies]
glyphweaveforge = "0.1"
```

Enable optional features when you need them:

```toml
[dependencies]
glyphweaveforge = { version = "0.1", features = ["renderer-typst"] }
```

## Main builder flow

```rust
use glyphweaveforge::Forge;

let pdf = Forge::new()
    .from_text("# Hello\n\nWorld")
    .to_memory()
    .convert()
    .expect("conversion should succeed");

let bytes = pdf.bytes.expect("memory output should contain PDF bytes");
assert!(bytes.starts_with(b"%PDF"));
```

## Supported inputs and outputs

Always available:

- `from_text`
- `from_bytes`
- `to_memory`
- injected resource resolvers through `with_resource_resolver`
- explicit backend selection through `with_backend`

Available with the default `fs` feature:

- `from_path`
- `to_file`
- `to_directory`
- filesystem-based resource lookup for local assets

## Backend selection

The default build uses the minimal built-in renderer. To opt into the Typst backend, enable `renderer-typst` and select it explicitly:

```rust
# #[cfg(feature = "renderer-typst")]
# {
use glyphweaveforge::{Forge, RenderBackendSelection};

let pdf = Forge::new()
    .from_text("# Hello from Typst")
    .to_memory()
    .with_backend(RenderBackendSelection::Typst)
    .convert()
    .expect("typst backend should succeed");

assert!(pdf.bytes.expect("bytes should exist").starts_with(b"%PDF"));
# }
```

## Feature matrix

- `fs` (default): enables path-based source/output helpers and filesystem resource loading.
- `renderer-minimal` (default): keeps the built-in lightweight renderer enabled and addressable in tests/documentation.
- `renderer-typst`: enables the Typst-backed renderer.
- `typst`: compatibility alias for `renderer-typst`.
- `mermaid`: currently does **not** add real Mermaid rendering; fenced `mermaid` blocks remain visible as explicit unsupported fallbacks.
- `math`: currently does **not** add real math layout; fenced `math` blocks remain visible as explicit unsupported fallbacks.

## Current Markdown behavior

Supported today:

- headings
- paragraphs
- emphasis/strong text
- inline code
- links
- unordered and ordered lists
- block quotes
- thematic breaks
- fenced code blocks
- images, including injected resolvers and memory-backed assets

Unsupported advanced Markdown stays visible instead of silently disappearing. Examples include:

- tables
- footnotes
- Mermaid diagrams
- math fences

These paths render deterministic fallback labels with the original content preserved in the output.

## Limitations

- This crate does **not** claim real Mermaid diagram rendering.
- This crate does **not** claim real TeX/LaTeX or advanced math typesetting.
- Advanced Markdown such as tables and footnotes is exposed through visible fallback text, not full layout support.
- The minimal renderer writes a compact PDF text stream; it is designed for deterministic output and testability rather than rich page design.

## Architecture note

Release work should preserve the internal boundary:

`api -> pipeline -> core -> adapters`
