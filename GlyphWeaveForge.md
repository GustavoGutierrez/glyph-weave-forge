# GlyphWeaveForge Technical Overview

GlyphWeaveForge is a Rust library crate that converts Markdown to PDF while
preserving explicit architecture boundaries and honest feature behavior.

## Core intent

- Native Rust-first Markdown-to-PDF conversion
- Embeddable library API for host applications
- Stable high-level builder flow with explicit extension points

## Architecture

```text
api -> pipeline -> core -> adapters
```

Each layer has a clear responsibility:

- **api**: public builder and configuration surface
- **pipeline**: orchestration from source to render request
- **core**: domain model, contracts, and errors
- **adapters**: parser, renderer, filesystem/resource integrations

## Public capability summary

- Source input from text, bytes, and filesystem paths (`fs` feature)
- Output to memory, explicit file path, or output directory (`fs` feature)
- Page size and layout configuration
- Built-in themes with optional JSON overrides
- Backend selection (`Minimal` default, `Typst` when feature-enabled)
- Feature-gated Mermaid and math support with explicit fallback behavior

## Feature policy

- Features must remain honest and test-backed.
- Unsupported syntax is never silently dropped.
- Fallback output should preserve user intent visibly.

## Resource resolution behavior

- Path-based documents resolve relative assets from the source directory.
- In-memory sources rely on caller-injected resource resolvers.

## Documentation policy

- Public-facing docs remain in English.
- README is authoritative for user-facing capability and limits.
- Supporting specs and planning docs should match implemented behavior.
