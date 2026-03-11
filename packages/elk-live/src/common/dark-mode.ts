const STORAGE_KEY = 'elk-rs-live-dark-mode';

export function setupDarkMode(): void {
  const toggle = document.getElementById('darkModeToggle');
  if (!toggle) return;

  const isDark = localStorage.getItem(STORAGE_KEY) === 'true';
  if (isDark) document.body.classList.add('elk-dark-mode');

  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'dark-mode-btn';
  btn.textContent = isDark ? '☀️' : '🌙';
  btn.title = 'Toggle dark mode';
  btn.onclick = () => {
    const active = document.body.classList.toggle('elk-dark-mode');
    localStorage.setItem(STORAGE_KEY, String(active));
    btn.textContent = active ? '☀️' : '🌙';
  };
  toggle.appendChild(btn);
}
