export function getParams(): Record<string, string> {
  return Object.fromEntries(new URLSearchParams(window.location.search));
}
