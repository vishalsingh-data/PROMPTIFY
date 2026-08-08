//! CLI entry point for Promptify.
//!
//! **Owns**: command-line argument parsing and user-facing terminal output for
//!           the `promptify` CLI binary — subcommands for status checks, log
//!           inspection, and config validation.
//! **Does not own**: proxy logic (→ `promptify-core`), detection logic of any
//!                   kind, or direct SQLite access (reads logs via the core API).

fn main() {
    // TODO(Phase 2+): implement CLI subcommands (status, logs, validate-config).
    println!("promptify CLI — not yet implemented");
}
