// The viewer's key map. Every key the viewer answers is in it once, with what it does, and the `?`
// dialog prints it. The frame binds its keys here with a handler (`app.tsx`). A view lists the keys it
// answers in `VIEW_KEYS`, and runs `ignored()` first in its own handler, so a key typed into a field or
// meant for a button keeps its own meaning.
//
// A key bound twice throws when it is bound, so two views cannot give one key two meanings.

import { createSignal } from "solid-js";

/** One line of the map: the keys, what they do, and the handler when the frame runs it. */
export interface Binding {
  /** The names `KeyboardEvent.key` gives, or `Mod+k` for Ctrl on Windows and Linux and Cmd on macOS. */
  keys: string[];
  /** What the key does, as the `?` dialog prints it. */
  action: string;
  /** Runs the key. A binding without one documents a key a view's own handler answers. */
  run?: (e: KeyboardEvent) => void;
}

/** True on macOS, where `Mod` is the Command key. */
export const MAC = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);

/** The keys the views answer in their own handlers, in the order the `?` dialog lists them. */
export const VIEW_KEYS: readonly Binding[] = [
  { keys: ["j", "ArrowDown"], action: "Select the next row, or go to the next search result" },
  { keys: ["k", "ArrowUp"], action: "Select the previous row, or go to the previous search result" },
  { keys: ["Home", "End"], action: "Select the first or the last row" },
  { keys: ["PageDown", "PageUp"], action: "Move the selection by one screen" },
  { keys: ["Enter"], action: "Open the selected request or exception" },
  { keys: ["p", " "], action: "Pause or follow new rows" },
  { keys: ["m"], action: "Mark the selected request. The next request you open is compared with it" },
  { keys: ["c"], action: "Compare a request with the previous request to the same route" },
  { keys: ["r"], action: "Replay a request: send it to your server again" },
  { keys: ["t"], action: "Make a test from a request" },
  { keys: ["s"], action: "Save the filter of a list, the search or an exception" },
  { keys: ["Escape"], action: "Close a dialog, leave the search box, or go back to the list" },
];

const [bindings, setBindings] = createSignal<readonly Binding[]>(VIEW_KEYS);

/** Every binding, frame and views, in the order the `?` dialog lists them. */
export { bindings };

/** Adds `list` to the map and returns the function that removes it again. Throws when one of its keys
 *  is in the map already. */
export function bind(list: Binding[]): () => void {
  const taken = new Set(bindings().flatMap((one) => one.keys));
  for (const one of list) for (const key of one.keys) if (taken.has(key)) throw new Error(`viewer: the key ${key} is bound twice`);
  setBindings((now) => [...list, ...now]);
  return () => setBindings((now) => now.filter((one) => !list.includes(one)));
}

/** The name of `e` in the map: `Mod+k` when Ctrl (Cmd on macOS) is held, otherwise `e.key`. */
function nameOf(e: KeyboardEvent): string {
  return (MAC ? e.metaKey : e.ctrlKey) ? `Mod+${e.key.toLowerCase()}` : e.key;
}

/** Runs the binding `e` names, and returns true when one ran. */
export function dispatch(e: KeyboardEvent): boolean {
  const name = nameOf(e);
  const one = bindings().find((b) => b.run && b.keys.includes(name));
  if (!one?.run) return false;
  // A `Mod+` key runs in a field too, as a browser's own shortcuts do, but never under a dialog.
  const mod = name.startsWith("Mod+");
  if (mod ? e.defaultPrevented || e.altKey || !!document.querySelector("dialog[open]") : ignored(e)) return false;
  e.preventDefault();
  one.run(e);
  return true;
}

/** How a key is shown: `Ctrl K` or `⌘ K` for `Mod+k`, `↓` for `ArrowDown`, `Space` for a space. */
export function label(key: string): string {
  if (key.startsWith("Mod+")) return `${MAC ? "⌘" : "Ctrl"} ${key.slice(4).toUpperCase()}`;
  return SHOWN[key] ?? key;
}

const SHOWN: Record<string, string> = {
  ArrowDown: "↓",
  ArrowUp: "↑",
  ArrowLeft: "←",
  ArrowRight: "→",
  PageDown: "Page Down",
  PageUp: "Page Up",
  Escape: "Esc",
  " ": "Space",
};

/** True when a view's handler should leave `e` alone: a browser shortcut, a key typed into a field,
 *  Enter or Space on a button or a link, or any key while a dialog is open. */
export function ignored(e: KeyboardEvent): boolean {
  if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey) return true;
  if (document.querySelector("dialog[open]")) return true;
  const target = e.target instanceof Element ? e.target : null;
  if (target?.closest("input, select, textarea, [contenteditable]")) return true;
  return (e.key === "Enter" || e.key === " ") && !!target?.closest("button, a[href], summary");
}
