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
    pub fn compress(&self, prompt: String) -> String {
        if !self.enabled {
            return prompt;
        }

        // TODO: Phase 4/Future - call ml-sidecar for real summarization/compression model

        // 1. Collapse whitespace
        let mut compressed = prompt.split_whitespace().collect::<Vec<_>>().join(" ");

        // 2. Strip filler phrases
        let fillers = [
            "can you please",
            "could you please",
            "i was wondering if",
            "would you mind",
            "please",
        ];
        
        for filler in fillers.iter() {
            // Case insensitive replace would be better, but standard replace is simple and conservative
            // We'll use a simple case-insensitive regex for filler phrases.
            // Since we're trying to keep it simple, let's just do a basic string replacement or use regex if available.
            // We have regex in the project.
        }
        
        let mut lower = compressed.to_lowercase();
        for filler in fillers.iter() {
            // Very naive replacement loop
            while let Some(idx) = lower.find(filler) {
                compressed.replace_range(idx..idx + filler.len(), "");
                lower.replace_range(idx..idx + filler.len(), "");
            }
        }
        
        // Fix up double spaces created by filler removal
        compressed = compressed.split_whitespace().collect::<Vec<_>>().join(" ");

        // 3. Dedupe repeated sentences
        let sentences: Vec<&str> = compressed.split(". ").collect();
        let mut deduped_sentences = Vec::new();
        for sentence in sentences {
            let s = sentence.trim();
            if !s.is_empty() && !deduped_sentences.contains(&s) {
                deduped_sentences.push(s);
            }
        }
        
        deduped_sentences.join(". ")
    }
}
