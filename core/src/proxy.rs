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
async fn intercept_handler(State(state): State<AppState>) -> (axum::http::StatusCode, &'static str) {
    // Stub prompt extraction for Phase 2 (to be replaced in Phase 3 when BackendAdapter is added)
    let prompt = "TODO_extract_real_prompt_from_body";

    // 1. Check rules
    let rule_matches = state.rules.check(prompt);

    // 2. Decode & re-check decoded text
    let decoded = state.decoder.decode(prompt);
    let mut decoded_matches = Vec::new();
    for d in &decoded {
        decoded_matches.extend(state.rules.check(&d.plaintext));
    }

    // 3. ML analysis
    let ml_signal = state.ml.analyze(prompt).await.unwrap_or(crate::ml_client::MlSignal {
        entropy: 0.0,
        flagged: false,
    });

    // 4. Scoring
    let mut signals = Vec::new();
    for rm in rule_matches.iter().chain(decoded_matches.iter()) {
        signals.push(Signal { label: rm.matched_pattern.clone(), score: rm.weight });
    }
    if ml_signal.flagged {
        signals.push(Signal { label: "ml_entropy".to_string(), score: 20 });
    }

    let (risk_score, decision) = state.scoring.score(&signals);

    // 5. Build Explanation
    let explanation = build_explanation(&signals, &decision, risk_score);

    // 6. Logging
    let mut hasher = Sha256::new();
    hasher.update(prompt);
    let hash_str = hex::encode(hasher.finalize());
    
    // We use a stub ISO-8601 string here for Phase 2 instead of importing chrono yet.
    let timestamp = "2026-08-08T00:00:00Z".to_string(); 

    let record = RequestRecord {
        timestamp,
        prompt_text: Some(prompt.to_string()),
        prompt_hash: hash_str,
        decision: decision.clone(),
        risk_score,
        trust_score: 100, // Phase 3 placeholder
        explanation,
        decoded_payloads_json: "[]".to_string(), // Serialize if needed later
    };

    let _ = state.logger.log_request(record).await;

    // 7. Forward (Stubbed until Phase 3)
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        "promptify-core: interception pipeline executed, but upstream forwarding not yet implemented",
    )
}
