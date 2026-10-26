// The viewer's theme: `system` follows the browser's `prefers-color-scheme`, and `dark` or `light`
// fixes one. The choice is kept in `localStorage` and written to `<html data-theme>`, which
// `style.css` reads; `system` removes the attribute.

import { createSignal } from "solid-js";

export type Theme = "system" | "dark" | "light";

/** The `localStorage` key of the fixed theme. */
const KEY = "novis-debug-theme";

function stored(): Theme {
  try {
    const value = localStorage.getItem(KEY);
    return value === "dark" || value === "light" ? value : "system";
  } catch {
    return "system";
  }
}

const [theme, setThemeSignal] = createSignal<Theme>(stored());
apply(theme());

export { theme };

/** Fixes `next`, or follows the browser again with `system`. */
export function setTheme(next: Theme): void {
  setThemeSignal(next);
  apply(next);
  try {
    if (next === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, next);
  } catch {
    // A browser that blocks storage keeps the choice for this tab alone.
  }
}

/** The next theme in the order the `t` key walks them. */
export function nextTheme(): Theme {
  return theme() === "system" ? "dark" : theme() === "dark" ? "light" : "system";
}

function apply(value: Theme): void {
  if (value === "system") delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = value;
}
