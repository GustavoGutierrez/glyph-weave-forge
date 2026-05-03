---
name: gwf-release
description: >
  Semantic version release workflow for glyph-weave-forge. Handles version bumping across ALL files,
  runs full pre-release validation, generates release notes from conventional commits, creates git tags,
  commits, pushes, and creates GitHub Release with packaged crate artifact.
  Trigger: When user says "release", "publish", "new version", "bump version", "release v", "deploy",
  "push", "commit changes", "create release", "new release", "lanzar", "publicar", "nueva versión".
license: Apache-2.0
metadata:
  author: gentleman-programming
  version: "1.0"
---

## When to Use

- User asks to release a new version of glyph-weave-forge
- User asks to bump the version
- User asks to push changes or publish
- User mentions "release vX.Y.Z", "nueva versión", "publicar"
- User wants a simple commit+push without a full release

## Two Modes

| Mode | Trigger words | What it does |
|------|--------------|--------------|
| **Release** | "release", "publish", "new version", "bump", "deploy", "lanzar", "publicar" | Full version bump + validation + commit + tag + push + GitHub Release |
| **Commit only** | "commit", "push changes", "just push", "commit and push", "subir cambios" | Git add + commit + push only (no version bump) |

**Always ask** which mode the user wants if ambiguous. Default to **Release** if they mention a version number.

---

## Release Workflow (Step by Step)

### Step 0: Check GitHub Auth

```bash
gh auth status
```

If not authenticated, instruct the user to run `gh auth login` first and STOP.

---

### Step 1: Determine new version

Read the current version from `Cargo.toml` → `[package] version` field.

Look at commits since the last version tag to auto-detect the bump level:
```bash
git log --oneline $(git describe --tags --abbrev=0 2>/dev/null)..HEAD
```

Follow **Semantic Versioning** based on conventional commit prefixes:
- **MAJOR** — commits containing `!:` (breaking change), or `feat!:` / `fix!:` etc.
- **MINOR** — commits with `feat:` prefix (new features, new functionality)
- **PATCH** — commits with `fix:`, `docs:`, `style:`, `refactor:`, `perf:`, `test:`, `chore:`, `ci:` (no new features)

**Decision table:**

| Commits contain | Bump |
|-----------------|------|
| Any `!:` breaking change | **MAJOR** |
| Any `feat:` (no breaking changes) | **MINOR** |
| Only `fix:`, `docs:`, `chore:`, etc. | **PATCH** |

**Present the detected bump to the user** with the reasoning. If the user specifies a version explicitly (e.g., "release 0.2.0"), use that instead.

---

### Step 2: Verify version is applied consistently

Before proceeding, scan ALL files that reference the crate version and verify they match after the bump:

| File | What to check | Pattern |
|------|--------------|---------|
| `Cargo.toml` | `version = "X.Y.Z"` | `version = "X.Y.Z"` |
| `README.md` | Dependency examples | `glyphweaveforge = "X.Y.Z"` and `version = "X.Y.Z"` |
| `docs/integration-guide.md` | Dependency examples | `glyphweaveforge = "X.Y.Z"` and `version = "X.Y.Z"` |

**⚠️ README.md and docs/integration-guide.md MUST match the Cargo.toml version. Do NOT skip these.**

Example: if bumping from 0.1.4 to 0.1.5:
- `Cargo.toml` line 3: `version = "0.1.5"`
- `README.md` lines 27, 34: change `"0.1.4"` → `"0.1.5"`
- `docs/integration-guide.md` lines 20, 27, 146: change `"0.1.4"` → `"0.1.5"`

Use `grep -rn '"X\.Y\.Z"' README.md docs/ Cargo.toml --include='*.md' --include='*.toml'` to find ALL occurrences before and after the bump.

---

### Step 2.5: Run Security & Hygiene Validation (MANDATORY GATE)

Before running cargo checks, execute the `gwf-validate` security scan.

**Load the gwf-validate skill** and run it in Pre-Release Gate mode on all files that will be published.

The validation returns a structured result:
```
status: pass | fail
critical_count: N
warning_count: N
info_count: N
findings: [ { file, line, level, category, detail, suggestion } ]
```

**Decision tree:**

| Result | Action |
|--------|--------|
| `status: fail` (CRITICAL found) | **STOP.** Show all CRITICAL findings with file, line, and suggested fixes. Do NOT proceed until fixed. |
| `status: pass`, `warning_count > 0` | Show WARNING summary. Ask user: "Hay WARNINGs. ¿Continuar de todos modos?" / "Warnings found. Proceed anyway?" |
| `status: pass`, clean | Continue to Step 3. |

Examples of what `gwf-validate` catches:
- **CRITICAL**: API keys, tokens, private keys, `.env` files, passwords in source
- **WARNING**: Local paths (`/home/user/...`), debug prints, editor artifacts
- **INFO**: Metadata consistency, documentation limitations

If the release is blocked by CRITICAL findings, report each finding clearly:
```
🚫 Release blocked — CRITICAL findings:

src/config.rs:42 — API key found: api_key = "sk-abc123..."
  → Fix: Move to environment variable or .env (not published)

tests/fixtures/data.json:15 — JWT token found
  → Fix: Use a fake/example token for tests

Fix these issues and run validation again.
```

---

### Step 3: Run pre-release validation (MANDATORY GATE)

All of these MUST pass. If any fail, STOP and report the error to the user.

```bash
cargo fmt --check
```

```bash
cargo check
```

```bash
cargo test
```

```bash
cargo test --all-features
```

```bash
cargo test --doc
```

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

```bash
cargo publish --dry-run
```

If all pass, show ✓ for each and proceed. If any fail, show ✗ with the error and STOP.

---

### Step 4: Generate release notes

Gather commits since the last version tag:
```bash
git log --oneline $(git describe --tags --abbrev=0 2>/dev/null)..HEAD
```

Categorize by conventional commit prefix:
- `feat:` / `feat!:` → **Features**
- `fix:` / `fix!:` → **Fixes**
- `perf:` → **Performance**
- `refactor:` → **Refactoring**
- `docs:` → **Documentation**
- `style:` / `chore:` / `ci:` / `test:` → **Maintenance**

Write release notes in **English** with this format:
```markdown
## Summary
- Feature description 1
- Feature description 2
- Fix description 1
```

If there are many commits in a category, group them:
```markdown
## What's Changed

### Features
- feat 1 description
- feat 2 description

### Fixes
- fix 1 description

### Maintenance
- chore/docs/ci items
```

Save the notes to a temporary file for the GitHub Release step.

---

### Step 5: Build and package the crate artifact

For a Rust library crate, package the source:
```bash
cargo package --allow-dirty
```

This creates `target/package/glyphweaveforge-X.Y.Z.crate`. This is the artifact that gets uploaded to crates.io and attached to the GitHub Release.

Verify the package contents are correct:
```bash
tar -tzf target/package/glyphweaveforge-X.Y.Z.crate
```

Ensure the package includes only the expected files (see `Cargo.toml` `include` list):
- `Cargo.toml`, `LICENSE`, `README.md`, `logo.png`
- `src/**`, `tests/**`
- No editor files, no absolute paths, no secrets

---

### Step 6: Stage, commit, tag, and push

Use `gh` CLI for GitHub operations, `git` for local operations only.

```bash
# Stage ALL changed files
git add Cargo.toml README.md docs/integration-guide.md
git add -A

# Verify what's staged
git status

# Commit with conventional commit format (English)
git commit -m "chore: bump version to X.Y.Z

$(cat /tmp/release-notes.txt)"
```

```bash
# Create annotated tag
git tag -a "vX.Y.Z" -m "vX.Y.Z — $(head -1 /tmp/release-notes.txt)"
```

```bash
# Push code and tag
git push origin main
git push origin "vX.Y.Z"
```

---

### Step 7: Create GitHub Release with artifact

```bash
gh release create "vX.Y.Z" \
    --title "vX.Y.Z" \
    --notes-file /tmp/release-notes.md \
    "target/package/glyphweaveforge-X.Y.Z.crate"
```

This creates the GitHub Release, attaches the release notes, and uploads the `.crate` package artifact.

---

### Step 8: Publish to crates.io (optional — ASK FIRST)

Ask the user: "¿Publicar también en crates.io?" / "Publish to crates.io as well?"

If yes:
```bash
cargo publish
```

⚠️ **WARNING**: `cargo publish` is irreversible. Confirm with the user before executing.

---

### Step 9: Confirmation

After completion, confirm:
- ✓ Commits pushed to `main`
- ✓ Tag `vX.Y.Z` pushed
- ✓ GitHub Release created with `.crate` artifact
- ✓ Release visible at `https://github.com/GustavoGutierrez/glyph-weave-forge/releases`
- ✓ (if published) Crate available at `https://crates.io/crates/glyphweaveforge`

---

## Commit-Only Workflow

When the user just wants to commit and push without a release:

### Step 1: Review changes
```bash
git status
git diff --stat
```

### Step 2: Write commit message in English

Follow [Conventional Commits](https://www.conventionalcommits.org/):
- `feat: description` — new feature
- `fix: description` — bug fix
- `docs: description` — documentation only
- `style: description` — formatting, missing semicolons, etc.
- `refactor: description` — code change that neither fixes a bug nor adds a feature
- `perf: description` — performance improvement
- `test: description` — adding or updating tests
- `chore: description` — build process, tooling, dependencies

### Step 3: Commit and push
```bash
git add -A
git commit -m "type: description"
git push origin main
```

---

## Version History Reference

Current version: **0.1.4**

Previous releases:
- v0.1.3 — themed Markdown tables, HTML img support, integration docs update
- v0.1.2 — typst rendering restored, font discovery
- v0.1.1 — expanded public API coverage
- v0.1.0 — initial release

---

## Repository Details

- **Remote**: `origin` → `git@github.com:GustavoGutierrez/glyph-weave-forge.git`
- **Branch**: `main`
- **Crate name**: `glyphweaveforge`
- **crates.io**: `https://crates.io/crates/glyphweaveforge`
- **docs.rs**: `https://docs.rs/glyphweaveforge`

---

## Important Notes

- **Never** skip validation failures — all cargo checks must pass
- **Always** use `gh` CLI for GitHub Release creation (not raw `git` + API calls)
- **Always** write commit messages and release notes in **English**
- **Always** verify that `README.md` and `docs/integration-guide.md` match the bumped version — these are commonly missed
- **Always** run `cargo publish --dry-run` before `cargo publish` to verify packaging
- **Always** push tags separately with `git push origin vX.Y.Z`
- The `Cargo.toml` `include` list already restricts published files — do not add unnecessary files
- Crates.io publishing is **optional** and requires user confirmation
