# core/analyzer.py

import firewall_engine.rules
print("RULE FILE PATH:", firewall_engine.rules.__file__)
import re
from firewall_engine.rules import RULE_DEFINITIONS


class PromptAnalyzer:

    def __init__(self):
        self.rules = RULE_DEFINITIONS

    def analyze(self, text: str) -> dict:
        findings = []
        risk_score = 0

        for category, rule in self.rules.items():
            for pattern in rule["patterns"]:
                if re.search(pattern, text, re.IGNORECASE):
                    #temporary line below!!!!!!!
                    print("DEBUG RULE:",category, rule)
                    findings.append({
                        "category": category,
                        "pattern": pattern,
                        "description": rule["description"]
                    })
                    risk_score += rule.get("weight", 1)

        confidence = min(100, risk_score * 20)
        verdict = self._generate_verdict(risk_score)

        return {
            "findings": findings,
            "risk_score": risk_score,
            "confidence": confidence,
            "verdict": verdict
        }

    def _generate_verdict(self, score: int) -> str:
        if score == 0:
            return "SAFE"
        elif score <= 2:
            return "LOW RISK"
        elif score <= 5:
            return "MEDIUM RISK"
        else:
            return "HIGH RISK"
