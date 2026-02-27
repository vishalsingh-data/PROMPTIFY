# firewall_engine/analyzer.py

import re
import base64
import unicodedata
import math
from firewall_engine.rules import RULE_DEFINITIONS
from firewall_engine.memory import ConversationMemory
from firewall_engine.policy import PolicyEngine
from firewall_engine.logger import log_attack


class Analyzer:
    def __init__(self):
        self.rules = RULE_DEFINITIONS
        self.memory = ConversationMemory()
        self.policy = PolicyEngine()

    def analyze(self, text: str, source: str = "user") -> dict:
        """
        Analyze user input or AI output.

        source: "user" or "ai"
        """
        text = self._preprocess_text(text)

        findings = []
        risk_score = 0
        triggered_categories = set()

        # -----------------------------
        # Pattern-Based Detection
        # -----------------------------
        for category, rule in self.rules.items():
            category_triggered = False
            for pattern in rule["patterns"]:
                if re.search(pattern, text, re.IGNORECASE):
                    findings.append({
                        "category": category,
                        "pattern": pattern,
                        "description": rule["description"]
                    })
                    category_triggered = True
            if category_triggered:
                risk_score += rule.get("weight", 1)
                triggered_categories.add(category)

        # -----------------------------
        # Entropy-Based Obfuscation Detection
        # -----------------------------
        entropy = self._calculate_entropy(text)
        if entropy > 5.0:
            risk_score += 4
            findings.append({
                "category": "obfuscation",
                "pattern": "high_entropy_text",
                "description": "Highly obfuscated or encoded content detected."
            })
        elif entropy > 4.5 and len(text) > 50:
            risk_score += 2
            findings.append({
                "category": "obfuscation",
                "pattern": "moderate_entropy_text",
                "description": "Potential obfuscation detected."
            })

        # -----------------------------
        # Update Conversation Memory
        # -----------------------------
        self.memory.add_turn(
            text=text,
            triggered_categories=triggered_categories,
            risk_score=risk_score
        )

        # -----------------------------
        # Apply Policy Engine
        # -----------------------------
        verdict, action = self.policy.evaluate(
            risk_score=self.memory.cumulative_score,
            categories_seen=self.memory.categories_seen,
            escalation_level=self.memory.escalation_level
        )

        # -----------------------------
        # Log if risky
        # -----------------------------
        if risk_score > 0:
            log_attack(text, {
                "findings": findings,
                "risk_score": risk_score,
                "categories_seen": list(self.memory.categories_seen),
                "escalation_level": self.memory.escalation_level,
                "verdict": verdict,
                "action": action
            }, source=source)

        return {
            "findings": findings,
            "risk_score": risk_score,
            "categories_seen": list(self.memory.categories_seen),
            "verdict": verdict,
            "action": action,
            "escalation_level": self.memory.escalation_level
        }

    def reset_conversation(self):
        self.memory.reset()

    # -----------------------------
    # Helpers
    # -----------------------------
    def _preprocess_text(self, text: str) -> str:
        text = unicodedata.normalize("NFKC", text)
        text = re.sub(r'[\u200B-\u200D\uFEFF]', '', text)

        potential_base64 = re.findall(r'[A-Za-z0-9+/=]{20,}', text)
        decoded_content = ""
        for blob in potential_base64:
            try:
                decoded = base64.b64decode(blob).decode('utf-8', errors='ignore')
                if len(decoded.strip()) > 5:
                    decoded_content += " " + decoded
            except Exception:
                continue
        text = text + decoded_content
        text = re.sub(r'[`*_>#-]', ' ', text)
        return text.lower()

    def _calculate_entropy(self, text: str) -> float:
        if not text:
            return 0.0
        freq = {}
        for char in text:
            freq[char] = freq.get(char, 0) + 1
        entropy = 0.0
        length = len(text)
        for count in freq.values():
            probability = count / length
            entropy -= probability * math.log2(probability)
        return entropy