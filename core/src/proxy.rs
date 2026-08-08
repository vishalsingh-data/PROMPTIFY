//! HTTP request routing and interception orchestration for `promptify-core`.
//!
//! **Owns**: the Axum router definition and the single intercept handler that
//!           sequences the detection pipeline stages in the correct order.
//! **Does not own**: any detection logic — scoring, rule evaluation, decoding,
//!                   ML calls, logging, or compression. Those are delegated to
//!                   their respective modules.

use axum::{Router, routing::post};

use crate::config::Config;

/// Build and return the Axum router wired to all supported LLM API paths.
///
/// Both the ollama (`/api/generate`) and OpenAI-compatible (`/v1/chat/completions`)
/// endpoints are intercepted by the same handler so a single pipeline covers all clients.
pub fn router(_cfg: Config) -> Router {
    Router::new()
        .route("/api/generate", post(intercept_handler))
        .route("/v1/chat/completions", post(intercept_handler))
}

/// Primary intercept handler — sequences the full detection pipeline.
///
/// Pipeline order (Phase 2 implementation):
/// 1. `RuleEngine::check(prompt)`
/// 2. `DecoderEngine::decode(prompt)` → re-check decoded payloads via `RuleEngine`
/// 3. `MlClient::analyze(prompt)`
/// 4. `ScoringEngine::score(signals)`
/// 5. `build_explanation(signals, decision)`
/// 6. `Logger::log_request(record)` — spawned async, never blocks response
/// 7a. `Decision::Allow`  → `Compressor::compress` → forward to upstream LLM
///                        → `ResponseAnalyzer` on streamed chunks → client
/// 7b. `Decision::Warn`   → forward + attach warning annotation
/// 7c. `Decision::Block`  → return synthetic refusal; upstream LLM never contacted
async fn intercept_handler() -> &'static str {
    // TODO(Phase 2): implement full interception pipeline.
    "promptify-core: interception not yet implemented"
}
