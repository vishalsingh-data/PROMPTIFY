# firewall_engine/policy.py

class PolicyEngine:
    """
    Enterprise policy evaluator for LLM Guardian.
    Decides action based on cumulative risk and categories seen.
    """

    # Default enterprise policies (can be expanded or made configurable)
    POLICIES = {
        "block_api_keys": True,
        "block_internal_urls": True,
        "block_credentials": True,
        "max_risk_threshold": 15  # Above this -> BLOCK
    }

    def evaluate(self, risk_score: int, categories_seen: set, escalation_level: int):
        """
        Evaluate risk and determine verdict and action.
        Returns: (verdict, action)
        """

        # Default verdict
        verdict = "SAFE"
        action = "ALLOW"

        # Escalation overrides
        if escalation_level >= 3:
            verdict = "HIGH RISK"
            action = "BLOCK"
        elif escalation_level == 2:
            verdict = "MEDIUM RISK"
            action = "FLAG"
        elif escalation_level == 1:
            verdict = "LOW RISK"
            action = "WARN"

        # Risk score threshold override
        if risk_score >= self.POLICIES["max_risk_threshold"]:
            verdict = "HIGH RISK"
            action = "BLOCK"

        # Specific category enforcement
        if self.POLICIES["block_api_keys"] and "data_exfiltration" in categories_seen:
            verdict = "HIGH RISK"
            action = "BLOCK"

        if self.POLICIES["block_credentials"] and "role_manipulation" in categories_seen:
            if verdict != "HIGH RISK":
                verdict = "MEDIUM RISK"
                action = "FLAG"

        return verdict, action