//! Explanation builder for `promptify-core`.
//!
//! **Owns**: assembling a human-readable `Explanation` from an ordered set of
//!           scored `Signal`s and the final `Decision`.
//! **Does not own**: scoring (→ `scoring`), deciding Allow/Warn/Block (→ `scoring`
//!                   / `decision`), or log formatting / persistence (→ `logging`).

use crate::decision::{Decision, Explanation};

/// A scored detection signal contributed by one pipeline stage.
#[derive(Debug, Clone)]
pub struct Signal {
    /// Short human-readable label identifying the signal source (e.g. `"rule:override_phrase"`).
    pub label: String,
    /// Contribution to the total risk score (0–100 scale, additive across signals).
    pub score: u8,
}

/// Assemble an `Explanation` from a slice of `Signal`s and the resolved `Decision`.
///
/// The summary line is derived from the highest-scoring signal. If `signals` is empty
/// the summary reflects a clean pass.
///
/// # Arguments
/// * `signals`     — All signals produced by the pipeline, highest-score first.
/// * `decision`    — The `Decision` that `ScoringEngine` produced.
/// * `risk_score`  — The aggregated risk score (0–100).
pub fn build_explanation(
    signals: &[Signal],
    decision: &Decision,
    risk_score: u8,
) -> Explanation {
    let mut sorted_signals = signals.to_vec();
    // Sort descending by score
    sorted_signals.sort_by(|a, b| b.score.cmp(&a.score));

    let summary = match decision {
        Decision::Allow => "Request passed all checks.".to_string(),
        Decision::Warn => {
            if let Some(top) = sorted_signals.first() {
                format!("Warning triggered primarily by {}", top.label)
            } else {
                "Warning triggered by unknown signals.".to_string()
            }
        },
        Decision::Block => {
            if let Some(top) = sorted_signals.first() {
                format!("Blocked primarily due to {}", top.label)
            } else {
                "Blocked by unknown signals.".to_string()
            }
        }
    };

    let signal_labels = sorted_signals.into_iter().map(|s| format!("{} (score: {})", s.label, s.score)).collect();

    Explanation {
        summary,
        signals: signal_labels,
        risk_score,
    }
}
