//! HTTP client for the `promptify-ml` sidecar service.
//!
//! **Owns**: serialising requests to and deserialising responses from the sidecar's
//!           `POST /analyze` endpoint. Timeout enforcement lives here.
//! **Does not own**: entropy math or ML logic (→ `ml-sidecar/entropy.py`), scoring
//!                   (→ `scoring`), rule evaluation (→ `rules`), or any I/O besides
//!                   the single HTTP call.

use serde::{Deserialize, Serialize};

/// Signal returned by the ML sidecar for a single prompt.
#[derive(Debug, Clone)]
pub struct MlSignal {
    /// Shannon entropy of the prompt (0.0 – ~8.0 for character distributions).
    pub entropy: f64,
    /// `true` when entropy exceeds the sidecar's internal threshold.
    pub flagged: bool,
}

/// Request body sent to `POST /analyze`.
#[derive(Debug, Serialize)]
struct AnalyzeRequest<'a> {
    text: &'a str,
}

/// Response body received from `POST /analyze`.
#[derive(Debug, Deserialize)]
struct AnalyzeResponse {
    entropy: f64,
    flagged: bool,
}

/// HTTP client for the `promptify-ml` FastAPI sidecar.
pub struct MlClient {
    /// Base URL of the sidecar (e.g. `"http://127.0.0.1:8500"`).
    pub base_url: String,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
}

impl MlClient {
    /// Create a new `MlClient` with the given base URL and timeout.
    pub fn new(base_url: String, timeout_ms: u64) -> Self {
        Self { base_url, timeout_ms }
    }

    /// Send `prompt` to `POST /analyze` and return the resulting `MlSignal`.
    ///
    /// If the sidecar is unreachable or times out, returns an error. The caller
    /// (`proxy.rs`) is responsible for deciding how to handle a sidecar failure
    /// (degrade gracefully vs. block by default).
    pub async fn analyze(&self, _prompt: &str) -> Result<MlSignal, Box<dyn std::error::Error>> {
        // TODO(Phase 2): build reqwest client with timeout, POST to /analyze, deserialise.
        todo!("Phase 2: implement HTTP call to sidecar")
    }
}
