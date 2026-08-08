"""
ML classifier stub for Promptify.

Owns:   Future ML-based classification of prompts (Phase 3+). When a trained
        model is available it will be loaded and called here.
Does not own: Entropy analysis (-> entropy.py), HTTP routing (-> main.py),
              or any rule-based logic (-> promptify-core/rules/).

This module is intentionally a no-op stub. All calls return a neutral result
until a trained model is integrated in Phase 3.
"""


def classify(text: str) -> dict:
    """
    Classify ``text`` using an ML model.

    Currently a stub — returns a neutral result so the pipeline can proceed
    without a trained model present.

    Phase 3 will replace this body with:
    1. Model loading (e.g. scikit-learn, transformers, or ONNX runtime).
    2. Feature extraction from ``text``.
    3. Inference and confidence score.

    Args:
        text: The raw prompt string to classify.

    Returns:
        A dict with keys:
            "label"      (str):   predicted class, e.g. "injection" / "neutral".
            "confidence" (float): model confidence in [0.0, 1.0].
    """
    # TODO(Phase 3): load trained model and run inference.
    _ = text  # suppress unused-variable linting until Phase 3
    return {"label": "neutral", "confidence": 0.0}
