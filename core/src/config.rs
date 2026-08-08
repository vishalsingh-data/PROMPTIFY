//! Configuration loading for `promptify-core`.
//!
//! **Owns**: deserialising `config/promptify.toml` into the `Config` struct tree
//!           and surfacing load/parse errors to the caller.
//! **Does not own**: validation of thresholds against business rules (that is the
//!                   caller's responsibility), or any runtime mutation of config
//!                   after startup.

use serde::Deserialize;
use std::path::Path;

/// Top-level configuration, mirroring every section of `promptify.toml`.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub proxy: ProxyConfig,
    pub backend: BackendConfig,
    pub thresholds: ThresholdConfig,
    pub logging: LoggingConfig,
    pub compression: CompressionConfig,
    pub ml_sidecar: MlSidecarConfig,
}

/// `[proxy]` — network binding and upstream target.
#[derive(Debug, Deserialize)]
pub struct ProxyConfig {
    pub listen_port: u16,
    pub upstream_url: String,
}

/// `[backend]` — upstream LLM server dialect.
#[derive(Debug, Deserialize)]
pub struct BackendConfig {
    /// One of `"ollama"`, `"llamacpp"`, or `"generic_openai_compatible"`.
    #[serde(rename = "type")]
    pub backend_type: String,
}

/// `[thresholds]` — risk-score cut-offs that drive `Decision`.
#[derive(Debug, Deserialize)]
pub struct ThresholdConfig {
    /// Score (0–100) at or above which a request is blocked.
    pub block_at: u8,
    /// Score (0–100) at or above which a request is warned.
    pub warn_at: u8,
}

/// `[logging]` — SQLite logging behaviour.
#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    /// When `true`, full prompt text is stored; otherwise only the SHA-256 hash.
    pub store_full_prompt_text: bool,
}

/// `[compression]` — optional prompt compression before forwarding.
#[derive(Debug, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
}

/// `[ml_sidecar]` — connection parameters for `promptify-ml`.
#[derive(Debug, Deserialize)]
pub struct MlSidecarConfig {
    /// Base URL of the FastAPI sidecar (e.g. `"http://127.0.0.1:8500"`).
    pub url: String,
    /// Maximum time in milliseconds to wait for a sidecar response.
    pub timeout_ms: u64,
}

impl Config {
    /// Load and deserialise `promptify.toml` from `path`.
    ///
    /// Returns an error if the file cannot be read or the TOML is malformed.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&raw)?;
        Ok(cfg)
    }
}
