export type Theme = "system" | "light" | "dark";
const key = "open-faktura-theme";
export function storedTheme(): Theme {
  try {
    const value = localStorage.getItem(key);
    return value === "dark" || value === "light" ? value : "system";
  } catch {
    return "system";
  }
}
export function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme =
    theme === "system"
      ? matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : theme;
  try {
    localStorage.setItem(key, theme);
  } catch {
    /* Appearance still applies when storage is unavailable. */
  }
}
export function initializeTheme() {
  applyTheme(storedTheme());
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (storedTheme() === "system") applyTheme("system");
  });
}
