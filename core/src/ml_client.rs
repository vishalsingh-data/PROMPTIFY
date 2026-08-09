//! HTTP client for the `promptify-ml` sidecar service.
//!
//! **Owns**: serialising requests to and deserialising responses from the sidecar's
//!           `POST /analyze` endpoint. Timeout enforcement lives here.
//! **Does not own**: entropy math or ML logic (→ `ml-sidecar/entropy.py`), scoring
//!                   (→ `scoring`), rule evaluation (→ `rules`), or any I/O besides
//!                   the single HTTP call.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Signal returned by the ML sidecar for a single prompt.
#[derive(Debug, Clone)]
pub struct MlSignal {
    pub prompt_entropy: f64,
    pub payload_entropies: Vec<f64>,
    pub high_entropy_flag: bool,
    pub classifier_verdict: String,
}

/// Request body sent to `POST /analyze`.
#[derive(Debug, Serialize)]
struct AnalyzeRequest<'a> {
    prompt: &'a str,
    decoded_payloads: Vec<&'a str>,
}

/// Response body received from `POST /analyze`.
#[derive(Debug, Deserialize)]
struct AnalyzeResponse {
    prompt_entropy: f64,
    payload_entropies: Vec<f64>,
    high_entropy_flag: bool,
    classifier_verdict: String,
}

/// HTTP client for the `promptify-ml` FastAPI sidecar.
pub struct MlClient {
    /// Base URL of the sidecar (e.g. `"http://127.0.0.1:8500"`).
    pub base_url: String,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    
    client: Client,
}

impl MlClient {
    /// Create a new `MlClient` with the given base URL and timeout.
    pub fn new(base_url: String, timeout_ms: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_default();
            
        Self { 
            base_url, 
            timeout_ms,
            client,
        }
    }

    /// Send `prompt` to `POST /analyze` and return the resulting `MlSignal`.
    pub async fn analyze(&self, prompt: &str, decoded_payloads: Vec<&str>) -> Result<MlSignal, Box<dyn std::error::Error>> {
        let url = format!("{}/analyze", self.base_url.trim_end_matches('/'));
        
        let req = AnalyzeRequest { prompt, decoded_payloads };
        
        let resp = self.client.post(&url)
            .json(&req)
            .send()
            .await?;
            
        let data: AnalyzeResponse = resp.json().await?;
        
        Ok(MlSignal {
            prompt_entropy: data.prompt_entropy,
            payload_entropies: data.payload_entropies,
            high_entropy_flag: data.high_entropy_flag,
            classifier_verdict: data.classifier_verdict,
        })
    }
}
