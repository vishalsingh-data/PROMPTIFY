//! Rolling-window response analysis for `promptify-core`.
//!
//! **Owns**: inspecting streamed LLM response chunks as they arrive, applying
//!           `RuleEngine` checks over a sliding character window, and deciding
//!           whether each chunk should be passed through or suppressed.
//! **Does not own**: rule definitions (→ `rules`), risk scoring (→ `scoring`),
//!                   persistence (→ `logging`), or request-side analysis (handled
//!                   in `proxy.rs` before forwarding).

use crate::decision::Decision;
use crate::rules::RuleEngine;
use std::sync::Arc;

/// Analyses LLM response chunks in a rolling character window.
///
/// The window allows the analyzer to detect attack payloads that span chunk
/// boundaries — e.g. a rule pattern split across two streamed tokens.
pub struct ResponseAnalyzer {
    /// Number of characters retained in the rolling buffer.
    pub window_size: usize,
    /// Internal rolling buffer of recent response text.
    buffer: String,
    /// Reference to the RuleEngine
    rules: Arc<RuleEngine>,
}

impl ResponseAnalyzer {
    /// Create a new `ResponseAnalyzer` with the given window size in characters.
    pub fn new(window_size: usize, rules: Arc<RuleEngine>) -> Self {
        Self {
            window_size,
            buffer: String::with_capacity(window_size * 2), // Pre-allocate some capacity
            rules,
        }
    }

    /// Feed `chunk` into the rolling window and return a pass/suppress `Decision`.
    ///
    /// - `Decision::Allow` — chunk is clean, forward to client.
    /// - `Decision::Block` — a rule fired; suppress this chunk and signal caller
    ///                        to close the stream.
    pub fn analyze_chunk(&mut self, chunk: &str) -> Decision {
        self.buffer.push_str(chunk);
        
        // If buffer is too long, truncate from the left to keep only `window_size` characters
        if self.buffer.len() > self.window_size {
            // Find the character boundary
            let start = self.buffer.len() - self.window_size;
            let mut char_boundary = start;
            while char_boundary < self.buffer.len() && !self.buffer.is_char_boundary(char_boundary) {
                char_boundary += 1;
            }
            if char_boundary < self.buffer.len() {
                // Keep everything from char_boundary onwards
                let new_buffer = self.buffer[char_boundary..].to_string();
                self.buffer = new_buffer;
            }
        }
        
        // Run rules check
        let matches = self.rules.check(&self.buffer);
        if !matches.is_empty() {
            return Decision::Block;
        }
        
        Decision::Allow
    }
}
