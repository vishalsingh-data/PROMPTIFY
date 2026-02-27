# firewall_engine/memory.py

class ConversationMemory:
    def __init__(self):
        self.reset()

    def add_turn(self, text: str, triggered_categories: set, risk_score: int):
        """
        Add a turn to memory with categories and risk score.
        """
        self.turns.append({
            "text": text,
            "categories": list(triggered_categories),
            "risk_score": risk_score
        })

        # Update cumulative score
        self.cumulative_score += risk_score

        # Update categories seen across all turns
        self.categories_seen.update(triggered_categories)

        # Update escalation level
        self._update_escalation()

    def _update_escalation(self):
        category_counts = len(self.categories_seen)
        if category_counts >= 3:
            self.escalation_level = 3
        elif category_counts == 2:
            self.escalation_level = 2
        elif category_counts == 1:
            self.escalation_level = 1
        else:
            self.escalation_level = 0

    def reset(self):
        self.turns = []
        self.cumulative_score = 0
        self.categories_seen = set()
        self.escalation_level = 0