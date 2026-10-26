// The viewer's keyboard: the list of every key, which the `?` dialog shows, and the test each view's
// key handler runs first, so a key typed into a field or meant for a button keeps its own meaning.

/** Every key the viewer answers, in the order the `?` dialog lists them. */
export const KEYS: ReadonlyArray<readonly [keys: string[], action: string]> = [
  [["j", "↓"], "Select the next request"],
  [["k", "↑"], "Select the previous request"],
  [["Home", "End"], "Select the first or the last request"],
  [["Page Down", "Page Up"], "Move the selection by one screen"],
  [["Enter"], "Open the selected request"],
  [["p", "Space"], "Pause or follow new requests"],
  [["/"], "Go to the search box"],
  [["t"], "Change the theme"],
  [["?"], "Show this list"],
  [["Esc"], "Close this list, leave the search box, or go back to the list"],
];

/** True when a view's handler should leave `e` alone: a browser shortcut, a key typed into a field,
 *  Enter or Space on a button or a link, or any key while a dialog is open. */
export function ignored(e: KeyboardEvent): boolean {
  if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey) return true;
  if (document.querySelector("dialog[open]")) return true;
  const target = e.target instanceof Element ? e.target : null;
  if (target?.closest("input, select, textarea, [contenteditable]")) return true;
  return (e.key === "Enter" || e.key === " ") && !!target?.closest("button, a[href], summary");
}
