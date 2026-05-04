# Engineering a Reliable Markdown-to-PDF Pipeline with Mermaid Diagrams

## Abstract

This article describes how a Rust crate can convert Markdown into production-ready PDFs while preserving clear architecture boundaries, explicit fallback behavior, and deterministic output. We use Mermaid diagrams to document the internal flow and implementation strategy.

## 1. Pipeline Overview

```mermaid
flowchart LR
    A[Markdown Source] --> B[Markdown Parser]
    B --> C[Core Document AST]
    C --> D[Pipeline Transform]
    D --> E[Render Backend]
    E --> F[PDF Output]
```

The pipeline is intentionally segmented (`api -> pipeline -> core -> adapters`) so each layer can evolve independently.

## 2. Conversion Sequence

```mermaid
sequenceDiagram
    participant U as User
    participant F as Forge API
    participant P as Pipeline
    participant R as Renderer
    U->>F: configure source/output/theme/backend
    F->>P: convert(options)
    P->>P: parse markdown
    P->>R: render(document, request)
    R-->>P: pdf bytes
    P-->>F: PdfOutput
```

The API layer remains stable while renderer internals can be swapped between Minimal and Typst.

## 3. Mermaid Handling State Model

```mermaid
stateDiagram-v2
    [*] --> MermaidFenceDetected
    MermaidFenceDetected --> FeatureDisabled: mermaid feature off
    MermaidFenceDetected --> RenderWithRust: mermaid feature on
    RenderWithRust --> EmbeddedDiagram: subset syntax supported
    RenderWithRust --> VisibleFallback: unsupported syntax
    FeatureDisabled --> VisibleFallback
    EmbeddedDiagram --> [*]
    VisibleFallback --> [*]
```

This model avoids silent data loss and keeps fallback output explicit.

## 4. Core Type Relationships

```mermaid
classDiagram
    class Document {
      +blocks: Vec~Block~
    }
    class Block {
      <<enum>>
      +Heading
      +Paragraph
      +Code
      +Mermaid
      +Math
      +Image
    }
    class RenderBackend {
      <<trait>>
      +render(document, request) Result~Vec~u8~~
    }
    Document --> Block
```

By elevating Mermaid to a dedicated block variant, downstream renderers can make feature-aware decisions cleanly.

## 5. Data Perspective for Artifacts

```mermaid
erDiagram
    CONVERSION_REQUEST ||--o{ RENDER_ASSET : contains
    CONVERSION_REQUEST ||--|| DOCUMENT_AST : produces
    DOCUMENT_AST ||--o{ BLOCK_NODE : includes
    BLOCK_NODE ||--o| MERMAID_ARTIFACT : may_render
```

The artifact relation helps reason about caching and lifecycle of rendered diagram assets.

## 6. Release Flow for Diagram Support

```mermaid
gitGraph
   commit id: "baseline"
   branch feature/mermaid
   checkout feature/mermaid
   commit id: "parse mermaid blocks"
   commit id: "typst rust-native renderer"
   commit id: "tests + docs"
   checkout main
   merge feature/mermaid
```

This workflow keeps implementation changes reviewable and test-backed.

## 7. Delivery Plan Snapshot

```mermaid
gantt
    title Mermaid Implementation Plan
    dateFormat  YYYY-MM-DD
    section Parser
    Add Mermaid AST block         :done, p1, 2026-05-01, 1d
    section Rendering
    Add Typst Rust renderer       :active, p2, 2026-05-02, 2d
    Add per-session cache         :p3, after p2, 1d
    section Validation
    Tests and PDF fixture run     :p4, after p3, 1d
```

## 8. Conclusion

A robust Markdown-to-PDF library should optimize for correctness first: explicit AST nodes, isolated rendering adapters, feature-gated behavior, and transparent fallback paths. With this approach, Mermaid support can be both practical and maintainable in real software delivery.
