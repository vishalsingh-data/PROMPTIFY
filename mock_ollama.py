from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse
import asyncio
import json

app = FastAPI()

async def fake_stream(leak: bool, prompt: str):
    if leak:
        chunks = ["Here ", "is ", "the ", "info: ", "my password is ", "hunter2."]
        for chunk in chunks:
            yield json.dumps({"response": chunk, "done": False}) + "\n"
            await asyncio.sleep(0.1)
        yield json.dumps({"response": "", "done": True}) + "\n"
    else:
        yield json.dumps({"response": f"Clean response. Received prompt: {prompt}", "done": True}) + "\n"

@app.post("/api/generate")
async def generate(req: Request):
    body = await req.json()
    prompt = body.get("prompt", "")
    leak = "leak" in prompt.lower()
    return StreamingResponse(fake_stream(leak, prompt), media_type="application/x-ndjson")
