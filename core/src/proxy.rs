//! HTTP request routing and interception orchestration for `promptify-core`.
//!
//! **Owns**: the Axum router definition, the `/health` liveness probe, and the
//!           single intercept handler that sequences the detection pipeline stages
//!           in the correct order.
//! **Does not own**: any detection logic — scoring, rule evaluation, decoding,
//!                   ML calls, logging, or compression. Those are delegated to
//!                   their respective modules.

use axum::{
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::config::Config;

/// Build and return the Axum router wired to all supported paths.
///
/// Routes:
/// - `GET  /health`                  — liveness probe (implemented)
/// - `POST /api/generate`            — ollama-compatible intercept (Phase 2)
/// - `POST /v1/chat/completions`     — OpenAI-compatible intercept (Phase 2)
pub fn router(_cfg: Config) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/generate", post(intercept_handler))
        .route("/v1/chat/completions", post(intercept_handler))
}

/// Liveness probe — confirms the HTTP stack is up and responding.
///
/// Returns `{"status": "ok", "service": "promptify-core"}`.
/// No business logic; deliberately kept inline as pure wiring.
async fn health_handler() -> Json<Value> {
    Json(json!({"status": "ok", "service": "promptify-core"}))
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
async fn intercept_handler() -> (axum::http::StatusCode, &'static str) {
    // TODO(Phase 2): implement full interception pipeline.
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        "promptify-core: interception pipeline not yet implemented",
    )
}
