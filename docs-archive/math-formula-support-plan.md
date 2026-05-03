# Math Formula Rendering Support Plan

This plan describes how GlyphWeaveForge should render Markdown math formulas in
the most standard and efficient way for the current pipeline.

## Current State

- Markdown parsing uses `pulldown-cmark`, but math parsing is not enabled in the
  parser options yet.
- Display math delimited with `$$` is preprocessed into fenced `math` code blocks
  before parsing.
- Fenced `math` blocks are intentionally rendered as visible unsupported
  fallbacks (`[unsupported:math]`).
- Inline math such as `$E=mc^2$` is currently treated as normal text because the
  parser does not emit math events without `Options::ENABLE_MATH`.
- The Typst backend already has native math support available through Typst math
  mode, but the document AST has no explicit math node yet.

## Recommended Architecture

The correct flow should be:

```text
Markdown
  -> pulldown-cmark with ENABLE_MATH
  -> GlyphWeaveForge AST with explicit math nodes
  -> TeX-subset to Typst math conversion
  -> Typst source
  -> PDF
```

Do not keep converting formulas into code blocks. Math should be represented as
math throughout the internal model so each renderer can make an honest decision:
render real math when supported or emit a clear fallback when unsupported.

## AST Changes

Add explicit math variants:

```rust
pub enum Inline {
    // existing variants...
    Math(String),
}

pub enum Block {
    // existing variants...
    Math { tex: String },
}
```

Rationale:

- Inline math and display math have different layout semantics.
- Display math should not be forced into a paragraph inline list.
- The minimal renderer can degrade gracefully while Typst renders formulas.

## Parser Changes

Enable math events in `PulldownParser`:

```rust
let options = Options::ENABLE_TABLES
    | Options::ENABLE_STRIKETHROUGH
    | Options::ENABLE_TASKLISTS
    | Options::ENABLE_FOOTNOTES
    | Options::ENABLE_MATH;
```

Then map:

- `Event::InlineMath(tex)` -> `Inline::Math(tex.to_string())`
- `Event::DisplayMath(tex)` -> `Block::Math { tex: tex.to_string() }`

Display math can appear in paragraph contexts. The parser should flush any
current paragraph before pushing a display math block, and then resume normal
paragraph handling.

Remove or narrow `preprocess_math_blocks`. Once `ENABLE_MATH` is active, the
preprocessor should not rewrite `$$` blocks into fenced code blocks because that
prevents real math rendering.

## TeX Subset to Typst Conversion

Typst math is not LaTeX. A small converter should support a documented subset
instead of blindly copying arbitrary TeX.

Implement:

```rust
pub(crate) fn latex_to_typst_math(input: &str) -> Result<String>
```

### Phase 1: Safe Subset

Support common Markdown/MathJax-style formulas:

- Variables, numbers, operators: `+`, `-`, `=`, `<`, `>`, `<=`, `>=`
- Superscripts/subscripts: `x^2`, `a_i`, `x^{n+1}`, `a_{ij}`
- Fractions: `\frac{a}{b}` -> `(a)/(b)` or `frac(a, b)` depending on chosen
  Typst style
- Square roots: `\sqrt{x}` -> `sqrt(x)`
- Sums/products/integrals/limits: `\sum`, `\prod`, `\int`, `\lim`
- Greek letters: `\alpha`, `\beta`, `\gamma`, `\delta`, `\pi`, `\Omega`, etc.
- Common symbols: `\pm`, `\infty`, `\approx`, `\neq`, `\leq`, `\geq`, `\to`,
  `\cdot`, `\times`

### Phase 2: Structured Parser

Avoid a long chain of global string replacements for constructs with braces.
Use a small tokenizer/parser for commands with balanced arguments:

- `\frac{numerator}{denominator}`
- `\sqrt{radicand}`
- grouped sub/superscripts (`^{...}`, `_{...}`)

This prevents corrupting nested expressions such as:

```tex
\frac{1}{\sqrt{2\pi\sigma^2}}
```

### Phase 3: Unsupported Constructs

Return a typed error for unsupported LaTeX environments/macros:

- `\begin{...}` / `\end{...}`
- `\left` / `\right` initially, unless translated deliberately
- `\usepackage`, custom macros, alignment environments

Recommended behavior:

- Typst backend: render a visible warning block or return a clear error based on
  the library's chosen strictness.
- Minimal backend: render `[math] <original expression>` text fallback.

## Typst Renderer Changes

Render inline math:

```typst
$converted-expression$
```

Render display math:

```typst
$ converted-expression $
```

Typst uses spaces inside dollar delimiters to make a display-style equation.
The renderer should escape Typst string/markup outside math but should not apply
normal markup escaping inside math expressions after conversion.

## Minimal Renderer Changes

The minimal renderer cannot typeset math. It should remain honest and readable:

- Inline: `[math: E=mc^2]`
- Display: a separate line `[math] E=mc^2`

This preserves content without claiming high-quality math layout.

## Testing Strategy

Add tests at four levels:

1. Parser tests: inline and display math create `Inline::Math` / `Block::Math`.
2. Converter tests: TeX subset converts to valid Typst math syntax.
3. Typst source tests: generated source contains `$...$` inline and `$ ... $`
   display math.
4. Integration tests: `renderer-typst` produces PDF bytes for a document with
   inline and display formulas.

Also keep negative tests for unsupported constructs so behavior stays explicit.

## Documentation Updates

Update README and crate docs to say:

- Math delimiters follow GFM-style `$...$` and `$$...$$`.
- The Typst backend renders a documented TeX subset.
- GlyphWeaveForge is not a full LaTeX engine.
- Unsupported macros/environments produce clear fallback or error output.

## Efficient Implementation Order

1. Add AST variants and parser mapping with `ENABLE_MATH`.
2. Remove display-math-to-code preprocessing.
3. Add minimal renderer fallbacks for new math nodes.
4. Add a small converter for safe TeX subset.
5. Render converted math in Typst.
6. Add integration tests and update docs.

This order keeps every commit testable and avoids breaking the minimal backend
while Typst support is introduced.
