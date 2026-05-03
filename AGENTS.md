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

---

## Release Rules

### Semantic Versioning (MANDATORY)

Every release MUST follow [SemVer 2.0](https://semver.org/). The `gwf-release` skill enforces this automatically:

| Bump level | Trigger | Example |
|------------|---------|---------|
| **MAJOR** | Breaking change (`!:` in conventional commit, or `feat!:`/`fix!:`), API incompatibility | `0.1.4` → `1.0.0` |
| **MINOR** | New feature (`feat:` commit, no breaking changes), new public API | `0.1.4` → `0.2.0` |
| **PATCH** | Bug fix (`fix:`), docs (`docs:`), refactor (`refactor:`), perf (`perf:`), test (`test:`), chore (`chore:`) | `0.1.4` → `0.1.5` |

**Version files that MUST stay in sync** — the release skill verifies all of these:
- `Cargo.toml` → `version = "X.Y.Z"`
- `README.md` → `glyphweaveforge = "X.Y.Z"` and `version = "X.Y.Z"`
- `docs/integration-guide.md` → `glyphweaveforge = "X.Y.Z"` and `version = "X.Y.Z"`

### Security Validation (AUTOMATIC GATE)

Before ANY release, `gwf-release` automatically invokes `gwf-validate` as a pre-publish gate:

1. **Phase 1 — Secrets & Credentials** (CRITICAL): API keys, tokens, private keys, passwords, `.env` files
2. **Phase 2 — Local Paths** (WARNING): absolute home paths, usernames, tool config dirs
3. **Phase 3 — File Hygiene** (WARNING): editor artifacts, `.gitignore` coverage, Cargo include audit
4. **Phase 4 — Code Hygiene** (WARNING): `println!`/`dbg!` in lib code, `.expect()` panicking in production
5. **Phase 5 — Metadata** (INFO): license, repo URL, feature honesty in docs

- If any **CRITICAL** finding → **release is BLOCKED**. Must fix before proceeding.
- If only **WARNING/INFO** → summary shown, user decides to continue.
- You can run validation manually anytime: `/gwf-validate` or say "validate"/"validar"/"analisis de seguridad".

### Pre-Release Checks

Before release-oriented changes are considered done, all of these MUST pass:

```bash
cargo fmt --check
cargo check
cargo test
cargo test --all-features
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo publish --dry-run
```

---

## License

- Use `license = "MIT"` in `Cargo.toml` for this project unless the user explicitly requests a different SPDX license.

---

## Available Skills

### Project-Specific Skills (`.agents/skills/`)

| Skill | When to use | Trigger keywords |
|-------|-------------|------------------|
| **gwf-release** | Semantic version release: bump version, validate, commit, tag, push, GitHub Release with artifact. Also handles commit+push without release. | "release", "publish", "new version", "bump", "deploy", "lanzar", "publicar", "nueva versión", "commit and push" |
| **gwf-validate** | Pre-release security and hygiene scan: secrets, local paths, editor artifacts, code hygiene, metadata. Runs automatically as gate in gwf-release. Can run standalone. | "validate", "validar", "security scan", "analisis de seguridad", "pre-release check", "revisar antes de release" |
| **architecture-patterns** | Designing layered architecture, refactoring module boundaries, implementing Clean/Hexagonal/DDD patterns, debugging dependency cycles. | "architecture", "clean architecture", "hexagonal", "DDD", "dependency", "layers" |
| **mermaid** | Creating flowcharts, sequence diagrams, state machines, class diagrams, Gantt charts in ` ```mermaid ` fenced blocks. | "diagram", "flowchart", "sequence", "mermaid", "Gantt" |
| **rust-best-practices** | Writing idiomatic Rust: borrowing vs cloning, error handling, performance, testing, documentation, clippy. | "rust", "borrow", "clone", "Result", "error handling", "test", "clippy", "performance" |
| **writing-specs-designs** | Building spec + design doc pack for a feature/UX change: diagram, flows, acceptance criteria. | "spec", "design doc", "feature spec", "technical spec", "acceptance criteria" |

### System Skills (OpenCode built-in)

| Skill | When to use | Trigger keywords |
|-------|-------------|------------------|
| **skill-creator** | Creating or modifying agent skills. | "create skill", "new skill", "modify skill", "skill creator" |
| **branch-pr** | Creating pull requests following issue-first workflow. | "pull request", "PR", "create PR" |
| **issue-creation** | Creating GitHub issues with proper format. | "create issue", "report bug", "feature request" |
| **judgment-day** | Parallel adversarial review by two independent sub-agents, with auto-fix until both pass. | "judgment day", "review adversarial", "dual review", "juzgar" |
| **find-skills** | Discovering installable skills for new capabilities. | "how do I do X", "find a skill", "is there a skill" |
| **release** | Semantic release for canopy-ruler (different project). | — (not for glyphweaveforge) |
| **go-testing** | Go/Bubbletea TUI testing patterns. | — (not for this Rust project) |

### Skill Discovery

- Project-specific skills are auto-discovered from `.agents/skills/<name>/SKILL.md`.
- System skills are auto-discovered from `~/.config/opencode/skills/` and `~/.agents/skills/`.
- Run `/gwf-validate` to invoke the validation skill directly.
- Run `gwf-release` to invoke the release skill which chains validation automatically.
