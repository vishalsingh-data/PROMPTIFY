chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.type === "ANALYZE_PROMPT") {
    console.log("[Promptify] Sending prompt to local core for analysis...", request.prompt);

    fetch("http://127.0.0.1:11433/extension/analyze", {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        prompt: request.prompt,
        source_url: request.source_url
      })
    })
      .then((response) => {
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        return response.json();
      })
      .then((data) => {
        console.log("[Promptify] Core response:", data);
        sendResponse({ success: true, data: data });
      })
      .catch((error) => {
        console.error("[Promptify] Failed to reach local core:", error);
        sendResponse({ success: false, error: error.message });
      });

    // Return true to indicate that we will send a response asynchronously
    return true;
  }
});
