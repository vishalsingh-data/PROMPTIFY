//! Signal aggregation and risk scoring for `promptify-core`.
//!
//! **Owns**: merging all detection signals (rule matches, decoder payloads, ML
//!           entropy) into a single 0–100 risk score, and mapping that score to a
//!           `Decision` using the configured thresholds.
//! **Does not own**: individual signal production (→ `rules`, `decoder`,
//!                   `ml_client`), explanation text assembly (→ `explain`),
//!                   or persistence (→ `logging`).

use crate::config::ThresholdConfig;
use crate::decision::Decision;
use crate::explain::Signal;

/// Aggregates detection signals into a final risk score and `Decision`.
pub struct ScoringEngine {
    thresholds: ThresholdConfig,
}

impl ScoringEngine {
    /// Create a new `ScoringEngine` with the supplied threshold configuration.
    pub fn new(thresholds: ThresholdConfig) -> Self {
        Self { thresholds }
    }

    /// Merge `signals` into a 0–100 risk score and derive the `Decision`.
    ///
    /// Returns `(risk_score, decision)`. The caller should pass the same `signals`
    /// slice to `explain::build_explanation` to produce the accompanying `Explanation`.
    ///
    /// Scoring rules (Phase 2 implementation):
    /// - Clamp the sum of all signal scores to [0, 100].
    /// - `risk_score >= thresholds.block_at` → `Decision::Block`
    /// - `risk_score >= thresholds.warn_at`  → `Decision::Warn`
    /// - otherwise                           → `Decision::Allow`
    pub fn score(&self, signals: &[Signal]) -> (u8, Decision) {
        let mut total_score: u16 = 0;
        for s in signals {
            total_score += s.score as u16;
        }
        
        let risk_score = if total_score > 100 { 100 } else { total_score as u8 };
        
        let decision = if risk_score >= self.thresholds.block_at {
            Decision::Block
        } else if risk_score >= self.thresholds.warn_at {
            Decision::Warn
        } else {
            Decision::Allow
        };
        
        (risk_score, decision)
    }
}
