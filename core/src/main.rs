//! Entry point for `promptify-core`.
//!
//! **Owns**: process startup, config loading, tracing initialisation, and
//!           binding the Axum HTTP server.
//! **Does not own**: request routing (→ `proxy`), detection logic of any kind,
//!                   or any business logic beyond wiring.

mod compressor;
mod config;
mod decision;
mod decoder;
mod explain;
mod logging;
mod ml_client;
mod proxy;
mod response_analyzer;
mod rules;
mod scoring;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialise structured tracing output.
    tracing_subscriber::fmt::init();

    // Load configuration from the canonical location.
    let cfg = match config::Config::load(std::path::Path::new("config/promptify.toml")) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    let addr = format!("0.0.0.0:{}", cfg.proxy.listen_port);
    tracing::info!("promptify-core listening on {}", addr);

    // Initialise Database
    let logger = logging::Logger::new("data/requests.db".to_string());
    if let Err(e) = std::fs::create_dir_all("data") {
        tracing::error!("Failed to create data directory: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = logger.init().await {
        tracing::error!("Failed to init logger DB: {}", e);
        std::process::exit(1);
    }

    // Load Ruleset
    let rules = match rules::RuleEngine::load(std::path::Path::new("core/src/rules/ruleset.json")) {
        Ok(r) => Arc::new(r),
        Err(e) => {
            tracing::error!("Failed to load ruleset: {}", e);
            std::process::exit(1);
        }
    };

    // Instantiate Engines
    let decoder = Arc::new(decoder::DecoderEngine::new());
    let ml = Arc::new(ml_client::MlClient::new(
        cfg.ml_sidecar.url.clone(),
        cfg.ml_sidecar.timeout_ms,
    ));
    let scoring = Arc::new(scoring::ScoringEngine::new(config::ThresholdConfig {
        block_at: cfg.thresholds.block_at,
        warn_at: cfg.thresholds.warn_at,
    }));

    let state = proxy::AppState {
        rules,
        decoder,
        ml,
        scoring,
        logger,
    };

    // Build the Axum router and hand off to proxy.
    let app = proxy::router(state);

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}
