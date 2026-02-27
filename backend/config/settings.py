# config/settings.py

MAX_RISK_SCORE = 100

RISK_THRESHOLDS = {
    "SAFE": 30,
    "SUSPICIOUS": 70,
}

RULE_SCORES = {
    "instruction_override": 40,
    "data_exfiltration": 30,
    "role_manipulation": 20,
    "encoding_detected": 30,
    "high_entropy": 20,
}
