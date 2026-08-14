/**
 * PROMPTIFY CONTENT SCRIPT (Generic UI Interceptor & Incoming Response Analyzer)
 * 
 * Intercepts Enter keypresses AND Send Button clicks across any LLM site.
 * Observes the DOM to generically detect when incoming LLM responses finish 
 * generating, and analyzes them for malicious content.
 */

console.log("[Promptify] Generic UI Interceptor loaded on", window.location.hostname);

let isSynthesizingEvent = false;
let lastActiveInput = null;
let isAwaitingResponse = false;

// ==========================================
// FLOATING MASCOT WIDGET
// ==========================================
let floatingWidget = null;
function initFloatingWidget() {
  if (document.getElementById('promptify-floating-widget')) return;
  
  floatingWidget = document.createElement('div');
  floatingWidget.id = 'promptify-floating-widget';
  Object.assign(floatingWidget.style, {
    position: 'fixed',
    bottom: '20px',
    left: '20px',
    zIndex: '2147483646',
    transition: 'all 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275)',
    cursor: 'help',
    filter: 'drop-shadow(0 4px 12px rgba(0,0,0,0.5))'
  });

  const mascotImg = document.createElement('img');
  mascotImg.src = chrome.runtime.getURL('mascot.svg');
  mascotImg.style.width = '64px';
  mascotImg.style.height = '64px';
  mascotImg.style.transition = 'all 0.3s';
  mascotImg.id = 'promptify-mascot-img';
  
  // Add hover effect
  floatingWidget.addEventListener('mouseenter', () => mascotImg.style.transform = 'scale(1.1) translateY(-5px)');
  floatingWidget.addEventListener('mouseleave', () => mascotImg.style.transform = 'scale(1) translateY(0)');

  floatingWidget.appendChild(mascotImg);
  document.body.appendChild(floatingWidget);
}

// Initialize immediately
initFloatingWidget();

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

  const clickedBtn = event.target.closest('button, [role="button"], [aria-label*="end"], [aria-label*="ubmit"], [data-testid*="send"]');
  
  if (clickedBtn) {
    if (isNodeCloselyRelated(lastActiveInput, clickedBtn, 6)) {
      let promptText = getInputValue(lastActiveInput);
      if (!promptText) return;

      await handlePromptSubmission(event, lastActiveInput, () => {
        submitOriginalClickEvent(clickedBtn);
      });
    }
  }
}, true);

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

async function handlePromptSubmission(originalEvent, inputTarget, successCallback) {
  let promptText = getInputValue(inputTarget);
  if (!promptText) return;

  console.log("[Promptify] Intercepted prompt submission:", promptText);
  originalEvent.preventDefault();
  originalEvent.stopImmediatePropagation();

  const originalOpacity = inputTarget.style.opacity;
  inputTarget.style.opacity = "0.5";

  try {
    const response = await new Promise((resolve) => {
      chrome.runtime.sendMessage(
        { type: "ANALYZE_PROMPT", prompt: promptText, source_url: window.location.href, event_type: "outgoing" },
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
        console.log("[Promptify] Prompt allowed. Awaiting LLM response...");
        isAwaitingResponse = true; // Tell the observer to watch for the LLM's response
        successCallback();
      }
    } else {
      console.error("[Promptify] Local proxy down, allowing.", response?.error);
      isAwaitingResponse = true;
      successCallback();
    }
  } catch (err) {
    console.error("[Promptify] Error:", err);
    inputTarget.style.opacity = originalOpacity;
    isAwaitingResponse = true;
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
  target.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, composed: true }));
  target.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, cancelable: true, composed: true }));
  target.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, composed: true }));
  isSynthesizingEvent = false;
}

// ==========================================
// OUTGOING MODAL UI INJECTION
// ==========================================

function showModalPanel(target, data, originalPrompt, type, successCallback) {
  if (floatingWidget) floatingWidget.style.opacity = '0';

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
        border: 1px solid rgba(255,255,255,0.1); 
        animation: flyIn 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275) forwards;
        transform-origin: bottom left;
      }
      @keyframes flyIn { 
        0% { transform: translate(-40vw, 40vh) scale(0.1); opacity: 0; } 
        100% { transform: translate(0, 0) scale(1); opacity: 1; } 
      }
      .header {
        background: ${colorPrimary}; padding: 16px 20px; font-weight: bold; font-size: 16px;
        border-bottom: 2px solid ${colorSecondary}; display: flex; align-items: center;
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
        <div class="header">
          <img src="${chrome.runtime.getURL('mascot.svg')}" alt="Promptify" style="width: 28px; height: 28px; margin-right: 12px; filter: drop-shadow(0 2px 4px rgba(0,0,0,0.3));">
          ${titleText}
        </div>
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

  shadowRoot.getElementById('btn-cancel').addEventListener('click', () => {
    host.remove();
    if (floatingWidget) floatingWidget.style.opacity = '1';
  });

  let overrideConfirmState = false;
  const btnOverride = shadowRoot.getElementById('btn-override');
  btnOverride.addEventListener('click', () => {
    if (!overrideConfirmState) {
      overrideConfirmState = true;
      btnOverride.textContent = "Click to Confirm";
      btnOverride.classList.add('confirming');
    } else {
      host.remove();
      if (floatingWidget) floatingWidget.style.opacity = '1';
      if (isBlock) setInputValue(target, originalPrompt);
      console.log("[Promptify] Prompt override submitted. Awaiting LLM response...");
      isAwaitingResponse = true;
      setTimeout(() => successCallback(), 50);
    }
  });
}

// ==========================================
// INCOMING RESPONSE ANALYZER (DOM MUTATION)
// ==========================================

let responseDebounceTimer = null;
let activelyMutatingElement = null;

const observer = new MutationObserver((mutations) => {
  if (!isAwaitingResponse) return;

  // Find the deepest element that is actively receiving text
  for (const mutation of mutations) {
    if (mutation.type === "characterData" || mutation.type === "childList") {
      let target = mutation.target;
      if (target.nodeType === Node.TEXT_NODE) {
        target = target.parentElement;
      }
      
      // Specifically target likely LLM output containers. 
      // If the mutation happens outside these (like a sidebar updating), IGNORE IT!
      const likelyContainer = target.closest('[data-message-author-role="assistant"], .markdown');
      if (likelyContainer) {
        activelyMutatingElement = likelyContainer;
      }
    }
  }

  // If no valid LLM container mutated, ignore
  if (!activelyMutatingElement) return;

  // Reset the timer. If 1500ms passes without DOM changes, we assume generation is finished!
  clearTimeout(responseDebounceTimer);
  responseDebounceTimer = setTimeout(() => {
    finishResponseGeneration();
  }, 1500);
});

observer.observe(document.body, { childList: true, subtree: true, characterData: true });

async function finishResponseGeneration() {
  if (!isAwaitingResponse || !activelyMutatingElement) return;
  
  isAwaitingResponse = false; // Reset flag so we don't double-analyze
  
  // Walk up a bit to get the full message container if we're only tracking a paragraph
  let responseContainer = activelyMutatingElement.closest('[data-message-author-role="assistant"], .markdown') || activelyMutatingElement;
  
  const responseText = responseContainer.innerText || responseContainer.textContent;
  
  console.log(`[Promptify] Extracted Response (${responseText?.length || 0} chars):`, responseText);

  if (!responseText || responseText.length < 5) return;

  console.log("[Promptify] LLM Response Finished Generating. Sending to local proxy for analysis...");

  try {
    const res = await new Promise((resolve) => {
      chrome.runtime.sendMessage(
        { type: "ANALYZE_PROMPT", prompt: responseText, source_url: window.location.href, event_type: "incoming" },
        resolve
      );
    });

    console.log("[Promptify] Incoming Response Decision:", res?.data?.decision);

    if (res && res.success) {
      if (res.data.decision === "Block" || res.data.decision === "Warn") {
        censorIncomingResponse(responseContainer, res.data);
      }
    }
  } catch (err) {
    console.error("[Promptify] Error analyzing response:", err);
  }
}

function censorIncomingResponse(element, data) {
  if (floatingWidget) floatingWidget.style.opacity = '0';

  // Visually blur and block out the malicious text in the DOM
  element.style.position = "relative";
  
  // Wrap existing content to blur it
  const wrapper = document.createElement('div');
  wrapper.style.filter = "blur(8px)";
  wrapper.style.userSelect = "none";
  wrapper.style.pointerEvents = "none";
  
  // Move all children into wrapper
  while (element.firstChild) {
    wrapper.appendChild(element.firstChild);
  }
  element.appendChild(wrapper);

  // Create an overlay warning box
  const overlay = document.createElement('div');
  overlay.style.position = "absolute";
  overlay.style.top = "50%";
  overlay.style.left = "50%";
  overlay.style.transform = "translate(-50%, -50%) scale(0.1)";
  overlay.style.opacity = "0";
  overlay.style.background = "rgba(244, 67, 54, 0.95)";
  overlay.style.color = "#fff";
  overlay.style.padding = "16px 24px";
  overlay.style.borderRadius = "8px";
  overlay.style.boxShadow = "0 8px 24px rgba(0,0,0,0.5)";
  overlay.style.textAlign = "center";
  overlay.style.fontFamily = "-apple-system, sans-serif";
  overlay.style.zIndex = "10";
  overlay.style.minWidth = "250px";
  overlay.style.transition = "all 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275)";
  
  // Trigger animation after append
  setTimeout(() => {
    overlay.style.transform = "translate(-50%, -50%) scale(1)";
    overlay.style.opacity = "1";
  }, 50);

  overlay.innerHTML = `
    <div style="margin-bottom: 12px; display: flex; justify-content: center;">
      <img src="${chrome.runtime.getURL('mascot.svg')}" alt="Promptify Mascot" style="width: 48px; height: 48px; filter: drop-shadow(0 0 8px rgba(0,0,0,0.3));">
    </div>
    <h3 style="margin: 0 0 8px 0; font-size: 16px;">Malicious Response Blocked</h3>
    <p style="margin: 0 0 16px 0; font-size: 13px; opacity: 0.9;">Score: ${data.risk_score} | ${data.explanation.summary}</p>
    <button id="promptify-reveal-btn" style="background: #fff; color: #f44336; border: none; padding: 8px 16px; border-radius: 4px; font-weight: bold; cursor: pointer;">Reveal Anyway</button>
  `;

  element.appendChild(overlay);

  overlay.querySelector('#promptify-reveal-btn').addEventListener('click', () => {
    overlay.remove();
    if (floatingWidget) floatingWidget.style.opacity = '1';
    wrapper.style.filter = "none";
    wrapper.style.userSelect = "auto";
    wrapper.style.pointerEvents = "auto";
  });
}
