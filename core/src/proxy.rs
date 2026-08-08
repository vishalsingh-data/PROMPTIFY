//! HTTP request routing and interception orchestration for `promptify-core`.
//!
//! **Owns**: the Axum router definition, the `/health` liveness probe, and the
//!           single intercept handler that sequences the detection pipeline stages
//!           in the correct order.
//! **Does not own**: any detection logic — scoring, rule evaluation, decoding,
//!                   ML calls, logging, or compression. Those are delegated to
//!                   their respective modules.

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    decoder::DecoderEngine,
    decision::{Decision, Explanation},
    explain::{build_explanation, Signal},
    logging::{Logger, RequestRecord},
    ml_client::MlClient,
    rules::RuleEngine,
    scoring::ScoringEngine,
};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct AppState {
    pub rules: Arc<RuleEngine>,
    pub decoder: Arc<DecoderEngine>,
    pub ml: Arc<MlClient>,
    pub scoring: Arc<ScoringEngine>,
    pub logger: Logger,
    pub http_client: reqwest::Client,
    pub config: Arc<crate::config::Config>,
}

/// Build and return the Axum router wired to all supported paths.
///
/// Routes:
/// - `GET  /health`                  — liveness probe (implemented)
/// - `POST /api/generate`            — ollama-compatible intercept (Phase 2)
/// - `POST /v1/chat/completions`     — OpenAI-compatible intercept (Phase 2)
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/generate", post(intercept_handler))
        .route("/v1/chat/completions", post(intercept_handler))
        .with_state(state)
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
async fn intercept_handler(
    State(state): State<AppState>,
    mut req: axum::extract::Request,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    // Phase 2 Detection Logic is temporarily bypassed for pure passthrough in Phase 3.1
    // (In Phase 3.2, we will inspect the request body here before forwarding).

    // 1. Extract request components
    let method = req.method().clone();
    let uri = req.uri().clone();
    let mut headers = req.headers().clone();

    // Strip Host header so reqwest sets the correct upstream host
    headers.remove(axum::http::header::HOST);

    let path = uri.path();
    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    let upstream = format!("{}{}{}", state.config.proxy.upstream_url.trim_end_matches('/'), path, query);

    // Read full request body (prompts are small enough to buffer safely)
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(e) => return (axum::http::StatusCode::BAD_REQUEST, format!("Failed to read request body: {}", e)).into_response(),
    };

    // 2. Forward request to upstream
    let reqwest_req = state.http_client.request(method, &upstream)
        .headers(headers)
        .body(body_bytes);

    let upstream_resp = match reqwest_req.send().await {
        Ok(r) => r,
        Err(e) => return (axum::http::StatusCode::BAD_GATEWAY, format!("Upstream LLM error: {}", e)).into_response(),
    };

    // 3. Stream response back to client
    let mut response_builder = axum::http::Response::builder()
        .status(upstream_resp.status());

    for (name, value) in upstream_resp.headers() {
        response_builder = response_builder.header(name, value);
    }

    // Convert reqwest's stream into axum's Body to forward chunks as they arrive
    let stream = upstream_resp.bytes_stream();
    let body = axum::body::Body::from_stream(stream);

    response_builder.body(body).unwrap().into_response()
}
