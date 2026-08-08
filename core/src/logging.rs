//! Async SQLite request logging for `promptify-core`.
//!
//! **Owns**: the `requests` table schema, initialising the database file, and all
//!           INSERT operations. All writes are non-blocking so logging never adds
//!           latency to the response path.
//! **Does not own**: any business logic, scoring decisions, or explanation
//!                   assembly. It only persists exactly what it is given.

use crate::decision::{Decision, Explanation};
use rusqlite::{params, Connection};

/// A complete record of one intercepted request, ready to be persisted.
///
/// ## SQLite schema (column order matches INSERT statement in Phase 2)
/// ```sql
/// CREATE TABLE IF NOT EXISTS requests (
///     id                    INTEGER PRIMARY KEY AUTOINCREMENT,
///     timestamp             TEXT    NOT NULL,
///     prompt_text           TEXT,            -- NULL when store_full_prompt_text = false
///     prompt_hash           TEXT    NOT NULL,
///     decision              TEXT    NOT NULL,
///     risk_score            INTEGER NOT NULL,
///     trust_score           INTEGER NOT NULL,
///     explanation_json      TEXT    NOT NULL,
///     decoded_payloads_json TEXT    NOT NULL
/// );
/// ```
#[derive(Debug, Clone)]
pub struct RequestRecord {
    /// ISO-8601 timestamp of when the request was intercepted.
    pub timestamp: String,
    /// Full prompt text — `None` when `store_full_prompt_text = false`.
    pub prompt_text: Option<String>,
    /// SHA-256 hex digest of the raw prompt.
    pub prompt_hash: String,
    /// The verdict produced by `ScoringEngine`.
    pub decision: Decision,
    /// Aggregated risk score (0–100).
    pub risk_score: u8,
    /// Trust score for the session (0–100, reserved for Phase 3 trust engine).
    pub trust_score: u8,
    /// Serialised `Explanation` — stored as JSON text.
    pub explanation: Explanation,
    /// JSON array of `DecodedPayload`s found by `DecoderEngine`, serialised as text.
    pub decoded_payloads_json: String,
}

/// Async logger that persists `RequestRecord`s to SQLite without blocking callers.
#[derive(Clone)]
pub struct Logger {
    /// Filesystem path to the SQLite database file (under `data/`).
    pub db_path: String,
}

impl Logger {
    /// Create a new `Logger` targeting `db_path`.
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }

    /// Open (or create) the database and ensure the `requests` table exists.
    ///
    /// Must be called once at startup before any calls to `log_request`.
    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        let db_path = self.db_path.clone();
        
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS requests (
                    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp             TEXT    NOT NULL,
                    prompt_text           TEXT,
                    prompt_hash           TEXT    NOT NULL,
                    decision              TEXT    NOT NULL,
                    risk_score            INTEGER NOT NULL,
                    trust_score           INTEGER NOT NULL,
                    explanation_json      TEXT    NOT NULL,
                    decoded_payloads_json TEXT    NOT NULL
                )",
                [],
            )?;
            Ok::<(), rusqlite::Error>(())
        }).await??;
        
        Ok(())
    }

    /// Persist a `RequestRecord` asynchronously.
    ///
    /// Spawns a blocking task so the response path is never delayed.
    pub async fn log_request(
        &self,
        record: RequestRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let db_path = self.db_path.clone();
        
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;
            let decision_str = match record.decision {
                Decision::Allow => "ALLOW",
                Decision::Warn => "WARN",
                Decision::Block => "BLOCK",
            };
            
            let explanation_json = serde_json::to_string(&record.explanation)
                .unwrap_or_else(|_| "{}".to_string());
                
            conn.execute(
                "INSERT INTO requests (
                    timestamp, prompt_text, prompt_hash, decision,
                    risk_score, trust_score, explanation_json, decoded_payloads_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.timestamp,
                    record.prompt_text,
                    record.prompt_hash,
                    decision_str,
                    record.risk_score,
                    record.trust_score,
                    explanation_json,
                    record.decoded_payloads_json,
                ],
            )?;
            
            Ok::<(), rusqlite::Error>(())
        }).await??;
        
        Ok(())
    }
}
