# Promptify — Agent & Contributor Guidelines

## Project Overview

**Promptify** is a local-first AI firewall that intercepts prompts and responses
between a client and a local LLM server (ollama / llama.cpp), detecting prompt
injection, sensitive data requests, and encoded payload attacks before they reach
the model.

## Stack

- **Rust** (`axum`, `tokio`, `reqwest`) — core proxy and detection engine.
- **Python** (`FastAPI`) — ML sidecar handling entropy analysis and future ML
  classification.

---

## Architectural Rules

### 1. Follow the existing project structure exactly
Do **not** create new files or modules outside of what is specified in
`ARCHITECTURE.md`. Do not add logic to `main.rs` or `proxy.rs` beyond
wiring / orchestration.

### 2. Reuse, never reimplement
If functionality already exists in another module (e.g. rule matching in
`rules/mod.rs`), call it — never reimplement it inline elsewhere.

### 3. Module-level doc comments are mandatory
Add a `//! ...` doc comment at the top of **every new file** explaining:
- What the module **owns**.
- What it explicitly **does not** handle.

### 4. File-size guard
If an existing file would need to grow past **~300 lines** to fit a phase's
logic cleanly, **stop and flag it** instead of proceeding. Do not silently let
a file balloon.

### 5. Never commit data/ artefacts
Nothing under `data/` (SQLite logs or any generated runtime data) should ever
be committed.

### 6. Naming conventions
| Language | Convention |
|----------|------------|
| Rust     | `snake_case` for modules, functions, variables |
| Python   | `snake_case` for modules, functions, variables |

Reuse the existing vocabulary — do **not** invent synonyms for concepts that
already have names in the codebase:

| Canonical name     | Purpose                                      |
|--------------------|----------------------------------------------|
| `RuleEngine`       | Orchestrates all rule evaluation              |
| `DecoderEngine`    | Handles encoded-payload detection/decoding    |
| `ScoringEngine`    | Aggregates signal scores into a final verdict |
| `Decision`         | Typed outcome returned by the pipeline        |
| `Explanation`      | Human-readable rationale attached to Decision |
| `BackendAdapter`   | Abstraction over the upstream LLM server      |

---

## Quick-Reference Checklist

Before opening a PR / committing work, verify:

- [ ] No new files or modules created outside `ARCHITECTURE.md` scope.
- [ ] No logic added directly to `main.rs` or `proxy.rs` (only wiring).
- [ ] Every new `.rs` or `.py` file has a module-level doc comment.
- [ ] No file exceeds ~300 lines — flagged and split if needed.
- [ ] `data/` directory is listed in `.gitignore` and nothing under it is staged.
- [ ] No synonym types or structs invented — existing vocabulary used throughout.
