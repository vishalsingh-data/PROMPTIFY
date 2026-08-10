//! Rule engine for `promptify-core`.
//!
//! **Owns**: loading `ruleset.json` from disk, compiling patterns, and evaluating
//!           prompts against all three rule categories (override phrases, sensitive
//!           keywords, role-manipulation patterns). Returns typed `RuleMatch` results
//!           with fixed severity weights.
//! **Does not own**: decoding encoded payloads before rule evaluation (→ `decoder`),
//!                   aggregating match scores into a final risk score (→ `scoring`),
//!                   or any I/O beyond reading the single ruleset JSON file.

use regex::Regex;
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
#[derive(Debug, Clone, serde::Serialize)]
pub enum RuleCategory {
    OverridePhrase,
    SensitiveKeyword,
    RoleManipulation,
}

/// A single rule hit returned by `RuleEngine::check`.
#[derive(Debug, Clone, serde::Serialize)]
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
    override_phrases: Vec<String>,
    sensitive_keywords: Vec<String>,
    role_manipulation_patterns: Vec<Regex>,
}

impl RuleEngine {
    /// Load `ruleset.json` from `path`, deserialise, and prepare for evaluation.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(path)?;
        let ruleset: Ruleset = serde_json::from_str(&raw)?;
        
        // Lowercase strings for case-insensitive substring matching
        let override_phrases = ruleset.override_phrases.into_iter().map(|s| s.to_lowercase()).collect();
        let sensitive_keywords = ruleset.sensitive_keywords.into_iter().map(|s| s.to_lowercase()).collect();

        // Compile regexes for role patterns
        let mut role_manipulation_patterns = Vec::new();
        for p in ruleset.role_manipulation_patterns {
            role_manipulation_patterns.push(Regex::new(&p)?);
        }

        Ok(Self {
            override_phrases,
            sensitive_keywords,
            role_manipulation_patterns,
        })
    }

    /// Evaluate `prompt` against all rule categories.
    ///
    /// Returns every `RuleMatch` found, in the order: override phrases →
    /// sensitive keywords → role-manipulation patterns.
    pub fn check(&self, prompt: &str) -> Vec<RuleMatch> {
        let mut matches = Vec::new();
        let lower_prompt = prompt.to_lowercase();

        // Check exact substrings (case-insensitive)
        for phrase in &self.override_phrases {
            if lower_prompt.contains(phrase) {
                matches.push(RuleMatch {
                    category: RuleCategory::OverridePhrase,
                    matched_pattern: phrase.clone(),
                    weight: WEIGHT_OVERRIDE_PHRASE,
                });
            }
        }

        for keyword in &self.sensitive_keywords {
            if lower_prompt.contains(keyword) {
                matches.push(RuleMatch {
                    category: RuleCategory::SensitiveKeyword,
                    matched_pattern: keyword.clone(),
                    weight: WEIGHT_SENSITIVE_KEYWORD,
                });
            }
        }

        // Check regex patterns
        for re in &self.role_manipulation_patterns {
            if let Some(m) = re.find(prompt) {
                matches.push(RuleMatch {
                    category: RuleCategory::RoleManipulation,
                    matched_pattern: m.as_str().to_string(),
                    weight: WEIGHT_ROLE_MANIPULATION,
                });
            }
        }

        matches
    }
}
