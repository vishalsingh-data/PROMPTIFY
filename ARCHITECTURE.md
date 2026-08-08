# Promptify — Architecture

> **Living document.** Updated at the end of every phase.
> Current state: **Phase 1 — Scaffold** (stubs only, no detection logic).

---

## System Overview

Two processes, communicating over localhost HTTP:

```
┌─────────────────────────────────────────────────────────────┐
│                        Client                               │
│  (any LLM client configured to talk to localhost:11434)     │
└────────────────────────┬────────────────────────────────────┘
                         │ HTTP (thinks it's talking to ollama)
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                  promptify-core  (Rust)                     │
│  axum server bound to :11434                                │
│                                                             │
│  proxy.rs ──► RuleEngine        [rules/mod.rs]              │
│           ──► DecoderEngine     [decoder.rs]                │
│           ──► MlClient          [ml_client.rs]              │
│           ──► ScoringEngine     [scoring.rs]                │
│           ──► Decision/Explain  [decision.rs / explain.rs]  │
│           ──► Logger            [logging.rs]  (async)       │
│           ──► Compressor        [compressor.rs] (if Allow)  │
│           ──► ResponseAnalyzer  [response_analyzer.rs]      │
└────────────────────────┬──────────────────────┬────────────┘
                         │ POST /analyze         │ forward (Allow)
                         ▼                       ▼
         ┌───────────────────────┐   ┌──────────────────────┐
         │  promptify-ml (Python)│   │  Upstream LLM        │
         │  FastAPI :8500        │   │  ollama / llama.cpp  │
         │  entropy.py           │   │  :11435              │
         │  classifier.py (stub) │   └──────────────────────┘
         └───────────────────────┘
```

**Rule of thumb**: Rust owns plumbing and decisions. Python owns intelligence.

---

## Request Lifecycle

```
1.  Client sends prompt to promptify-core (:11434)
2.  proxy.rs receives request
3.  RuleEngine.check(prompt)           → Vec<RuleMatch>
4.  DecoderEngine.decode(prompt)       → Vec<DecodedPayload>
        └─ re-check decoded text via RuleEngine
5.  MlClient.analyze(prompt)           → MlSignal  {entropy, flagged}
6.  ScoringEngine.score(all signals)   → (u8 risk_score, Decision)
7.  build_explanation(signals)         → Explanation
8.  Logger.log_request(record)         (spawned async — never blocks response)
9a. Decision::Allow  → Compressor (if enabled) → forward to upstream LLM
                     → ResponseAnalyzer on streamed chunks → client
9b. Decision::Warn   → forward + attach warning annotation to response
9c. Decision::Block  → return synthetic refusal; upstream LLM never contacted
```

---

## Module Boundary Table

| File | Type | Owns | Does NOT own |
|------|------|------|--------------|
| `main.rs` | binary entry | startup, config load, server bind | any business logic |
| `proxy.rs` | orchestration | Axum router, pipeline sequencing | detection logic |
| `config.rs` | config | deserialise `promptify.toml` → `Config` | runtime mutation |
| `decision.rs` | types | `Decision` enum, `Explanation` struct | logic that computes them |
| `explain.rs` | builder | assemble `Explanation` from signals | scoring, routing |
| `rules/mod.rs` | engine | load `ruleset.json`, `RuleEngine::check` | decoding, scoring |
| `decoder.rs` | engine | `DecoderEngine`: detect & decode encoded payloads | rule evaluation on decoded text |
| `ml_client.rs` | HTTP client | `POST /analyze` to sidecar | entropy math, scoring |
| `scoring.rs` | engine | `ScoringEngine`: merge signals → risk score + Decision | individual signal production |
| `logging.rs` | persistence | SQLite schema + async INSERT | any business logic |
| `response_analyzer.rs` | analyzer | rolling-window response inspection | scoring, logging |
| `compressor.rs` | transformer | optional prompt compression (Allow path only) | detection |
| `ml-sidecar/main.py` | HTTP wiring | FastAPI app, route `/analyze` | entropy math, ML |
| `ml-sidecar/entropy.py` | math | Shannon entropy computation | HTTP, routing, scoring |
| `ml-sidecar/classifier.py` | ML stub | future classifier (Phase 3) | entropy, routing |

---

## Canonical Vocabulary

These names are fixed across all phases. Do not invent synonyms.

| Name | Kind | Location |
|------|------|----------|
| `RuleEngine` | struct | `rules/mod.rs` |
| `DecoderEngine` | struct | `decoder.rs` |
| `ScoringEngine` | struct | `scoring.rs` |
| `Decision` | enum | `decision.rs` |
| `Explanation` | struct | `decision.rs` |
| `BackendAdapter` | (Phase 2+) | `proxy.rs` area |
| `MlClient` | struct | `ml_client.rs` |
| `Logger` | struct | `logging.rs` |
| `ResponseAnalyzer` | struct | `response_analyzer.rs` |
| `Compressor` | struct | `compressor.rs` |

---

## Data Schemas

### `config/promptify.toml` (owned by `config.rs`)

```toml
[proxy]
listen_port = 11434
upstream_url = "http://127.0.0.1:11435"

[backend]
type = "ollama"   # "ollama" | "llamacpp" | "generic_openai_compatible"

[thresholds]
block_at = 70
warn_at  = 30

[logging]
store_full_prompt_text = true

[compression]
enabled = false

[ml_sidecar]
url        = "http://127.0.0.1:8500"
timeout_ms = 500
```

### SQLite `requests` table (owned by `logging.rs`)

```sql
CREATE TABLE requests (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp             TEXT    NOT NULL,
    prompt_text           TEXT,            -- NULL when store_full_prompt_text = false
    prompt_hash           TEXT    NOT NULL,
    decision              TEXT    NOT NULL,
    risk_score            INTEGER NOT NULL,
    trust_score           INTEGER NOT NULL,
    explanation_json      TEXT    NOT NULL,
    decoded_payloads_json TEXT    NOT NULL
);
```

### `rules/ruleset.json` (owned by `rules/mod.rs`)

```json
{
  "override_phrases":          [...],   // weight 40
  "sensitive_keywords":        [...],   // weight 35
  "role_manipulation_patterns":[...]    // weight 25
}
```

---

## Repo Layout

```
c:\PROMPTIFY\
├── Cargo.toml               # Cargo workspace (members: core, cli)
├── core/                    # Rust crate — proxy + detection pipeline
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # startup wiring; reads port from config
│       ├── proxy.rs         # router: GET /health ✅  POST /api/generate (stub)
│       ├── config.rs
│       ├── decision.rs
│       ├── explain.rs
│       ├── decoder.rs
│       ├── ml_client.rs
│       ├── scoring.rs
│       ├── logging.rs
│       ├── response_analyzer.rs
│       ├── compressor.rs
│       └── rules/
│           ├── mod.rs
│           └── ruleset.json
├── ml-sidecar/              # Python FastAPI — entropy + future ML
│   ├── main.py              # GET /health ✅  POST /analyze (stub)
│   ├── entropy.py
│   ├── classifier.py
│   └── requirements.txt
├── cli/                     # Rust CLI binary (placeholder)
│   ├── Cargo.toml
│   └── src/main.rs
├── extension/               # Phase 4 — browser extension
├── proxy-ca/                # Phase 5 — HTTPS MITM CA
├── config/
│   └── promptify.toml
├── data/                    # GITIGNORED — runtime SQLite
├── ARCHITECTURE.md          # this file
├── AGENTS.md
├── CLAUDE.md
└── README.md
```

---

## Phase History

| Phase | Status | What was built |
|-------|--------|----------------|
| 0 | ✅ | Repo cleared, AGENTS.md / CLAUDE.md committed |
| 1 | ✅ | Full scaffold: directory tree, stub modules with doc comments, config/rule schemas, ARCHITECTURE.md |
| 1b | ✅ | Cargo workspace (`core` + `cli`); `GET /health` live on both services; config port wiring |
| 2 | ✅ | Detection logic: RuleEngine, DecoderEngine, ScoringEngine, MlClient, Logger |
