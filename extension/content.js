/**
 * PROMPTIFY CONTENT SCRIPT (Generic UI Interceptor with Modal Popup)
 * 
 * This script listens for Enter keypresses globally on any text input element.
 * It uses a centered, fixed-position Shadow DOM modal to display warnings 
 * or blocks. This ensures we don't break the host site's DOM (like ProseMirror).
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
        console.log("[Promptify] Background response:", response);

        if (response && response.success) {
          const data = response.data;
          
          if (data.decision === "Block") {
            // Clear the malicious prompt from the box initially
            if (isTextArea || isTextInput) {
              target.value = "";
            } else {
              target.innerText = "";
            }
            target.dispatchEvent(new Event('input', { bubbles: true }));
            
            showModalPanel(target, data, promptText, "Block");
            
          } else if (data.decision === "Warn") {
            // Warn still interrupts the flow to ask for permission
            showModalPanel(target, data, promptText, "Warn");
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
// CENTERED MODAL UI INJECTION
// ==========================================

function showModalPanel(target, data, originalPrompt, type) {
  // Create a host element for the Shadow DOM appended to body
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
        position: fixed;
        top: 0; left: 0; width: 100vw; height: 100vh;
        background: rgba(0,0,0,0.6);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 2147483647; /* max z-index */
        backdrop-filter: blur(4px);
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      }
      .modal {
        background: #1e1e2e;
        color: #fff;
        width: 400px;
        border-radius: 12px;
        box-shadow: 0 10px 30px rgba(0,0,0,0.5);
        overflow: hidden;
        border: 1px solid rgba(255,255,255,0.1);
        animation: scaleIn 0.2s ease-out;
      }
      @keyframes scaleIn {
        from { transform: scale(0.95); opacity: 0; }
        to { transform: scale(1); opacity: 1; }
      }
      .header {
        background: ${colorPrimary};
        padding: 16px 20px;
        font-weight: bold;
        font-size: 16px;
        display: flex;
        align-items: center;
        border-bottom: 2px solid ${colorSecondary};
      }
      .body {
        padding: 20px;
      }
      .summary {
        font-size: 15px;
        margin-bottom: 16px;
        line-height: 1.5;
      }
      .score {
        display: inline-block;
        background: rgba(255,255,255,0.1);
        padding: 4px 8px;
        border-radius: 4px;
        font-size: 12px;
        margin-bottom: 16px;
        font-weight: bold;
      }
      .reasons {
        background: rgba(0,0,0,0.2);
        padding: 12px;
        border-radius: 6px;
        font-size: 13px;
        color: #ccc;
        margin-bottom: 20px;
      }
      .reasons div { margin-bottom: 6px; }
      .reasons div:last-child { margin-bottom: 0; }
      .footer {
        display: flex;
        gap: 12px;
        justify-content: flex-end;
      }
      button {
        border: none;
        padding: 10px 16px;
        border-radius: 6px;
        cursor: pointer;
        font-weight: bold;
        font-size: 14px;
        transition: all 0.2s;
      }
      .btn-cancel {
        background: transparent;
        color: #fff;
        border: 1px solid rgba(255,255,255,0.2);
      }
      .btn-cancel:hover {
        background: rgba(255,255,255,0.1);
      }
      .btn-override {
        background: ${colorPrimary};
        color: #fff;
      }
      .btn-override:hover {
        background: ${colorSecondary};
      }
      .btn-override.confirming {
        background: #e91e63;
      }
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

  btnCancel.addEventListener('click', () => {
    host.remove();
  });

  let overrideConfirmState = false;
  
  btnOverride.addEventListener('click', () => {
    if (!overrideConfirmState) {
      // First click: ask for confirmation
      overrideConfirmState = true;
      btnOverride.textContent = "Click to Confirm";
      btnOverride.classList.add('confirming');
    } else {
      // Second click: override and submit
      host.remove();
      
      // Restore the original text to the box if it was cleared
      if (isBlock) {
        const isTextArea = target.tagName === "TEXTAREA";
        const isTextInput = target.tagName === "INPUT" && target.type === "text";
        if (isTextArea || isTextInput) {
          target.value = originalPrompt;
        } else {
          target.innerText = originalPrompt;
        }
        target.dispatchEvent(new Event('input', { bubbles: true }));
      }

      // Wait a tiny bit for JS frameworks to catch up with the text restoration
      setTimeout(() => {
        submitOriginalEvent(target);
      }, 50);
    }
  });
}
