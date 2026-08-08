//! Optional prompt compression for `promptify-core`.
//!
//! **Owns**: reducing prompt token count before forwarding to the upstream LLM,
//!           gated by the `[compression] enabled` config flag.
//! **Does not own**: detection logic of any kind. `Compressor` runs only on
//!                   prompts that have already received a `Decision::Allow` from
//!                   `ScoringEngine`, never before.

/// Handles optional prompt compression on the Allow path.
pub struct Compressor {
    /// When `false`, `compress` is a no-op and the prompt is returned unchanged.
    pub enabled: bool,
}

impl Compressor {
    /// Create a new `Compressor`.
    ///
    /// `enabled` should be taken directly from `Config::compression.enabled`.
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Compress `prompt` if compression is enabled; otherwise return it unchanged.
    ///
    /// Phase 2 will implement a concrete strategy (e.g. whitespace normalisation,
    /// repeated-token deduplication). The interface is intentionally simple so the
    /// caller in `proxy.rs` does not need to change when the strategy evolves.
    pub fn compress(&self, prompt: String) -> String {
        if !self.enabled {
            return prompt;
        }
        // TODO(Phase 2): implement compression strategy.
        todo!("Phase 2: implement compression strategy")
    }
}
