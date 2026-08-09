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
use crate::rules::RuleMatch;
use crate::ml_client::MlSignal;

/// Aggregates detection signals into a final risk score and `Decision`.
pub struct ScoringEngine {
    thresholds: ThresholdConfig,
}

impl ScoringEngine {
    /// Create a new `ScoringEngine` with the supplied threshold configuration.
    pub fn new(thresholds: ThresholdConfig) -> Self {
        Self { thresholds }
    }

    /// Merge signals into a 0–100 risk score and derive the `Decision`.
    pub fn score(
        &self,
        raw_rules: &[RuleMatch],
        decoded_rules: &[RuleMatch],
        ml_signal: Option<&MlSignal>,
    ) -> (u8, Decision, Vec<Signal>) {
        let mut signals = Vec::new();
        let mut total_score: u16 = 0;

        for m in raw_rules {
            signals.push(Signal {
                label: format!("rule:{:?}", m.category),
                score: m.weight,
            });
            total_score += m.weight as u16;
        }

        for m in decoded_rules {
            let adjusted_weight = (m.weight as f32 * 1.5).round() as u8;
            signals.push(Signal {
                label: format!("decoded_rule:{:?}", m.category),
                score: adjusted_weight,
            });
            total_score += adjusted_weight as u16;
        }

        if let Some(ml) = ml_signal {
            if ml.high_entropy_flag {
                let ml_score = 30; // Assign 30 for high entropy
                signals.push(Signal {
                    label: "ml:high_entropy".to_string(),
                    score: ml_score,
                });
                total_score += ml_score as u16;
            }
        }
        
        let risk_score = if total_score > 100 { 100 } else { total_score as u8 };
        
        let decision = if risk_score >= self.thresholds.block_at {
            Decision::Block
        } else if risk_score >= self.thresholds.warn_at {
            Decision::Warn
        } else {
            Decision::Allow
        };
        
        (risk_score, decision, signals)
    }
}
