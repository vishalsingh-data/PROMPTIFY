# firewall_engine/rules.py

RULE_DEFINITIONS = {

    "instruction_override": {
        "weight": 2,
        "patterns": [
            r"ignore previous instructions",
            r"disregard earlier instructions",
            r"forget previous context",
            r"override system",
            r"new instructions:",
        ],
        "description": "Instruction override attempt detected."
    },

    "data_exfiltration": {
        "weight": 3,
        "patterns": [
            r"reveal.*secret",
            r"reveal.*api key",
            r"show.*api key",
            r"give.*api key",
            r"dump.*database",
            r"expose.*config",
            r"internal.*data",
            r"print.*credentials",
            r"access.*admin",

            #  Extended detection
            r"reveal.*system prompt",
            r"show.*system prompt",
            r"print.*system prompt",
            r"dump.*internal",
            r"internal.*configuration",
            r"hidden.*instructions"
        ],
        "description": "Sensitive data request detected."
    },

    "role_manipulation": {
        "weight": 1,
        "patterns": [
            r"you are now",
            r"act as system",
            r"act as admin",
            r"pretend to be",
        ],
        "description": "Role manipulation attempt detected."
    }
}