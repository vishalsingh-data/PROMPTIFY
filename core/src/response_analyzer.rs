//! Rolling-window response analysis for `promptify-core`.
//!
//! **Owns**: inspecting streamed LLM response chunks as they arrive, applying
//!           `RuleEngine` checks over a sliding character window, and deciding
//!           whether each chunk should be passed through or suppressed.
//! **Does not own**: rule definitions (→ `rules`), risk scoring (→ `scoring`),
//!                   persistence (→ `logging`), or request-side analysis (handled
//!                   in `proxy.rs` before forwarding).

use crate::decision::Decision;

/// Analyses LLM response chunks in a rolling character window.
///
/// The window allows the analyzer to detect attack payloads that span chunk
/// boundaries — e.g. a rule pattern split across two streamed tokens.
pub struct ResponseAnalyzer {
    /// Number of characters retained in the rolling buffer.
    pub window_size: usize,
    /// Internal rolling buffer of recent response text.
    buffer: String,
}

impl ResponseAnalyzer {
    /// Create a new `ResponseAnalyzer` with the given window size in characters.
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            buffer: String::new(),
        }
    }

    /// Feed `chunk` into the rolling window and return a pass/suppress `Decision`.
    ///
    /// - `Decision::Allow` — chunk is clean, forward to client.
    /// - `Decision::Block` — a rule fired; suppress this chunk and signal caller
    ///                        to close the stream.
    pub fn analyze_chunk(&mut self, _chunk: &str) -> Decision {
        // TODO(Phase 2): append chunk to buffer, trim to window_size, run RuleEngine.
        todo!("Phase 2: implement rolling-window response analysis")
    }
}
