document.addEventListener('DOMContentLoaded', () => {
  const statusText = document.getElementById('status-text');
  const statusDot = document.getElementById('status-dot');
  
  const elAnalyzed = document.getElementById('stat-analyzed');
  const elWarned = document.getElementById('stat-warned');
  const elBlocked = document.getElementById('stat-blocked');

  // Fetch initial stats from background script
  chrome.runtime.sendMessage({ type: "GET_STATS" }, (response) => {
    if (response && response.success) {
      elAnalyzed.textContent = response.stats.analyzed;
      elWarned.textContent = response.stats.warned;
      elBlocked.textContent = response.stats.blocked;
    }
  });

  function checkConnection() {
    fetch('http://127.0.0.1:11433/health')
      .then(response => {
        if (response.ok) {
          statusDot.classList.remove('offline');
          statusDot.classList.add('online');
          statusText.textContent = 'Proxy is Online';
          statusText.style.color = '#4CAF50';
        } else {
          throw new Error('Not OK');
        }
      })
      .catch(() => {
        statusDot.classList.remove('online');
        statusDot.classList.add('offline');
        statusText.textContent = 'Proxy is Offline';
        statusText.style.color = '#F44336';
      });
  }

  // Initial check
  checkConnection();

  // Poll every 2 seconds while the popup is open
  setInterval(checkConnection, 2000);
});
