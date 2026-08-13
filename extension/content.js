/**
 * PROMPTIFY CONTENT SCRIPT (Generic UI Interceptor with Inline UI)
 * 
 * This script listens for Enter keypresses globally on any text input element.
 * It injects a Shadow DOM element to display warnings or blocks directly above
 * the text input box without relying on native browser alerts.
 */

console.log("[Promptify] Generic UI Interceptor loaded on", window.location.hostname);

let isSynthesizingEnter = false;

document.body.addEventListener("keydown", async (event) => {
  if (isSynthesizingEnter) return;

  if (event.key === "Enter" && !event.shiftKey) {
    const target = event.target;

    const isTextArea = target.tagName === "TEXTAREA";
    const isTextInput = target.tagName === "INPUT" && target.type === "text";
    const isContentEditable = target.isContentEditable;

    if (isTextArea || isTextInput || isContentEditable) {
      let promptText = "";
      if (isTextArea || isTextInput) {
        promptText = target.value;
      } else if (isContentEditable) {
        promptText = target.innerText || target.textContent;
      }

      promptText = promptText ? promptText.trim() : "";
      
      if (!promptText) return;

      console.log("[Promptify] Intercepted prompt:", promptText);

      // PREVENT the chat site from submitting the prompt
      event.preventDefault();
      event.stopImmediatePropagation();

      const originalOpacity = target.style.opacity;
      target.style.opacity = "0.5";

      try {
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
            injectBlockPanel(target, data, promptText);
            
            // Clear the malicious prompt from the box initially
            if (isTextArea || isTextInput) {
              target.value = "";
            } else {
              target.innerText = "";
            }
            target.dispatchEvent(new Event('input', { bubbles: true }));
            
          } else if (data.decision === "Warn") {
            injectWarnBanner(target, data);
            submitOriginalEvent(target);
          } else {
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
}, true);


function submitOriginalEvent(target) {
  isSynthesizingEnter = true;
  
  const enterEvent = new KeyboardEvent('keydown', {
    key: 'Enter',
    code: 'Enter',
    keyCode: 13,
    which: 13,
    bubbles: true,
    cancelable: true,
    composed: true
  });

  target.dispatchEvent(enterEvent);
  isSynthesizingEnter = false;
}

// ==========================================
// INLINE UI INJECTION LOGIC
// ==========================================

function createShadowContainer(target) {
  // Find a stable parent to attach our UI to
  const parent = target.parentElement;
  
  // Create a host element for the Shadow DOM
  const host = document.createElement('div');
  host.style.position = 'absolute';
  host.style.zIndex = '999999';
  host.style.width = '100%';
  host.style.pointerEvents = 'none'; // let clicks pass through the host itself
  
  // Position it relatively above the input target
  // We insert it right before the target
  parent.style.position = parent.style.position || 'relative';
  parent.insertBefore(host, target);
  
  const shadowRoot = host.attachShadow({ mode: 'open' });
  return { host, shadowRoot };
}

function injectWarnBanner(target, data) {
  const { host, shadowRoot } = createShadowContainer(target);
  host.style.bottom = '100%'; // position above the input box
  host.style.marginBottom = '10px';

  shadowRoot.innerHTML = `
    <style>
      .promptify-warn {
        background-color: #ff9800;
        color: #fff;
        padding: 8px 12px;
        border-radius: 6px;
        font-family: sans-serif;
        font-size: 12px;
        box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        pointer-events: auto;
        display: inline-block;
        animation: fadeOut 3s forwards;
        animation-delay: 4s;
      }
      @keyframes fadeOut {
        to { opacity: 0; visibility: hidden; }
      }
    </style>
    <div class="promptify-warn">
      ⚠️ <b>Warning (Score: ${data.risk_score}):</b> ${data.explanation.summary}
    </div>
  `;

  // Auto-remove the host after animation completes
  setTimeout(() => host.remove(), 7500);
}

function injectBlockPanel(target, data, originalPrompt) {
  // Remove any existing block panels for this target
  if (target._promptifyBlockHost) {
    target._promptifyBlockHost.remove();
  }

  const { host, shadowRoot } = createShadowContainer(target);
  host.style.bottom = '100%';
  host.style.marginBottom = '10px';
  host.style.pointerEvents = 'auto'; // Block panels need interaction
  
  target._promptifyBlockHost = host;

  shadowRoot.innerHTML = `
    <style>
      .promptify-block {
        background-color: #f44336;
        color: #fff;
        padding: 16px;
        border-radius: 8px;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
        font-size: 14px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.2);
        max-width: 400px;
        border: 1px solid #d32f2f;
      }
      .promptify-block h3 {
        margin: 0 0 8px 0;
        font-size: 16px;
        display: flex;
        align-items: center;
        gap: 6px;
      }
      .promptify-block p {
        margin: 0 0 12px 0;
        line-height: 1.4;
      }
      .promptify-reasons {
        background: rgba(0,0,0,0.1);
        padding: 8px;
        border-radius: 4px;
        margin-bottom: 12px;
        font-size: 12px;
      }
      .promptify-btn-group {
        display: flex;
        gap: 8px;
        justify-content: flex-end;
      }
      button {
        border: none;
        padding: 8px 12px;
        border-radius: 4px;
        cursor: pointer;
        font-weight: bold;
        transition: background 0.2s;
      }
      .btn-cancel {
        background: #fff;
        color: #f44336;
      }
      .btn-cancel:hover {
        background: #f1f1f1;
      }
      .btn-override {
        background: transparent;
        color: #fff;
        border: 1px solid rgba(255,255,255,0.5);
      }
      .btn-override:hover {
        background: rgba(255,255,255,0.1);
      }
      .btn-override.confirming {
        background: #ff9800;
        border-color: #ff9800;
        color: #fff;
      }
    </style>
    <div class="promptify-block">
      <h3>🛡️ Promptify Firewall Blocked Request</h3>
      <p><b>Score: ${data.risk_score}</b> - ${data.explanation.summary}</p>
      <div class="promptify-reasons">
        ${data.explanation.reasons.map(r => `<div>• ${r}</div>`).join('')}
      </div>
      <div class="promptify-btn-group">
        <button class="btn-cancel" id="btn-cancel">Discard Prompt</button>
        <button class="btn-override" id="btn-override">Submit Anyway</button>
      </div>
    </div>
  `;

  const btnCancel = shadowRoot.getElementById('btn-cancel');
  const btnOverride = shadowRoot.getElementById('btn-override');

  btnCancel.addEventListener('click', () => {
    host.remove();
    target._promptifyBlockHost = null;
  });

  let overrideConfirmState = false;
  
  btnOverride.addEventListener('click', () => {
    if (!overrideConfirmState) {
      // First click: ask for confirmation
      overrideConfirmState = true;
      btnOverride.textContent = "Are you sure? Click again.";
      btnOverride.classList.add('confirming');
    } else {
      // Second click: override and submit
      host.remove();
      target._promptifyBlockHost = null;
      
      // Restore the original text to the box
      const isTextArea = target.tagName === "TEXTAREA";
      const isTextInput = target.tagName === "INPUT" && target.type === "text";
      if (isTextArea || isTextInput) {
        target.value = originalPrompt;
      } else {
        target.innerText = originalPrompt;
      }
      target.dispatchEvent(new Event('input', { bubbles: true }));

      // Wait a tiny bit for JS frameworks to catch up with the text restoration
      setTimeout(() => {
        submitOriginalEvent(target);
      }, 50);
    }
  });
}
