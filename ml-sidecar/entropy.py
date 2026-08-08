"""
Shannon entropy analysis for Promptify.

Owns:   Computing Shannon entropy over the character distribution of prompt text
        and applying a fixed threshold to produce a boolean 'flagged' signal.
Does not own: HTTP routing (-> main.py), ML classification (-> classifier.py),
              risk scoring decisions (-> promptify-core/scoring.rs), or any I/O.
"""

import math
from collections import Counter

# Entropy threshold above which a prompt is considered potentially suspicious.
# High entropy correlates with Base64, hex, or otherwise obfuscated payloads.
# This value is tuned in Phase 3 using labelled data.
ENTROPY_THRESHOLD: float = 4.5


def analyze(text: str) -> dict:
    """
    Compute Shannon entropy of ``text`` and return a result dict.

    Shannon entropy H = -Σ p(c) · log₂ p(c) over all unique characters c.
    Maximum theoretical entropy for 256-character alphabet ≈ 8.0 bits.

    Args:
        text: The raw prompt string to analyse.

    Returns:
        A dict with keys:
            "entropy" (float): rounded to 4 decimal places.
            "flagged" (bool): True when entropy > ENTROPY_THRESHOLD.
    """
    if not text:
        return {"entropy": 0.0, "flagged": False}

    counts = Counter(text)
    total = len(text)
    entropy = -sum(
        (count / total) * math.log2(count / total)
        for count in counts.values()
    )

    return {"entropy": round(entropy, 4), "flagged": entropy > ENTROPY_THRESHOLD}
