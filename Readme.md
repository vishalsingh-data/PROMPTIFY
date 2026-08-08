# Promptify

**A local-first AI firewall** that intercepts prompts and responses between a
client and a local LLM server (ollama / llama.cpp), detecting prompt injection,
sensitive data requests, and encoded payload attacks before they reach the model.

---

## Architecture

```
Client → promptify-core (Rust/Axum) → upstream LLM
                   ↕
          promptify-ml (Python/FastAPI)
```

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the full system map and request lifecycle.

## Stack

| Component | Language | Role |
|-----------|----------|------|
| `core/` | Rust (axum, tokio, reqwest) | Proxy, detection pipeline, decisions, logging |
| `ml-sidecar/` | Python (FastAPI) | Entropy analysis, future ML classification |
| `cli/` | Rust | CLI for status, log inspection, config validation |

## Quick Start

```bash
# 1. Start the ML sidecar
cd ml-sidecar && pip install -r requirements.txt
uvicorn main:app --port 8500

# 2. Start the core proxy
cd core && cargo run

# 3. Point your LLM client at localhost:11434 (instead of ollama's default)
```

## Configuration

Edit `config/promptify.toml` — see inline comments for all options.

## Project Status

🚧 **Phase 1 — Architecture & Scaffold** (current)
