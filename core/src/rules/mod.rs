//! Rule engine for `promptify-core`.
//!
//! **Owns**: loading `ruleset.json` from disk, compiling patterns, and evaluating
//!           prompts against all three rule categories (override phrases, sensitive
//!           keywords, role-manipulation patterns). Returns typed `RuleMatch` results
//!           with fixed severity weights.
//! **Does not own**: decoding encoded payloads before rule evaluation (→ `decoder`),
//!                   aggregating match scores into a final risk score (→ `scoring`),
//!                   or any I/O beyond reading the single ruleset JSON file.

use serde::Deserialize;
use std::path::Path;

// ── Severity weights — fixed by architecture spec §2.4 ────────────────────────

/// Severity weight for override-phrase matches.
pub const WEIGHT_OVERRIDE_PHRASE: u8 = 40;
/// Severity weight for sensitive-keyword matches.
pub const WEIGHT_SENSITIVE_KEYWORD: u8 = 35;
/// Severity weight for role-manipulation pattern matches.
pub const WEIGHT_ROLE_MANIPULATION: u8 = 25;

// ── Public types ───────────────────────────────────────────────────────────────

/// The rule category a match belongs to.
#[derive(Debug, Clone)]
pub enum RuleCategory {
    OverridePhrase,
    SensitiveKeyword,
    RoleManipulation,
}

/// A single rule hit returned by `RuleEngine::check`.
#[derive(Debug, Clone)]
pub struct RuleMatch {
    /// The category that produced this match.
    pub category: RuleCategory,
    /// The pattern or keyword string that matched.
    pub matched_pattern: String,
    /// Fixed severity weight for this category (see `WEIGHT_*` constants above).
    pub weight: u8,
}

// ── Internal ruleset representation ───────────────────────────────────────────

/// Internal representation of the JSON ruleset loaded from `ruleset.json`.
#[derive(Debug, Deserialize)]
struct Ruleset {
    override_phrases: Vec<String>,
    sensitive_keywords: Vec<String>,
    role_manipulation_patterns: Vec<String>,
}

// ── RuleEngine ─────────────────────────────────────────────────────────────────

/// Loads and evaluates `ruleset.json` against incoming prompts.
pub struct RuleEngine {
    ruleset: Ruleset,
}

impl RuleEngine {
    /// Load `ruleset.json` from `path`, deserialise, and prepare for evaluation.
    ///
    /// Pattern compilation (regex pre-compilation) will be added in Phase 2.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(path)?;
        let ruleset: Ruleset = serde_json::from_str(&raw)?;
        Ok(Self { ruleset })
    }

    /// Evaluate `prompt` against all rule categories.
    ///
    /// Returns every `RuleMatch` found, in the order: override phrases →
    /// sensitive keywords → role-manipulation patterns. An empty `Vec` means
    /// no rules fired.
    pub fn check(&self, _prompt: &str) -> Vec<RuleMatch> {
        // TODO(Phase 2): iterate categories, run substring / regex matching.
        todo!("Phase 2: implement pattern evaluation across all rule categories")
    }
}
