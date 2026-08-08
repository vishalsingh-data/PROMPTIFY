from fastapi import FastAPI
from fastapi.responses import StreamingResponse

app = FastAPI()

async def fake_stream():
    yield '{"response": "ok", "done": true}\n'

@app.post("/api/generate")
async def generate():
    return StreamingResponse(fake_stream(), media_type="application/x-ndjson")
