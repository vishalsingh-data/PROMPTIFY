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

/// Abstraction over upstream LLM server dialect.
pub trait BackendAdapter: Send + Sync {
    /// Extracts the core prompt from the JSON request payload.
    fn translate_request(&self, json_body: &serde_json::Value) -> Option<String>;
    
    /// Injects a modified prompt back into the JSON payload (used by Phase 3.9 Compression).
    fn inject_compressed_prompt(&self, json_body: &mut serde_json::Value, compressed: String);
    
    /// Determines if the response will be streamed based on the request payload.
    fn is_streaming_response(&self, json_body: &serde_json::Value) -> bool;
    
    /// Parses a raw stream chunk from the network and extracts plain text.
    fn extract_chunk_text(&self, chunk: &str) -> Option<String>;
    
    /// Formats the truncation notice in the backend's native stream format.
    fn format_truncation_notice(&self) -> String;
}

pub struct OllamaAdapter;

impl BackendAdapter for OllamaAdapter {
    fn translate_request(&self, json_body: &serde_json::Value) -> Option<String> {
        if let Some(p) = json_body.get("prompt").and_then(|v| v.as_str()) {
            return Some(p.to_string());
        } 
        if let Some(messages) = json_body.get("messages").and_then(|v| v.as_array()) {
            let mut extracted = String::new();
            for msg in messages {
                if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
                    extracted.push_str(content);
                    extracted.push('\n');
                }
            }
            if !extracted.is_empty() {
                return Some(extracted);
            }
        }
        None
    }

    fn inject_compressed_prompt(&self, json_body: &mut serde_json::Value, compressed: String) {
        if json_body.get("prompt").is_some() {
            json_body["prompt"] = serde_json::Value::String(compressed);
        } else if let Some(messages) = json_body.get_mut("messages").and_then(|v| v.as_array_mut()) {
            if let Some(last) = messages.last_mut() {
                last["content"] = serde_json::Value::String(compressed);
            }
        }
    }

    fn is_streaming_response(&self, json_body: &serde_json::Value) -> bool {
        json_body.get("stream").and_then(|v| v.as_bool()).unwrap_or(true)
    }

    fn extract_chunk_text(&self, chunk: &str) -> Option<String> {
        serde_json::from_str::<serde_json::Value>(chunk)
            .ok()
            .and_then(|v| v.get("response").and_then(|r| r.as_str()).map(|s| s.to_string()))
    }

    fn format_truncation_notice(&self) -> String {
        "\n\n[Promptify] Stream truncated due to sensitive content detection.".to_string()
    }
}

pub struct OpenAiCompatibleAdapter;

impl BackendAdapter for OpenAiCompatibleAdapter {
    fn translate_request(&self, json_body: &serde_json::Value) -> Option<String> {
        if let Some(messages) = json_body.get("messages").and_then(|v| v.as_array()) {
            let mut extracted = String::new();
            for msg in messages {
                if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
                    extracted.push_str(content);
                    extracted.push('\n');
                }
            }
            if !extracted.is_empty() {
                return Some(extracted);
            }
        }
        None
    }

    fn inject_compressed_prompt(&self, json_body: &mut serde_json::Value, compressed: String) {
        if let Some(messages) = json_body.get_mut("messages").and_then(|v| v.as_array_mut()) {
            if let Some(last) = messages.last_mut() {
                last["content"] = serde_json::Value::String(compressed);
            }
        }
    }

    fn is_streaming_response(&self, json_body: &serde_json::Value) -> bool {
        json_body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    fn extract_chunk_text(&self, chunk: &str) -> Option<String> {
        let trimmed = chunk.trim();
        if trimmed.is_empty() || trimmed == "data: [DONE]" {
            return None;
        }
        
        if let Some(json_str) = trimmed.strip_prefix("data: ") {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                    if let Some(first_choice) = choices.first() {
                        if let Some(delta) = first_choice.get("delta") {
                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                return Some(content.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn format_truncation_notice(&self) -> String {
        let notice = "\n\n[Promptify] Stream truncated due to sensitive content detection.";
        let chunk = serde_json::json!({
            "choices": [{
                "delta": {
                    "content": notice
                }
            }]
        });
        format!("data: {}\n\ndata: [DONE]\n\n", chunk.to_string())
    }
}

pub fn build_adapter(backend_type: &str) -> Box<dyn BackendAdapter> {
    match backend_type {
        "ollama" => Box::new(OllamaAdapter),
        "llamacpp" | "generic_openai_compatible" => Box::new(OpenAiCompatibleAdapter),
        _ => {
            tracing::warn!("Unknown backend type '{}', defaulting to ollama", backend_type);
            Box::new(OllamaAdapter)
        }
    }
}
