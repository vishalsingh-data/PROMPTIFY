/**
 * PROMPTIFY CONTENT SCRIPT (Generic UI Interceptor)
 * 
 * This script runs in the isolated extension world. It listens for Enter keypresses
 * globally on any text input element. This guarantees it works on ANY LLM site 
 * without needing specific CSS selectors or intercepting messy background network traffic.
 */

console.log("[Promptify] Generic UI Interceptor loaded on", window.location.hostname);

// A flag to prevent infinite loops when we artificially re-dispatch the Enter key
let isSynthesizingEnter = false;

document.body.addEventListener("keydown", async (event) => {
  // If we are currently re-dispatching an allowed event, ignore it
  if (isSynthesizingEnter) return;

  // Only intercept Enter (without shift, as shift+enter is usually newline)
  if (event.key === "Enter" && !event.shiftKey) {
    const target = event.target;

    // Check if the target is a text input area
    const isTextArea = target.tagName === "TEXTAREA";
    const isTextInput = target.tagName === "INPUT" && target.type === "text";
    const isContentEditable = target.isContentEditable;

    if (isTextArea || isTextInput || isContentEditable) {
      // Extract the text depending on the element type
      let promptText = "";
      if (isTextArea || isTextInput) {
        promptText = target.value;
      } else if (isContentEditable) {
        promptText = target.innerText || target.textContent;
      }

      promptText = promptText ? promptText.trim() : "";
      
      // Ignore empty submissions
      if (!promptText) return;

      console.log("[Promptify] Intercepted prompt:", promptText);

      // PREVENT the chat site from submitting the prompt
      event.preventDefault();
      event.stopImmediatePropagation();

      // Show a visual loading state
      const originalOpacity = target.style.opacity;
      target.style.opacity = "0.5";

      try {
        // Send the prompt to background.js for analysis
        const response = await new Promise((resolve) => {
          chrome.runtime.sendMessage(
            {
              type: "ANALYZE_PROMPT",
              prompt: promptText,
              source_url: window.location.href
            },
            (res) => resolve(res)
          );
        });

        target.style.opacity = originalOpacity;

        if (response && response.success) {
          const data = response.data;
          
          if (data.decision === "Block") {
            alert(`[PROMPTIFY FIREWALL]\n\nRequest Blocked!\n\nReason: ${data.explanation.summary}`);
            
            // Clear the malicious prompt from the box
            if (isTextArea || isTextInput) {
              target.value = "";
            } else {
              target.innerText = "";
            }
            
            // Dispatch an input event so JS frameworks (like React) register the clear
            target.dispatchEvent(new Event('input', { bubbles: true }));
            
          } else if (data.decision === "Warn") {
            const proceed = confirm(`[PROMPTIFY FIREWALL]\n\nWarning: ${data.explanation.summary}\n\nDo you want to proceed?`);
            if (proceed) {
              submitOriginalEvent(target);
            }
          } else {
            // Allow: It's clean, submit it automatically
            submitOriginalEvent(target);
          }
        } else {
          console.error("[Promptify] Local proxy may be down. Allowing request by default.", response?.error);
          submitOriginalEvent(target);
        }
      } catch (err) {
        console.error("[Promptify] Error communicating with background script:", err);
        target.style.opacity = originalOpacity;
        submitOriginalEvent(target);
      }
    }
  }
}, true); // Use the capturing phase to intercept before React/Vue/Angular handlers

/**
 * Re-dispatches the Enter keypress to let the site natively submit the prompt.
 */
function submitOriginalEvent(target) {
  isSynthesizingEnter = true;
  
  // Create a nearly identical Enter keydown event
  const enterEvent = new KeyboardEvent('keydown', {
    key: 'Enter',
    code: 'Enter',
    keyCode: 13,
    which: 13,
    bubbles: true,
    cancelable: true,
    composed: true // Crucial for shadow DOMs
  });

  // Dispatch it on the original target
  target.dispatchEvent(enterEvent);
  
  // Reset the flag immediately after dispatch
  isSynthesizingEnter = false;
}
