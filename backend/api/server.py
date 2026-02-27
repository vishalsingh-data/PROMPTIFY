# api/server.py

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uuid

from firewall_engine.analyzer import Analyzer
from firewall_engine.logger import log_attack

app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # hackathon demo mode
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
analyzer = Analyzer()  # Analyzer already contains memory

# In-memory session store (replace with Redis/db in real production)
SESSION_STORE = {}


# ---------------------------
# Request Models
# ---------------------------

class AnalyzeRequest(BaseModel):
    text: str
    session_id: str | None = None  # Optional session_id from frontend


class AnalyzeOutputRequest(BaseModel):
    text: str
    session_id: str  # Required for AI output (must belong to a session)


# ---------------------------
# Helper Functions
# ---------------------------

def get_memory(session_id: str):
    """Retrieve or create memory for a given session ID."""
    if session_id not in SESSION_STORE:
        SESSION_STORE[session_id] = analyzer.memory.__class__()
    return SESSION_STORE[session_id]


def enforce_policy(result: dict):
    """Modify result if enforcement rules are needed."""
    if result.get("verdict") == "HIGH RISK":
        result["action_taken"] = "BLOCKED"
    else:
        result["action_taken"] = "ALLOW"
    return result


# ---------------------------
# Endpoints
# ---------------------------

@app.post("/analyze-input")
async def analyze_input(request: AnalyzeRequest):
    try:
        # Generate session_id if not provided
        session_id = request.session_id or str(uuid.uuid4())
        memory = get_memory(session_id)

        # Swap analyzer memory to session-specific memory
        original_memory = analyzer.memory
        analyzer.memory = memory

        # Analyze user input
        result = analyzer.analyze(request.text, source="user")

        # Apply policy enforcement
        result = enforce_policy(result)

        # Log risky prompts
        if result.get("risk_score", 0) > 0:
            log_attack(
                session_id=session_id,
                text=request.text,
                analysis_result=result,
                source="user"
            )

        # Restore original analyzer memory
        analyzer.memory = original_memory

        return {
            "session_id": session_id,
            "result": result,
            "memory": {
                "turns": memory.turns,
                "cumulative_score": memory.cumulative_score,
                "escalation_level": memory.escalation_level
            }
        }

    except Exception as e:
        return {"error": str(e)}


@app.get("/health")
def health():
    return {"status": "ok"}


@app.post("/analyze-output")
async def analyze_output(request: AnalyzeOutputRequest):
    try:
        session_id = request.session_id
        memory = get_memory(session_id)

        # Swap analyzer memory
        original_memory = analyzer.memory
        analyzer.memory = memory

        # Analyze AI output
        result = analyzer.analyze(request.text, source="ai")

        # Apply enforcement
        result = enforce_policy(result)

        # Log risky AI outputs
        if result.get("risk_score", 0) > 0:
            log_attack(
                session_id=session_id,
                text=request.text,
                analysis_result=result,
                source="ai"
            )

        # Restore analyzer memory
        analyzer.memory = original_memory

        return {
            "session_id": session_id,
            "result": result,
            "memory": {
                "turns": memory.turns,
                "cumulative_score": memory.cumulative_score,
                "escalation_level": memory.escalation_level
            }
        }

    except Exception as e:
        return {"error": str(e)}