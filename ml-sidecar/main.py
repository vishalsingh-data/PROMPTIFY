"""
ML sidecar for Promptify — FastAPI application entry point.

Owns:   HTTP routing for the sidecar, Pydantic request/response models, and
        wiring calls through to entropy.py and classifier.py.
Does not own: entropy math (-> entropy.py), ML classification (-> classifier.py),
              or any Rust-side orchestration logic.
"""

from fastapi import FastAPI
from pydantic import BaseModel

from entropy import analyze as entropy_analyze

app = FastAPI(
    title="promptify-ml",
    version="0.1.0",
    description="Entropy analysis and ML classification sidecar for Promptify.",
)


class AnalyzeRequest(BaseModel):
    """Payload sent by promptify-core's MlClient."""
    text: str


class AnalyzeResponse(BaseModel):
    """Response consumed by promptify-core's MlClient."""
    entropy: float
    flagged: bool


@app.post("/analyze", response_model=AnalyzeResponse)
async def analyze(request: AnalyzeRequest) -> AnalyzeResponse:
    """
    Primary analysis endpoint called by promptify-core.

    Delegates to entropy.analyze() for Shannon entropy computation.
    Phase 3 will also call classifier.classify() here.
    """
    result = entropy_analyze(request.text)
    return AnalyzeResponse(entropy=result["entropy"], flagged=result["flagged"])


@app.get("/health")
async def health() -> dict:
    """Liveness probe — promptify-core checks this on startup."""
    return {"status": "ok"}
