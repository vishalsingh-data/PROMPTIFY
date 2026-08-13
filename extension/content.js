/**
 * PROMPTIFY CONTENT SCRIPT (Generic UI Interceptor with Modal Popup)
 * 
 * Intercepts Enter keypresses AND Send Button clicks across any LLM site.
 */

console.log("[Promptify] Generic UI Interceptor loaded on", window.location.hostname);

let isSynthesizingEvent = false;
let lastActiveInput = null;

// Track the most recently used text input
document.body.addEventListener("focusin", (event) => {
  const target = event.target;
  if (target.tagName === "TEXTAREA" || (target.tagName === "INPUT" && target.type === "text") || target.isContentEditable) {
    lastActiveInput = target;
  }
});
document.body.addEventListener("input", (event) => {
  const target = event.target;
  if (target.tagName === "TEXTAREA" || (target.tagName === "INPUT" && target.type === "text") || target.isContentEditable) {
    lastActiveInput = target;
  }
});

// 1. Intercept Enter Key
document.body.addEventListener("keydown", async (event) => {
  if (isSynthesizingEvent) return;

  if (event.key === "Enter" && !event.shiftKey) {
    const target = event.target;
    const isTextArea = target.tagName === "TEXTAREA";
    const isTextInput = target.tagName === "INPUT" && target.type === "text";
    const isContentEditable = target.isContentEditable;

    if (isTextArea || isTextInput || isContentEditable) {
      await handlePromptSubmission(event, target, () => {
        submitOriginalKeyEvent(target);
      });
    }
  }
}, true);

// 2. Intercept Mouse Clicks on Generic "Send" buttons
document.body.addEventListener("mousedown", async (event) => {
  if (isSynthesizingEvent) return;
  if (!lastActiveInput) return;

  // Find if the clicked element (or its parents) looks like a submit button
  const clickedBtn = event.target.closest('button, [role="button"], [aria-label*="end"], [aria-label*="ubmit"], [data-testid*="send"]');
  
  if (clickedBtn) {
    // Check if this button is physically close to our lastActiveInput in the DOM tree
    // This generic heuristic ensures we only intercept the send button for the chat, not a random button
    if (isNodeCloselyRelated(lastActiveInput, clickedBtn, 6)) {
      
      // If the input is empty, no need to intercept
      let promptText = getInputValue(lastActiveInput);
      if (!promptText) return;

      // Intercept the click!
      await handlePromptSubmission(event, lastActiveInput, () => {
        submitOriginalClickEvent(clickedBtn);
      });
    }
  }
}, true); // Capturing phase to catch it before React onClick handlers

/**
 * Checks if two DOM nodes share a common ancestor within a certain depth.
 * This is a highly robust generic way to check if a Send button belongs to a Chat input!
 */
function isNodeCloselyRelated(node1, node2, maxDepth) {
  let ancestor1 = node1;
  for (let i = 0; i < maxDepth && ancestor1; i++) {
    let ancestor2 = node2;
    for (let j = 0; j < maxDepth && ancestor2; j++) {
      if (ancestor1 === ancestor2) return true;
      ancestor2 = ancestor2.parentElement;
    }
    ancestor1 = ancestor1.parentElement;
  }
  return false;
}

function getInputValue(target) {
  let text = "";
  if (target.tagName === "TEXTAREA" || target.tagName === "INPUT") {
    text = target.value;
  } else if (target.isContentEditable) {
    text = target.innerText || target.textContent;
  }
  return text ? text.trim() : "";
}

function setInputValue(target, text) {
  if (target.tagName === "TEXTAREA" || target.tagName === "INPUT") {
    target.value = text;
  } else {
    target.innerText = text;
  }
  target.dispatchEvent(new Event('input', { bubbles: true }));
}

/**
 * Core generic submission handler logic
 */
async function handlePromptSubmission(originalEvent, inputTarget, successCallback) {
  let promptText = getInputValue(inputTarget);
  if (!promptText) return;

  console.log("[Promptify] Intercepted prompt submission:", promptText);

  // Stop the original event (Enter key or Mouse click)
  originalEvent.preventDefault();
  originalEvent.stopImmediatePropagation();

  const originalOpacity = inputTarget.style.opacity;
  inputTarget.style.opacity = "0.5";

  try {
    const response = await new Promise((resolve) => {
      chrome.runtime.sendMessage(
        { type: "ANALYZE_PROMPT", prompt: promptText, source_url: window.location.href },
        resolve
      );
    });

    inputTarget.style.opacity = originalOpacity;

    if (response && response.success) {
      const data = response.data;
      if (data.decision === "Block") {
        setInputValue(inputTarget, "");
        showModalPanel(inputTarget, data, promptText, "Block", successCallback);
      } else if (data.decision === "Warn") {
        showModalPanel(inputTarget, data, promptText, "Warn", successCallback);
      } else {
        successCallback();
      }
    } else {
      console.error("[Promptify] Local proxy down, allowing.", response?.error);
      successCallback();
    }
  } catch (err) {
    console.error("[Promptify] Error:", err);
    inputTarget.style.opacity = originalOpacity;
    successCallback();
  }
}

function submitOriginalKeyEvent(target) {
  isSynthesizingEvent = true;
  const enterEvent = new KeyboardEvent('keydown', {
    key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true, cancelable: true, composed: true
  });
  target.dispatchEvent(enterEvent);
  isSynthesizingEvent = false;
}

function submitOriginalClickEvent(target) {
  isSynthesizingEvent = true;
  // Dispatch a full click sequence just to be safe with React
  target.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, composed: true }));
  target.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, cancelable: true, composed: true }));
  target.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, composed: true }));
  isSynthesizingEvent = false;
}

// ==========================================
// CENTERED MODAL UI INJECTION
// ==========================================

function showModalPanel(target, data, originalPrompt, type, successCallback) {
  const host = document.createElement('div');
  document.body.appendChild(host);
  const shadowRoot = host.attachShadow({ mode: 'open' });
  
  const isBlock = type === "Block";
  const titleText = isBlock ? "🛡️ Firewall Blocked Request" : "⚠️ Firewall Warning";
  const colorPrimary = isBlock ? "#f44336" : "#ff9800";
  const colorSecondary = isBlock ? "#d32f2f" : "#f57c00";

  shadowRoot.innerHTML = `
    <style>
      .overlay {
        position: fixed; top: 0; left: 0; width: 100vw; height: 100vh;
        background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center;
        z-index: 2147483647; backdrop-filter: blur(4px);
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      }
      .modal {
        background: #1e1e2e; color: #fff; width: 400px; border-radius: 12px;
        box-shadow: 0 10px 30px rgba(0,0,0,0.5); overflow: hidden;
        border: 1px solid rgba(255,255,255,0.1); animation: scaleIn 0.2s ease-out;
      }
      @keyframes scaleIn { from { transform: scale(0.95); opacity: 0; } to { transform: scale(1); opacity: 1; } }
      .header {
        background: ${colorPrimary}; padding: 16px 20px; font-weight: bold; font-size: 16px;
        border-bottom: 2px solid ${colorSecondary};
      }
      .body { padding: 20px; }
      .summary { font-size: 15px; margin-bottom: 16px; line-height: 1.5; }
      .score {
        display: inline-block; background: rgba(255,255,255,0.1); padding: 4px 8px;
        border-radius: 4px; font-size: 12px; margin-bottom: 16px; font-weight: bold;
      }
      .reasons {
        background: rgba(0,0,0,0.2); padding: 12px; border-radius: 6px; font-size: 13px;
        color: #ccc; margin-bottom: 20px;
      }
      .reasons div { margin-bottom: 6px; }
      .footer { display: flex; gap: 12px; justify-content: flex-end; }
      button {
        border: none; padding: 10px 16px; border-radius: 6px; cursor: pointer;
        font-weight: bold; font-size: 14px; transition: all 0.2s;
      }
      .btn-cancel { background: transparent; color: #fff; border: 1px solid rgba(255,255,255,0.2); }
      .btn-cancel:hover { background: rgba(255,255,255,0.1); }
      .btn-override { background: ${colorPrimary}; color: #fff; }
      .btn-override:hover { background: ${colorSecondary}; }
      .btn-override.confirming { background: #e91e63; }
    </style>
    <div class="overlay">
      <div class="modal">
        <div class="header">${titleText}</div>
        <div class="body">
          <div class="score">Risk Score: ${data.risk_score}</div>
          <div class="summary">${data.explanation.summary}</div>
          <div class="reasons">
            ${data.explanation.reasons.map(r => `<div>• ${r}</div>`).join('')}
          </div>
          <div class="footer">
            <button class="btn-cancel" id="btn-cancel">${isBlock ? "Discard Prompt" : "Cancel"}</button>
            <button class="btn-override" id="btn-override">Submit Anyway</button>
          </div>
        </div>
      </div>
    </div>
  `;

  const btnCancel = shadowRoot.getElementById('btn-cancel');
  const btnOverride = shadowRoot.getElementById('btn-override');

  btnCancel.addEventListener('click', () => host.remove());

  let overrideConfirmState = false;
  btnOverride.addEventListener('click', () => {
    if (!overrideConfirmState) {
      overrideConfirmState = true;
      btnOverride.textContent = "Click to Confirm";
      btnOverride.classList.add('confirming');
    } else {
      host.remove();
      if (isBlock) setInputValue(target, originalPrompt);
      setTimeout(() => successCallback(), 50);
    }
  });
}
