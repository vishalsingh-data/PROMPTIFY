# firewall_engine/logger.py

import os
import json
from datetime import datetime

# Logs directory (ensure it exists)
LOG_DIR = os.path.join(os.path.dirname(__file__), "../../logs")
os.makedirs(LOG_DIR, exist_ok=True)


def log_attack(session_id: str, text: str, analysis_result: dict, source: str = "user"):
    """
    Logs a detected risky prompt or AI output to a daily JSONL file.
    Designed for production-level auditing and monitoring.
    
    Parameters:
    - session_id: unique session identifier for multi-turn conversations
    - text: the user or AI text analyzed
    - analysis_result: dictionary returned from Analyzer
    - source: "user" or "ai"
    """
    log_file = os.path.join(LOG_DIR, f"{datetime.today().strftime('%Y-%m-%d')}.jsonl")
    
    log_entry = {
        "timestamp": datetime.utcnow().isoformat(),
        "session_id": session_id,
        "source": source,
        "text": text,
        "risk_score": analysis_result.get("risk_score", 0),
        "categories": analysis_result.get("categories_seen", []),
        "verdict": analysis_result.get("verdict", ""),
        "action": analysis_result.get("action", ""),
        "escalation_level": analysis_result.get("escalation_level", 0)
    }

    try:
        with open(log_file, "a", encoding="utf-8") as f:
            f.write(json.dumps(log_entry, ensure_ascii=False) + "\n")
    except Exception as e:
        print(f"[Logger Error] Could not write log: {e}")