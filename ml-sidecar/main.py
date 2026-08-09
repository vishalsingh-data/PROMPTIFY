"""
ML sidecar for Promptify — FastAPI application entry point.

Owns:   HTTP routing for the sidecar, Pydantic request/response models, and
        wiring calls through to entropy.py and classifier.py.
Does not own: entropy math (-> entropy.py), ML classification (-> classifier.py),
              or any Rust-side orchestration logic.
"""

from fastapi import FastAPI
from pydantic import BaseModel
from typing import List

from entropy import ENTROPY_THRESHOLD, analyze as entropy_analyze

app = FastAPI(
    title="promptify-ml",
    version="0.1.0",
    description="Entropy analysis and ML classification sidecar for Promptify.",
)


class AnalyzeRequest(BaseModel):
    """Payload sent by promptify-core's MlClient."""
    prompt: str
    decoded_payloads: List[str]


class AnalyzeResponse(BaseModel):
    """Response consumed by promptify-core's MlClient."""
    prompt_entropy: float
    payload_entropies: List[float]
    high_entropy_flag: bool
    classifier_verdict: str = "not_implemented"


@app.post("/analyze", response_model=AnalyzeResponse)
async def analyze(request: AnalyzeRequest) -> AnalyzeResponse:
    """
    Primary analysis endpoint called by promptify-core.

    Delegates to entropy.analyze() for Shannon entropy computation.
    Phase 3 will also call classifier.classify() here.
    """
    prompt_ent = entropy_analyze(request.prompt)["entropy"]
    payload_ents = [entropy_analyze(p)["entropy"] for p in request.decoded_payloads]
    
    any_high = prompt_ent > ENTROPY_THRESHOLD or any(e > ENTROPY_THRESHOLD for e in payload_ents)
    
    return AnalyzeResponse(
        prompt_entropy=prompt_ent,
        payload_entropies=payload_ents,
        high_entropy_flag=any_high,
        # TODO Phase: replace with trained classifier
        classifier_verdict="not_implemented"
    )


@app.get("/health")
async def health() -> dict:
    """Liveness probe — promptify-core checks this on startup."""
    return {"status": "ok"}
