//! Core decision types for `promptify-core`.
//!
//! **Owns**: the `Decision` enum and `Explanation` struct — the canonical typed
//!           outcome returned by the detection pipeline for every request.
//! **Does not own**: the logic that computes a `Decision` (→ `scoring`), the text
//!                   that populates an `Explanation` (→ `explain`), or persistence
//!                   of either (→ `logging`).

use serde::{Deserialize, Serialize};

/// The verdict produced by the detection pipeline for a single intercepted request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Decision {
    /// The request passes all checks — forward it to the upstream LLM.
    Allow,
    /// The request is suspicious but below the block threshold — forward with a warning annotation.
    Warn,
    /// The request is blocked — return a synthetic refusal; the real LLM is never contacted.
    Block,
}

/// Human-readable rationale attached to a `Decision`.
///
/// Serialised as JSON and stored verbatim in the `explanation_json` column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    /// Short one-line summary shown in CLI output and log previews.
    pub summary: String,
    /// Ordered list of detection signals that contributed to the decision,
    /// from highest-scoring to lowest.
    pub signals: Vec<String>,
    /// Risk score (0–100) at the moment the decision was made.
    pub risk_score: u8,
}
