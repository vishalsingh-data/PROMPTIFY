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

#[tokio::main]
async fn main() {
    // Initialise structured tracing output.
    tracing_subscriber::fmt::init();

    // Load configuration from the canonical location.
    let cfg = match config::Config::load(std::path::Path::new("../config/promptify.toml")) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    let addr = format!("0.0.0.0:{}", cfg.proxy.listen_port);
    tracing::info!("promptify-core listening on {}", addr);

    // Build the Axum router and hand off to proxy.
    let app = proxy::router(cfg);

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
