# AGENTS.md

Project guidance for automated contributors working on `glyphweaveforge`.

## Scope

- This is a Rust library crate for Markdown-to-PDF conversion.
- Public-facing crate documentation must stay in **English**.
- User-facing release instructions may be written in Spanish when explicitly requested.

## Engineering standards

- Preserve the internal boundary: `api -> pipeline -> core -> adapters`.
- Prefer small, test-backed changes over broad rewrites.
- Keep feature flags honest; do not claim real Mermaid or math rendering unless it is actually implemented.
- Maintain compatibility of the public builder API unless a breaking change is explicitly requested.

## Security and publication hygiene

- Never publish secrets, tokens, credentials, private keys, or local environment details.
- Do not add absolute filesystem paths, local usernames, machine-specific directories, or editor cache output to public docs, tests, examples, or packaged sources.
- Review `Cargo.toml` package metadata and the package include list before release changes.
- Keep the published crate focused on source, tests, README, license, and essential assets only.
- Avoid examples that expose local paths such as `/home/...`, `/Users/...`, `C:\...`, or tool-specific config directories.

## Documentation rules

- Keep `README.md` suitable for GitHub, crates.io, and docs.rs.
- Keep crate docs in sync with `README.md` through `src/lib.rs`.
- Document limitations explicitly rather than implying unsupported capability.

## Validation expectations

Before release-oriented changes are considered done, run the relevant checks:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo test --all-features`
- `cargo test --doc`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo publish --dry-run`

## Release note

- Prefer `license = "Apache-2.0"` in `Cargo.toml` for this project unless the user explicitly requests a different SPDX license.
