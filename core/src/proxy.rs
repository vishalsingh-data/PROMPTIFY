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
    // Strip Content-Length because we might have modified the body size (Phase 3.9)
    headers.remove(axum::http::header::CONTENT_LENGTH);

    let path = uri.path();
    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    let upstream = format!("{}{}{}", state.config.proxy.upstream_url.trim_end_matches('/'), path, query);

    // Read full request body (prompts are small enough to buffer safely)
    let mut body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(e) => return (axum::http::StatusCode::BAD_REQUEST, format!("Failed to read request body: {}", e)).into_response(),
    };

    tracing::info!("--- INCOMING REQUEST --- bytes: {:?}", body_bytes.len());

    let mut warning_header = None;

    let adapter = crate::config::build_adapter(&state.config.backend.backend_type);
    let mut is_streaming = true; // default to true

    if let Ok(mut json_body) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        is_streaming = adapter.is_streaming_response(&json_body);
        let extracted_prompt = adapter.translate_request(&json_body).unwrap_or_default();
        tracing::info!("Parsed JSON body via adapter.");

        if !extracted_prompt.is_empty() {
            let all_rule_matches = state.rules.check(&extracted_prompt);

            // Phase 3.3: Recursive Decoder cascade
            let decoded_payloads = state.decoder.decode(&extracted_prompt);
            let mut all_decoded_matches = Vec::new();
            for payload in &decoded_payloads {
                let mut decoded_matches = state.rules.check(&payload.plaintext);
                all_decoded_matches.append(&mut decoded_matches);
            }

            // Phase 3.4: ML Sidecar Entropy Analysis
            let decoded_plaintext: Vec<&str> = decoded_payloads.iter().map(|p| p.plaintext.as_str()).collect();
            let ml_result = state.ml.analyze(&extracted_prompt, decoded_plaintext).await;
            let ml_signal = ml_result.as_ref().ok();

            // Phase 3.5: Risk Scoring and Decision
            let (risk_score, decision, signals) = state.scoring.score(&all_rule_matches, &all_decoded_matches, ml_signal);
            let explanation = build_explanation(&signals, &decision, risk_score);

            // Phase 3.7: Logging
            let timestamp = chrono::Utc::now().to_rfc3339();
            let mut hasher = sha2::Sha256::new();
            hasher.update(extracted_prompt.as_bytes());
            let prompt_hash = hex::encode(hasher.finalize());

            let prompt_text = if state.config.logging.store_full_prompt_text {
                Some(extracted_prompt.clone())
            } else {
                None
            };
            
            let decoded_payloads_json = serde_json::to_string(&decoded_payloads).unwrap_or_else(|_| "[]".to_string());
            
            let record = RequestRecord {
                event_type: "request".to_string(),
                timestamp,
                prompt_text,
                prompt_hash,
                decision: decision.clone(),
                risk_score,
                trust_score: 100, // Hardcoded for Phase 3
                explanation: explanation.clone(),
                decoded_payloads_json,
            };
            
            state.logger.log_request(record);

            match decision {
                Decision::Block => {
                    tracing::warn!("Blocking request. Score: {} - {}", risk_score, explanation.summary);
                    let synthetic = json!({
                        "promptify_blocked": true,
                        "risk_score": risk_score,
                        "reason": explanation.summary,
                        "reasons": explanation.reasons,
                    });
                    return (axum::http::StatusCode::FORBIDDEN, axum::Json(synthetic)).into_response();
                }
                Decision::Warn => {
                    tracing::info!("Warning request. Score: {} - {}", risk_score, explanation.summary);
                    warning_header = Some(explanation.summary);
                }
                Decision::Allow => {
                    tracing::info!("Allowing request. Score: {}", risk_score);
                    
                    // Phase 3.9: Compression
                    if state.config.compression.enabled {
                        let compressor = crate::compressor::Compressor::new(state.config.compression.enabled);
                        let compressed = compressor.compress(extracted_prompt);
                        adapter.inject_compressed_prompt(&mut json_body, compressed);
                        
                        // Re-serialize
                        if let Ok(vec) = serde_json::to_vec(&json_body) {
                            body_bytes = axum::body::Bytes::from(vec);
                            tracing::info!("Compressed prompt before forwarding.");
                        }
                    }
                }
            }
        }
    }

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
    
    if let Some(warn_msg) = warning_header {
        if let Ok(hv) = axum::http::HeaderValue::from_str(&warn_msg) {
            response_builder = response_builder.header("X-Promptify-Warning", hv);
        }
    }

    // Convert reqwest's stream into axum's Body to forward chunks as they arrive
    let stream = upstream_resp.bytes_stream();
    let mut analyzer = crate::response_analyzer::ResponseAnalyzer::new(200, state.rules.clone());
    let logger = state.logger.clone();
    
    use futures_util::StreamExt;
    
    let stream_adapter = crate::config::build_adapter(&state.config.backend.backend_type);
    let modified_stream = async_stream::stream! {
        let mut stream = stream;
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        let mut blocked = false;
                        if let Some(extracted_text) = stream_adapter.extract_chunk_text(text) {
                            let decision = analyzer.analyze_chunk(&extracted_text);
                            if decision == Decision::Block {
                                blocked = true;
                            }
                        }
                        
                        if blocked {
                            let notice = stream_adapter.format_truncation_notice();
                            yield Ok(axum::body::Bytes::from(notice));
                            
                            let record = RequestRecord {
                                event_type: "response_blocked".to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                prompt_text: None,
                                prompt_hash: "RESPONSE_BLOCK".to_string(),
                                decision: Decision::Block,
                                risk_score: 100,
                                trust_score: 100,
                                explanation: Explanation {
                                    summary: "Response blocked mid-stream".to_string(),
                                    reasons: vec!["Sensitive content detected in response".to_string()],
                                    risk_score: 100,
                                },
                                decoded_payloads_json: "[]".to_string(),
                            };
                            logger.log_request(record);
                            
                            break;
                        } else {
                            yield Ok(bytes);
                        }
                    } else {
                        // Not UTF-8, yield as is
                        yield Ok(bytes);
                    }
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        }
    };

    let body = axum::body::Body::from_stream(modified_stream);
    response_builder.body(body).unwrap().into_response()
}
