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
    override_phrases: Vec<Regex>,
    sensitive_keywords: Vec<Regex>,
    role_manipulation_patterns: Vec<Regex>,
}

impl RuleEngine {
    /// Load `ruleset.json` from `path`, deserialise, and prepare for evaluation.
    ///
    /// Pattern compilation (regex pre-compilation) will be added in Phase 2.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(path)?;
        let ruleset: Ruleset = serde_json::from_str(&raw)?;
        
        let compile = |patterns: Vec<String>| -> Result<Vec<Regex>, Box<dyn std::error::Error>> {
            let mut compiled = Vec::new();
            for p in patterns {
                // Compile case-insensitive for robustness against simple obfuscation
                let regex = Regex::new(&format!("(?i){}", p))?;
                compiled.push(regex);
            }
            Ok(compiled)
        };

        Ok(Self {
            override_phrases: compile(ruleset.override_phrases)?,
            sensitive_keywords: compile(ruleset.sensitive_keywords)?,
            role_manipulation_patterns: compile(ruleset.role_manipulation_patterns)?,
        })
    }

    /// Evaluate `prompt` against all rule categories.
    ///
    /// Returns every `RuleMatch` found, in the order: override phrases →
    /// sensitive keywords → role-manipulation patterns. An empty `Vec` means
    /// no rules fired.
    pub fn check(&self, prompt: &str) -> Vec<RuleMatch> {
        let mut matches = Vec::new();

        let mut check_category = |patterns: &[Regex], category: RuleCategory, weight: u8| {
            for re in patterns {
                if let Some(m) = re.find(prompt) {
                    matches.push(RuleMatch {
                        category: category.clone(),
                        matched_pattern: m.as_str().to_string(),
                        weight,
                    });
                }
            }
        };

        check_category(&self.override_phrases, RuleCategory::OverridePhrase, WEIGHT_OVERRIDE_PHRASE);
        check_category(&self.sensitive_keywords, RuleCategory::SensitiveKeyword, WEIGHT_SENSITIVE_KEYWORD);
        check_category(&self.role_manipulation_patterns, RuleCategory::RoleManipulation, WEIGHT_ROLE_MANIPULATION);

        matches
    }
}
