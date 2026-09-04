// Theme is one class on <html> and one localStorage key. No component knows about it.
// The tokens in index.css do the rest, so dark is the same components with different values.
export type Theme = "light" | "dark" | "system"

const KEY = "surya.theme"
const media = () => window.matchMedia("(prefers-color-scheme: dark)")

export function readTheme(): Theme {
  const saved = localStorage.getItem(KEY)
  return saved === "light" || saved === "dark" || saved === "system" ? saved : "system"
}

export function applyTheme(theme: Theme) {
  const dark = theme === "dark" || (theme === "system" && media().matches)
  document.documentElement.classList.toggle("dark", dark)
}

export function setTheme(theme: Theme) {
  localStorage.setItem(KEY, theme)
  applyTheme(theme)
}

// Called once from main.tsx so every route loads in the saved theme, not just Settings.
// While the choice is System, the OS switching at night flips the app with it.
export function applyStoredTheme() {
  applyTheme(readTheme())
  media().addEventListener("change", () => readTheme() === "system" && applyTheme("system"))
}
