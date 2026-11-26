// The three messages between the viewer and the editor panel that frames it, and nothing else. They
// exist only when the page is under the embed base path (`base.ts`).
//
// - The page sends `{type: "open", file, line, column}` when a source site is clicked. The editor
//   opens the file. The path mappings in the settings are applied first.
// - The panel sends `{type: "theme", value}` with `system`, `dark` or `light`.
// - The panel sends `{type: "go", path}` with a path the router reads, and the viewer shows it.
//
// A message is taken only from `window.parent` and only from a `vscode-webview:` origin. `open` goes
// only to that origin: the one the panel's first message came from, or the frame's ancestor.

import { embedded } from "./base.ts";
import type { Site } from "./editor-link.ts";
import { mapPath } from "./editor-link.ts";
import { navigate, route } from "./router.ts";
import { setTheme } from "./theme.ts";
import type { PathMapping } from "./api.ts";

/** The scheme of every origin the page takes a message from and sends one to. */
const EDITOR = "vscode-webview:";

/** The longest path a `go` message may name. */
const PATH_MAX = 2048;

/** The panel's origin, learned from its first message or the frame's ancestor. */
let panel: string | null = null;

function editorOrigin(origin: unknown): origin is string {
  return typeof origin === "string" && origin.startsWith(EDITOR);
}

/** The origin `open` is sent to, or `null` when the page has no editor to send it to. */
function target(): string | null {
  if (panel) return panel;
  const ancestor = location.ancestorOrigins?.[0];
  return editorOrigin(ancestor) ? ancestor : null;
}

/** Whether `path` is one the router reads: a view's path, with or without a query or a hash. */
export function readable(path: unknown): path is string {
  if (typeof path !== "string" || path.length > PATH_MAX) return false;
  if (!path.startsWith("/") || path.startsWith("//") || path.includes("\\")) return false;
  const at = path.split(/[?#]/)[0]!;
  return at === "/" || route(at).view !== "list";
}

/** Reads one message. Gives `true` when it was the panel's and was taken. */
export function take(e: MessageEvent): boolean {
  if (!embedded || e.source !== window.parent || !editorOrigin(e.origin)) return false;
  const data: unknown = e.data;
  if (typeof data !== "object" || data === null || !("type" in data)) return false;
  if (data.type === "theme" && "value" in data && (data.value === "system" || data.value === "dark" || data.value === "light")) {
    panel ??= e.origin;
    setTheme(data.value);
    return true;
  }
  if (data.type === "go" && "path" in data && readable(data.path)) {
    panel ??= e.origin;
    navigate(data.path);
    return true;
  }
  return false;
}

/** Starts taking the panel's messages. Gives the function that stops it. Does nothing when the page
 *  is not framed by an editor. */
export function listenToEditor(): () => void {
  if (!embedded) return () => {};
  addEventListener("message", take);
  return () => removeEventListener("message", take);
}

/** Sends `open` for `site` to the editor, after `mappings`. Gives `false` when there is no editor. */
export function openInEditor(site: Site, mappings: readonly PathMapping[]): boolean {
  const to = embedded ? target() : null;
  if (!to) return false;
  const message: { type: "open"; file: string; line: number; column?: number } = {
    type: "open",
    file: mapPath(site.file, mappings),
    line: site.line,
  };
  if (site.column !== undefined) message.column = site.column;
  window.parent.postMessage(message, to);
  return true;
}
