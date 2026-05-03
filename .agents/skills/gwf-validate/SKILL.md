---
name: gwf-validate
description: >
  Pre-release security and hygiene validation for glyph-weave-forge.
  Scans for secrets, local paths, PII, debug artifacts, and packaging issues
  before a release or commit. Acts as a mandatory gate for gwf-release and
  can be run manually. Validates only what THIS repository ships.
  Trigger: When user says "validate", "validar", "pre-release check",
  "security scan", "revisar antes de release", "analisis de seguridad",
  "/gwf-validate", or when gwf-release invokes it as a pre-publish gate.
license: Apache-2.0
metadata:
  author: gentleman-programming
  version: "2.0.0"
allowed-tools: Bash(grep:*) Bash(cargo:*) Bash(git:*) Bash(find:*) Bash(ls:*) Bash(wc:*) Bash(sed:*) Bash(head:*) Bash(cut:*) Bash(which:*) Read Write Edit Glob Grep
---

# GWF Validate — Pre-Release Security & Hygiene Scanner

## When to Use

- Before running `gwf-release` (invoked automatically by the release skill as a gate)
- User explicitly asks "validate", "validar", "security scan", "revisar antes de subir"
- As part of a pre-commit hook to catch issues early
- Before `cargo publish` to ensure published crate is clean

## Severity Levels

| Level | Meaning | Action |
|-------|---------|--------|
| **CRITICAL** | Secrets, keys, tokens, credentials exposed | **BLOCK release.** Must be fixed before proceeding. |
| **WARNING** | Local paths, debug artifacts, hygiene issues | **Recommend fix.** Release proceeds but with notice. |
| **INFO** | Minor concerns, best-practice suggestions | **Log only.** No action required. |

---

## Tool Detection (ALWAYS run first)

Before running any scan, detect available tools:

```bash
# Check for ripgrep (preferred: faster, better filtering)
if command -v rg >/dev/null 2>&1; then
    echo "TOOL=rg"   # use rg commands below
else
    echo "TOOL=grep"  # use grep fallback commands below
fi

# Always check cargo
cargo --version >/dev/null 2>&1 || echo "ERROR: cargo not found"
```

**If `rg` is available**: use the `rg` variant commands (they're simpler and faster).
**If only `grep` is available**: use the `grep` fallback variants (always work, no install needed).

To install `rg` if desired:
```bash
cargo install ripgrep
# or: sudo apt install ripgrep
# or: brew install ripgrep
```

---

## Modes

### Mode 1: Pre-Release Gate (used by gwf-release)

Run all checks on files that WILL be published (files in Cargo.toml `include` list).
If any CRITICAL finding → **STOP the release**.

Returns structured result:
```
status: pass | fail
critical_count: N
warning_count: N
info_count: N
findings: [ { file, line, level, category, detail, suggestion } ]
```

### Mode 2: Manual Validation (standalone)

User runs `/gwf-validate` manually. Same checks, but reports interactively.
User can decide what to fix.

### Mode 3: Quick Pre-Commit Scan

Scan ONLY staged changes (`git diff --cached --name-only`).
Report CRITICAL findings only (warnings are logged but don't block commit).

---

## Publishable Files Scope

This crate's `Cargo.toml` `include` list defines what gets published:

```
include = [
    "/Cargo.toml",
    "/LICENSE",
    "/README.md",
    "/logo.png",
    "/src/**",
    "/tests/**",
]
```

**Phase 1** (CRITICAL) scans ALL workspace files — secrets anywhere are dangerous.
**Phases 2-4** (WARNING) scan ONLY publishable paths: `src/`, `tests/`, `README.md`, `LICENSE`.
**Phase 5** (INFO) scans `Cargo.toml` and `README.md`.

---

## Phase 1: Secrets & Credentials (CRITICAL)

Scope: ALL files in workspace (exclude `.git/` and `target/`).

### 1.1 API Keys & Access Tokens

**rg variant:**
```bash
# OpenAI keys (sk-...), GitHub tokens (ghp_...), HuggingFace (hf_...), Slack (xoxb-...)
rg -n --no-heading 'sk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{36,}|hf_[a-zA-Z0-9]{20,}|xox[bprs]-[a-zA-Z0-9-]+' \
    --glob '!.git' --glob '!target' --glob '!*.lock' .
```

**grep fallback:**
```bash
# Find files containing token patterns
grep -rEl --include='*.rs' --include='*.toml' --include='*.md' --include='*.json' --include='*.yml' \
    --exclude-dir='.git' --exclude-dir='target' \
    -E 'sk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{36,}|hf_[a-zA-Z0-9]{20,}|xox[bprs]-[a-zA-Z0-9-]+' . 2>/dev/null
# If hits found, show with line numbers:
grep -rIn --include='*.rs' --include='*.toml' --include='*.md' --include='*.json' --include='*.yml' \
    --exclude-dir='.git' --exclude-dir='target' \
    -E 'sk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{36,}|hf_[a-zA-Z0-9]{20,}|xox[bprs]-[a-zA-Z0-9-]+' . 2>/dev/null
```

### 1.2 Private Keys (PEM format)

**rg variant:**
```bash
rg -n --no-heading '-----BEGIN.*PRIVATE KEY-----' --glob '!.git' --glob '!target' .
```

**grep fallback:**
```bash
grep -rIn --exclude-dir='.git' --exclude-dir='target' -E 'BEGIN.*PRIVATE KEY' . 2>/dev/null
```

### 1.3 JWT Tokens

**rg variant:**
```bash
rg -n --no-heading 'eyJ[A-Za-z0-9_=-]{20,}\.[A-Za-z0-9_=-]{20,}\.[A-Za-z0-9_=-]{20,}' \
    --glob '!.git' --glob '!target' .
```

**grep fallback:**
```bash
grep -rIn --exclude-dir='.git' --exclude-dir='target' \
    -E 'eyJ[A-Za-z0-9_=-]{20,}\.[A-Za-z0-9_=-]{20,}' . 2>/dev/null
```

### 1.4 SSH Key References

**rg variant:**
```bash
rg -n --no-heading 'id_rsa|id_ecdsa|id_ed25519|ssh[_-]?key' -i \
    --glob '!.git' --glob '!target' .
```

**grep fallback:**
```bash
grep -rIn --exclude-dir='.git' --exclude-dir='target' \
    -iE 'id_rsa|id_ecdsa|id_ed25519|ssh[_-]?key' . 2>/dev/null
```

### 1.5 Bare `.env` Files

```bash
find . -name '.env' \
    -not -name '*.example' \
    -not -name '*.template' \
    -not -name '*.sample' \
    -not -name '*.dist' \
    -not -path './.git/*' \
    -not -path './target/*' 2>/dev/null
```

If a bare `.env` file exists with content → **CRITICAL**.

### 1.6 Password/Credential Assignments

Checks for password-like variables with non-trivial values:

**grep (both tools):**
```bash
grep -rIn --exclude-dir='.git' --exclude-dir='target' \
    -iE 'password|passwd|credential' . 2>/dev/null | \
    grep -v 'password = ""' | \
    grep -v 'password = "test"' | \
    grep -v 'password.*example' | \
    grep -v '#\[cfg(test)\]' | \
    grep -v 'mod tests'
```

**Exception**: `password = ""` and `password = "test"` in test files are NOT flagged.

---

## Phase 2: Local Paths & Machine References (WARNING)

Scope: ONLY publishable files (`src/`, `tests/`, `README.md`, `LICENSE`).

### 2.1 Home Directory Paths

**rg variant:**
```bash
rg -n '/home/[a-zA-Z][^/]*/' src/ tests/ README.md 2>/dev/null
rg -n '/Users/[a-zA-Z][^/]*/' src/ tests/ README.md 2>/dev/null
```

**grep fallback:**
```bash
grep -rIn -E '/home/[a-zA-Z][^/[:space:]"]*/' src/ tests/ README.md 2>/dev/null
```

### 2.2 Tool Config Directories

Checks for references to local tool configs (IDE, shell, etc.):

**rg variant:**
```bash
rg -n '\.config/opencode|\.config/code|\.vscode|\.idea|\.cargo' src/ tests/ README.md 2>/dev/null
```

**grep fallback:**
```bash
grep -rIn -E '\.config/(opencode|code)|\.vscode|\.idea|\.cargo' src/ tests/ README.md 2>/dev/null
```

### 2.3 Local Username Detection

Extract any OS-level usernames from publishable files:

```bash
# Compare against actual OS user
WHOAMI=$(whoami)
grep -rIn "$WHOAMI" src/ tests/ README.md 2>/dev/null
```

If the local username (`whoami`) appears in publishable files → **WARNING**.
On this project, `tests/api_smoke.rs` contains `"meridian"` as a test fixture for path sanitization.

### 2.4 Absolute Paths in Docs

```bash
grep -rIn -E '/(home|Users|tmp|etc|opt|var|root)/[a-zA-Z]' README.md 2>/dev/null
```

---

## Phase 3: File Hygiene (WARNING)

### 3.1 Editor Artifacts

```bash
find . \( -name '*.swp' -o -name '*.swo' -o -name '*~' -o -name '.DS_Store' -o -name 'Thumbs.db' -o -name '*.bak' \) \
    -not -path './.git/*' -not -path './target/*' 2>/dev/null
```

If any found → WARNING.

### 3.2 Cargo Package Audit

```bash
cargo package --list --allow-dirty 2>/dev/null
```

Review that the published list does NOT include:
- `.env`, `.env.*` files
- Editor artifacts
- CI config with embedded secrets
- `node_modules`, `__pycache__`
- Non-project directories

### 3.3 `.gitignore` Coverage

```bash
echo "=== .gitignore audit ==="
for pattern in '.env' '*.swp' '.DS_Store' 'Thumbs.db'; do
    if grep -qF "$pattern" .gitignore 2>/dev/null; then
        echo "  OK: $pattern"
    else
        echo "  MISSING: $pattern should be in .gitignore"
    fi
done
```

Auto-fix: append missing patterns to `.gitignore`.

---

## Phase 4: Code Hygiene (WARNING)

Scope: `src/` directory only (library code that ships).

### 4.1 Debug Artifacts in Library Code

Checks for `println!`, `dbg!`, `eprintln!` outside of test code:

**rg variant:**
```bash
rg -n 'println!|dbg!|eprintln!' src/ --glob '!*tests*' --glob '!*test*' --glob '!*mod_test*' 2>/dev/null
```

**grep fallback:**
```bash
grep -rIn -E 'println!|dbg!|eprintln!' src/ --include='*.rs' 2>/dev/null | \
    grep -v '#\[cfg(test)\]' | grep -v 'mod tests' | grep -v 'tests.rs'
```

### 4.2 `.expect()` in Production Code

`.expect()` panics on failure — dangerous in a library. `.unwrap_or()` is safe (has fallback).

**rg variant:**
```bash
rg -n '\.expect\(' src/ --glob '!*tests*' --glob '!*test*' --glob '!*mod_test*' 2>/dev/null
```

**grep fallback:**
```bash
grep -rIn '\.expect(' src/ --include='*.rs' 2>/dev/null | \
    grep -v '#\[cfg(test)\]' | grep -v 'mod tests' | grep -v 'tests.rs'
```

**Exception**: `.expect()` in `#[cfg(test)]` modules is fine.
**Note**: `.unwrap_or()` / `.unwrap_or_else()` are safe (they have defaults) — do NOT flag these.

### 4.3 Panic-Inducing Patterns

```bash
grep -rIn -E 'panic!|todo!|unreachable!' src/ --include='*.rs' 2>/dev/null | \
    grep -v '#\[cfg(test)\]' | grep -v 'mod tests'
```

---

## Phase 5: Package Metadata (INFO)

### 5.1 Cargo.toml Validation

```bash
echo "=== Package Metadata ==="
grep -E '^(name|version|license|repository|homepage|authors|include)' Cargo.toml

echo ""
echo "=== Repository is remote? ==="
grep -c 'https://github.com' Cargo.toml >/dev/null && echo "OK: remote URL" || echo "WARN: not a remote URL"

echo ""
echo "=== License ==="
grep 'license' Cargo.toml | grep -q 'MIT' && echo "OK: MIT" || echo "INFO: check license"
```

### 5.2 Feature Flags vs Documentation Honesty

```bash
echo "=== Feature flags that exist but may be mock/partial ==="
for feat in mermaid math; do
    if grep -q "^${feat} =" Cargo.toml 2>/dev/null; then
        echo "Feature '$feat' exists — check README for honesty:"
        grep -n "not.*claim.*$feat\|$feat.*subset\|$feat.*fallback\|$feat.*mock" README.md 2>/dev/null || \
            echo "  WARN: no limitation notice found for '$feat' in README"
    fi
done
```

### 5.3 Public vs Non-Public File Audit

```bash
echo "=== Files that exist but are NOT in Cargo.toml include ==="
# List all tracked files, compare against include globs
git ls-files | while read f; do
    case "$f" in
        Cargo.toml|LICENSE|README.md|logo.png|src/*|tests/*) ;;
        .gitignore|.gitattributes|*.md) ;;
        *) echo "  $f (not in include list — safe, won't be published)" ;;
    esac
done
```

---

## Execution Flow

### As a Pre-Release Gate (called by gwf-release)

```
0. Detect tools (rg vs grep)
1. Phase 1 — Secrets (CRITICAL) → if found → FAIL, STOP
2. Phase 2 — Local Paths (WARNING) → collect
3. Phase 3 — File Hygiene (WARNING) → collect
4. Phase 4 — Code Hygiene (WARNING) → collect
5. Phase 5 — Metadata (INFO) → collect
6. Return structured report
```

If `status: fail`, the release skill MUST NOT proceed.

### As a Standalone Validator

```
0. Tool detection
1. Run all 5 phases
2. Show findings grouped by severity
3. For each finding → file, line, what, suggested fix
4. Summary table
```

### As a Quick Pre-Commit

```
0. git diff --cached --name-only
1. Phase 1 (secrets) on staged files only
2. If CRITICAL → ask user "Block commit?"
```

---

## Integration with gwf-release

The `gwf-release` skill invokes `gwf-validate` at **Step 2.5** (between version verify and cargo checks):

```
Step 2: Verify version
Step 2.5: Run gwf-validate
  → CRITICAL found → STOP, show findings, suggest fixes
  → Only WARNINGS → show summary, ask "Continue?"
  → Clean → proceed
Step 3: Cargo validation checks
```

---

## Auto-Fix Capabilities

| Finding | Auto-Fix |
|---------|----------|
| Missing `.gitignore` patterns | Append to `.gitignore` |
| `.DS_Store` / editor swap files | Add to `.gitignore`, `git rm --cached` |
| `println!` in library code | Replace with `log::debug!` or remove |
| Local username in test fixtures | Replace with generic placeholder like `"localuser"` |

**CRITICAL findings are NEVER auto-fixed** — always require human review.

---

## Known False Positives (Project-Specific)

These patterns are expected and MUST NOT be flagged:

| File / Pattern | Why It's OK |
|----------------|-------------|
| `Cargo.toml`: `repository = "https://github.com/GustavoGutierrez/..."` | Official public repo URL |
| `Cargo.toml`: `authors = ["Gustavo Gutiérrez ..."]` | Intentionally public |
| `src/adapters/render/typst.rs`: `/usr/share/fonts/` | System font paths (not personal) |
| Test files: `password = ""` or `password = "test"` | Fake test fixtures |
| Any file: `.unwrap_or()` / `.unwrap_or_else()` | Has fallback, never panics |
| `src/math.rs`: `.expect("conversion should succeed")` in test functions | Inside `#[cfg(test)]` |
| `src/adapters/render/typst.rs`: `.expect(...)` in test functions | Inside `#[cfg(test)]` |
| `tests/api_smoke.rs`: `".config/opencode"` and `"meridian"` | Intentional fixtures for path-sanitization tests — **but should be reviewed for replacement with generic placeholders** |

---

## Current Project Baseline (known issues)

After running gwf-validate on this codebase (2026-05-03):

### ✅ Phase 1 (CRITICAL): CLEAN
No API keys, tokens, private keys, JWTs, SSH keys, `.env` files, or real passwords found.

### ⚠️ Phase 2 (WARNING): 1 finding
| File | Line | Detail | Suggestion |
|------|------|--------|------------|
| `tests/api_smoke.rs` | 179, 181 | Local username `meridian` and `.config/opencode` in test fixture | Intentional (path sanitization test), but consider replacing `"meridian"` with `"localuser"` |

### ⚠️ Phase 3 (WARNING): RESOLVED
`.gitignore` was missing `.env`, `*.swp`, `.DS_Store` — **FIXED**.

### ⚠️ Phase 4 (WARNING): 1 finding
| File | Line | Detail | Suggestion |
|------|------|--------|------------|
| `src/adapters/markdown.rs` | 459 | `.expect("table state should exist")` in production code | Replace with proper error handling (`ok_or` + `?`) instead of panicking |

### ℹ️ Phase 5 (INFO): CLEAN
- Package metadata uses proper remote URL, MIT license
- README honestly documents mermaid/math limitations (lines 296-297)

---

## Commands

### Run Full Validation

```bash
# Step 0: Tool detection
if command -v rg >/dev/null 2>&1; then TOOL=rg; else TOOL=grep; fi; echo "Using: $TOOL"

# Phase 1: Secrets
echo "=== Phase 1: Secrets ==="
if [ "$TOOL" = "rg" ]; then
    rg -n 'sk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{36,}' --glob '!.git' --glob '!target' . || echo "  clean"
    rg -n '-----BEGIN.*PRIVATE KEY-----' --glob '!.git' --glob '!target' . || echo "  clean"
else
    grep -rIn -E 'sk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{36,}' --exclude-dir='.git' --exclude-dir='target' . || echo "  clean"
    grep -rIn -E 'BEGIN.*PRIVATE KEY' --exclude-dir='.git' --exclude-dir='target' . || echo "  clean"
fi
find . -name '.env' -not -name '*.example' -not -name '*.template' -not -path './.git/*' -not -path './target/*' || echo "  no .env files"

# Phase 2: Local paths (publishable files only)
echo "=== Phase 2: Local Paths ==="
grep -rIn -E '/home/[a-zA-Z][^/]*/' src/ tests/ README.md || echo "  clean"
WHOAMI=$(whoami); grep -rIn "$WHOAMI" src/ tests/ README.md || echo "  no username leak"

# Phase 3: File hygiene
echo "=== Phase 3: File Hygiene ==="
find . \( -name '*.swp' -o -name '.DS_Store' \) -not -path './.git/*' -not -path './target/*' || echo "  clean"
cargo package --list --allow-dirty 2>/dev/null | head -5

# Phase 4: Code hygiene
echo "=== Phase 4: Code Hygiene ==="
grep -rIn '\.expect(' src/ --include='*.rs' | grep -v '#\[cfg(test)\]' | grep -v 'mod tests' || echo "  no panicking expects"
grep -rIn -E 'println!|dbg!' src/ --include='*.rs' | grep -v '#\[cfg(test)\]' || echo "  no debug prints"

# Phase 5: Metadata
echo "=== Phase 5: Metadata ==="
grep -E 'license|repository|include' Cargo.toml
```

### Quick Pre-Commit Scan

```bash
git diff --cached --name-only | while read f; do
    [ -z "$f" ] && continue
    grep -Hn -E 'sk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{36,}|BEGIN.*PRIVATE KEY' "$f" 2>/dev/null
done
```

---

## Resources

- **Project AGENTS.md**: `/AGENTS.md` — security and publication hygiene rules for this crate
- **Cargo.toml**: `include` list defines what gets published
- **gwf-release skill**: invokes gwf-validate at Step 2.5

Base directory for this skill: file:///home/meridian/Documentos/Proyectos/Personal/dev-forge/glyph-weave-forge/.agents/skills/gwf-validate
