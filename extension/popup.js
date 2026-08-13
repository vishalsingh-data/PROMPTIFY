document.addEventListener('DOMContentLoaded', () => {
  const statusText = document.getElementById('status-text');
  const statusDot = document.getElementById('status-dot');

  // Ping the local proxy health endpoint
  fetch('http://127.0.0.1:11433/health')
    .then(response => {
      if (response.ok) {
        statusDot.classList.remove('offline');
        statusDot.classList.add('online');
        statusText.textContent = 'Proxy is Online';
      } else {
        throw new Error('Not OK');
      }
    })
    .catch(() => {
      statusDot.classList.remove('online');
      statusDot.classList.add('offline');
      statusText.textContent = 'Proxy is Offline';
    });
});
