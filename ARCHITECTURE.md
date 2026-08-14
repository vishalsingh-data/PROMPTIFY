# Promptify Architecture

Promptify is a local-first AI firewall designed to intercept prompts and responses
between a client and an upstream LLM server. It detects prompt injection, data
exfiltration, and encoded payload attacks using a combination of static rules
and ML-based entropy analysis.

## High-Level Architecture

The system consists of two primary processes:

1. **Promptify Core (Rust)**: A high-performance reverse proxy that sits between
   the client and the upstream LLM server (e.g., Ollama, llama.cpp, or OpenAI).
   It handles all traffic interception, rule evaluation, decoding, and scoring.
2. **ML Sidecar (Python)**: An auxiliary service that provides ML-based detection
   capabilities (e.g., entropy analysis) that are too complex or slow to implement
   in pure Rust. The core communicates with the sidecar via HTTP or gRPC.

### Data Flow (System Proxy)

1. **Ingress**: The client sends a request to the proxy.
2. **Analysis Pipeline**:
   - `proxy::intercept_handler` extracts the prompt from the request body.
   - The request is passed to the `RuleEngine` for static analysis.
   - The request is passed to the `DecoderEngine` to detect obfuscation.
   - The request is passed to the `ML Sidecar` for entropy and ML analysis.
   - The results from all engines are aggregated by the `ScoringEngine`.
3. **Decision**: The `ScoringEngine` outputs a `Decision` (Allow, Warn, Block).
4. **Action**:
   - If `Allow` or `Warn` (and user overrides), the proxy forwards the request to the upstream LLM.
   - If `Block`, the proxy returns an immediate HTTP error response detailing the `Explanation`.

### Data Flow (Browser Extension)

To protect non-technical users directly in web UIs (ChatGPT, Claude, Gemini) without requiring a system-wide proxy or root CA installation, Promptify includes a Manifest V3 browser extension.

1. **Generic UI Interception**: `content.js` listens globally for `Enter` keystrokes and generic submit button clicks. If triggered inside a text input or `contenteditable` element, it extracts the prompt and pauses the submission locally.
2. **Side-channel Analysis**: The prompt is sent to `background.js`, which fires an out-of-band `POST /extension/analyze` request directly to `promptify-core` running on localhost.
3. **Pipeline Reuse**: `promptify-core` routes this request through the exact same `RuleEngine`, `DecoderEngine`, and `ScoringEngine` pipeline, but does *not* forward it to an upstream LLM. It simply returns the `Decision` and `Explanation`.
4. **Client-side Enforcement**: The extension reads the decision.
   - If `Allow`: It silently synthesizes an `Enter` keypress or click to let the website's native code submit the prompt.
   - If `Warn`: It injects a sleek banner above the input, but still submits the prompt.
   - If `Block`: It clears the input box and injects a robust Shadow DOM modal overlay explaining the block reasons, giving the user an option to manually discard or override the block.

## Directory Structure

- `cli/`: Command-line interface for starting and managing Promptify.
  - `src/main.rs`: CLI entrypoint (clap parser, orchestration).
- `core/`: The main Rust detection engine and proxy.
  - `src/main.rs`: Core entrypoint (starts the axum server).
  - `src/proxy.rs`: Request routing, proxy logic, and side-channel endpoints.
  - `src/rules/`: Static rule definitions and regex matching.
  - `src/decoder.rs`: Base64/Hex decoding and recursive payload detection.
  - `src/scoring.rs`: Logic for aggregating signals into a final score.
  - `src/decision.rs`: Typed definition of the `Decision` enum.
  - `src/explain.rs`: Generates human-readable rationales for decisions.
- `ml-sidecar/`: Python FastApi service.
  - `main.py`: Sidecar entrypoint.
  - `analyzer.py`: ML models (entropy calculation, future transformers).
- `extension/`: Chrome Manifest V3 browser extension.
  - `manifest.json`: Configuration and permissions.
  - `content.js`: The Generic UI interceptor and Shadow DOM UI renderer.
  - `background.js`: Cross-origin fetch bridge and session telemetry tracking.
  - `popup.html/js`: The UI shown when clicking the extension icon.
