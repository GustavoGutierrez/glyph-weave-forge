# Practical GPT Model Allocation for SDD in OpenCode

This note captures a practical way to assign models to Spec-Driven Development
(SDD) phases while balancing quality and token cost.

## Layered model strategy

- **High reasoning**: architecture and cross-cutting decisions
- **Medium reasoning**: structure, decomposition, and quality checks
- **Execution-focused**: implementation-heavy coding work

## Recommended baseline profile

Use this profile for most production work:

```text
sdd-orchestrator -> high-reasoning fast model
sdd-explore      -> high-reasoning fast model
sdd-propose      -> high-reasoning fast model
sdd-spec         -> medium-cost structured model
sdd-design       -> high-reasoning full model
sdd-tasks        -> medium-cost fast model
sdd-apply        -> code-optimized model
sdd-verify       -> medium-cost structured model
sdd-archive      -> medium-cost fast model
```

## Escalation policy

1. Start with the baseline profile.
2. If outputs are insufficient, escalate only the bottleneck phase.
3. De-escalate to cheaper profiles for repetitive or low-risk work.

## Working rules

1. Avoid premium reasoning models for long routine code generation.
2. Prefer code-optimized models for implementation tasks.
3. Keep prompts focused on relevant files and constraints.
4. Split large tasks into smaller batches to improve reliability.
5. Escalate model power only when needed.

## Summary

The best default is a **mixed profile**: strong reasoning for design/orchestration,
efficient structured models for specs/tasks, and a code-optimized model for apply.
