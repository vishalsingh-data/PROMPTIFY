use clap::{Parser, Subcommand};
use reqwest::Client;
use rusqlite::Connection;
use serde_json::Value;
use std::process::Command;
use std::time::Duration;
use tokio::signal;

#[derive(Parser)]
#[command(name = "promptify")]
#[command(about = "Promptify CLI — manage and inspect the local AI firewall", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the proxy server (and optionally the ML sidecar)
    Serve {
        #[arg(long)]
        with_ml_sidecar: bool,
    },
    /// Check the status of Promptify Core and the ML Sidecar
    Status,
    /// Replay a logged request and show the detailed analysis breakdown
    Replay {
        log_id: i64,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Serve { with_ml_sidecar } => {
            serve(*with_ml_sidecar).await?;
        }
        Commands::Status => {
            status().await?;
        }
        Commands::Replay { log_id } => {
            replay(*log_id).await?;
        }
    }

    Ok(())
}

async fn serve(with_ml_sidecar: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting promptify-core...");
    
    let mut core_child = Command::new("cargo")
        .args(["run", "-p", "promptify-core"])
        .spawn()
        .expect("Failed to start promptify-core");

    let mut sidecar_child = None;
    if with_ml_sidecar {
        println!("Starting ML sidecar...");
        sidecar_child = Some(
            Command::new("uvicorn")
                .args(["main:app", "--port", "8500"])
                .current_dir("ml-sidecar")
                .spawn()
                .expect("Failed to start ML sidecar"),
        );
    }

    // Wait for Ctrl+C
    match signal::ctrl_c().await {
        Ok(()) => {
            println!("\nReceived Ctrl+C, shutting down gracefully...");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }

    // Kill children
    println!("Stopping promptify-core...");
    let _ = core_child.kill();
    let _ = core_child.wait();

    if let Some(mut sc) = sidecar_child {
        println!("Stopping ML sidecar...");
        let _ = sc.kill();
        let _ = sc.wait();
    }

    println!("Shutdown complete.");
    Ok(())
}

async fn status() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder().timeout(Duration::from_millis(500)).build()?;

    let check_service = |url: &str| {
        let client = client.clone();
        let url = url.to_string();
        async move {
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => "UP",
                _ => "DOWN",
            }
        }
    };

    let core_status = check_service("http://127.0.0.1:11433/health").await;
    let ml_status = check_service("http://127.0.0.1:8500/health").await;

    println!("{:-<40}", "");
    println!("{:<20} | {}", "Service", "Status");
    println!("{:-<40}", "");
    println!("{:<20} | {}", "Promptify Core", core_status);
    println!("{:<20} | {}", "ML Sidecar", ml_status);
    println!("{:-<40}", "");

    Ok(())
}

async fn replay(log_id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open("data/promptify.db")?;
    let mut stmt = conn.prepare("SELECT prompt_text FROM requests WHERE id = ?1")?;
    
    let prompt_text_opt: Option<String> = stmt.query_row([log_id], |row| row.get(0))
        .map_err(|_| format!("Could not find log entry with ID {}.", log_id))?;

    let prompt_text = match prompt_text_opt {
        Some(text) => text,
        None => return Err(format!("Cannot replay log ID {} because the prompt text was not stored (store_full_prompt_text was false).", log_id).into()),
    };

    println!("Replaying log ID {}...", log_id);
    
    let client = Client::new();
    let res = client.post("http://127.0.0.1:11433/api/replay")
        .json(&serde_json::json!({
            "prompt": prompt_text
        }))
        .send()
        .await?;
        
    if !res.status().is_success() {
        eprintln!("Failed to call /api/replay on core proxy. Is it running? (Status: {})", res.status());
        return Ok(());
    }
    
    let body: Value = res.json().await?;
    
    println!("\n=== REPLAY BREAKDOWN ===");
    println!("Prompt: {}", prompt_text);
    println!("\n--- Decoder Engine ---");
    if let Some(payloads) = body.get("decoded_payloads").and_then(|v| v.as_array()) {
        if payloads.is_empty() {
            println!("No encoded payloads found.");
        } else {
            for (i, p) in payloads.iter().enumerate() {
                let text = p.get("plaintext").and_then(|v| v.as_str()).unwrap_or("");
                let typ = p.get("encoding_type").and_then(|v| v.as_str()).unwrap_or("unknown");
                println!("  [{}] Type: {}, Plaintext: {}", i, typ, text);
            }
        }
    }
    
    println!("\n--- Rule Engine ---");
    if let Some(matches) = body.get("rule_matches").and_then(|v| v.as_array()) {
        if matches.is_empty() {
            println!("No rules matched.");
        } else {
            for m in matches {
                let category = m.get("category").and_then(|v| v.as_str()).unwrap_or("");
                let pattern = m.get("matched_pattern").and_then(|v| v.as_str()).unwrap_or("");
                let severity = m.get("severity_weight").and_then(|v| v.as_u64()).unwrap_or(0);
                println!("  - Category: {} (Weight: {}), Pattern: '{}'", category, severity, pattern);
            }
        }
    }
    
    println!("\n--- ML Sidecar ---");
    if let Some(ml) = body.get("ml_signal") {
        if ml.is_null() {
            println!("No ML signal (sidecar down?)");
        } else {
            let entropy = ml.get("prompt_entropy").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let flag = ml.get("high_entropy_flag").and_then(|v| v.as_bool()).unwrap_or(false);
            println!("  Entropy: {:.2} (High Entropy Flag: {})", entropy, flag);
        }
    }
    
    println!("\n--- Scoring & Decision ---");
    let score = body.get("risk_score").and_then(|v| v.as_u64()).unwrap_or(0);
    let decision = body.get("decision").and_then(|v| v.as_str()).unwrap_or("Unknown");
    
    println!("  Risk Score: {}", score);
    println!("  Final Decision: {}", decision);
    
    println!("========================\n");
    
    Ok(())
}
