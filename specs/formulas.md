# Math formula support reference

GlyphWeaveForge handles math through Markdown math parsing and renderer-specific
output behavior.

## Parsing model

Math is parsed from Markdown using GFM-style delimiters:

- Inline math: `$...$`
- Display math: `$$...$$`

The parser emits dedicated math events, which are mapped to explicit math nodes
in the internal document model.

## Rendering model

- **Typst backend**: renders a documented TeX subset as real Typst math.
- **Minimal backend**: emits readable, explicit math fallbacks.

## Supported TeX subset (Typst path)

- Fractions and roots: `\frac`, `\sqrt`
- Super/subscripts: `x^2`, `a_i`, grouped forms
- Operators and symbols: `\sum`, `\prod`, `\int`, `\lim`, `\pm`, `\infty`,
  `\approx`, `\neq`, `\leq`, `\geq`, `\to`, `\cdot`, `\times`
- Common Greek letters such as `\alpha`, `\beta`, `\gamma`, `\delta`, `\pi`,
  `\lambda`, `\sigma`, `\Omega`, `\Delta`

## Unsupported constructs

Unsupported LaTeX environments/macros are not silently ignored. They are surfaced
as explicit conversion notices or fallback output, depending on backend behavior.

Examples include:

- `\begin{...}` / `\end{...}` environments
- custom package macros and alignment environments

## User-facing contract

GlyphWeaveForge is **not** a full LaTeX engine. It supports a practical subset for
common Markdown math workflows and keeps unsupported input visible and explicit.
