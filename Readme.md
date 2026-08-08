# Promptify

**A local-first AI firewall** that intercepts prompts and responses between a
client and a local LLM server (ollama / llama.cpp), detecting prompt injection,
sensitive data requests, and encoded payload attacks before they reach the model.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full system design.

---

## Services

| Service | Language | Default port | Path |
|---------|----------|-------------|------|
| `promptify-core` | Rust / Axum | `11434` | `core/` |
| `promptify-ml` | Python / FastAPI | `8500` | `ml-sidecar/` |

---

## Running locally

> **All commands are run from the repo root** (`c:\PROMPTIFY`) unless noted.

### 1 — promptify-ml (ML sidecar)

```bash
cd ml-sidecar
python -m venv venv
# Windows:
venv\Scripts\activate
# macOS / Linux:
source venv/bin/activate

pip install -r requirements.txt
uvicorn main:app --port 8500 --reload
```

Health check: `curl http://localhost:8500/health`
→ `{"status":"ok"}`

### 2 — promptify-core (Rust proxy)

> **⚠️ Port conflict**: promptify-core binds to port `11434` — the same port
> ollama uses by default. **Stop ollama before starting promptify-core**,
> otherwise ollama will hold the port and our server cannot bind.
>
> On Windows (ollama tray app): right-click the ollama tray icon → Quit.
> Then verify the port is free: `netstat -ano | findstr :11434` (should return nothing).

```bash
# From repo root:
cargo run -p promptify-core
```

The port is read from `config/promptify.toml` (`listen_port = 11434`).

Health check: `curl http://localhost:11434/health`
→ `{"status":"ok","service":"promptify-core"}`

### 3 — CLI (placeholder)

```bash
cargo run -p promptify-cli
```

---

## Configuration

Edit `config/promptify.toml`. Key fields:

```toml
[proxy]
listen_port = 11434          # port promptify-core binds to
upstream_url = "http://127.0.0.1:11435"  # real ollama / llama.cpp

[ml_sidecar]
url = "http://127.0.0.1:8500"
timeout_ms = 500
```

---

## Project Status

| Phase | Status | Description |
|-------|--------|-------------|
| 0 | ✅ | Repo cleared, AGENTS.md / CLAUDE.md |
| 1 | ✅ | Full scaffold: directory tree, stub modules, schemas |
| 1b | ✅ | Cargo workspace, GET /health on both services |
| 2 | ⬜ | Detection logic: RuleEngine, DecoderEngine, ScoringEngine, Logger |
