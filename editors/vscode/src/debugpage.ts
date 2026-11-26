// The page the *Open Debug Viewer* panel shows: one frame of the viewer that `nvs debug` serves, and
// one script that passes three messages between that frame and the extension
// (`rule:ide/the-debug-viewer-opens-from-one-button`,
// `rule:testing/the-viewer-is-framed-only-by-the-editor-under-a-token-path`).
//
// The panel draws nothing of its own (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
// Its policy admits the frame's one origin and the one script, and nothing may connect. The frame is
// the embed link the language server answered (`nvs/debugViewer`), which carries the token in its
// path, so no cookie is needed in a frame that is cross-site to the panel.
//
// **The messages.** The frame sends `open` with a file and a line, and the script passes it to the
// extension, which checks it again ([`opened`], [`within`]). The extension sends `theme` and `go`,
// and the script passes each to the frame, at the frame's origin alone. The script posts `theme`
// once the frame loads, because the viewer learns the panel's origin from its first message and
// sends `open` to that origin only.
//
// Nothing here imports `vscode`, so the headless suite runs it in plain Node. `src/debugviewer.ts` is
// the commands and the panel.

import { isAbsolute, relative, resolve } from "node:path";

/** The longest path a `go` message may name, as the viewer reads it. */
export const PATH_MAX = 2048;

/** A theme the viewer takes. */
export type Theme = "system" | "dark" | "light";

/** What the page needs: the embed link, the path to open first, the theme and the script's nonce. */
export interface Page {
  embed: string;
  path: string;
  theme: Theme;
  nonce: string;
}

/**
 * The origin of `embed`, or `undefined` when it is not an `http:` or `https:` link with a path
 * under `/embed/`.
 */
export function frameOrigin(embed: string): string | undefined {
  let url: URL;
  try {
    url = new URL(embed);
  } catch {
    return undefined;
  }
  if ((url.protocol !== "http:" && url.protocol !== "https:") || !url.pathname.startsWith("/embed/")) {
    return undefined;
  }
  return url.origin;
}

/**
 * The content security policy: the frame's one origin, the one script with `nonce`, and its one
 * style block with the same nonce. Nothing may connect, and nothing else may load.
 */
export function policy(origin: string, nonce: string): string {
  return [
    "default-src 'none'",
    `frame-src ${origin}`,
    `script-src 'nonce-${nonce}'`,
    `style-src 'nonce-${nonce}'`,
    "connect-src 'none'",
    "img-src 'none'",
    "form-action 'none'",
    "base-uri 'none'",
    "object-src 'none'",
  ].join("; ");
}

/** Whether `path` is one the viewer may be sent to: an absolute path of this origin. */
export function goable(path: unknown): path is string {
  return typeof path === "string"
    && path.length <= PATH_MAX
    && path.startsWith("/")
    && !path.startsWith("//")
    && !path.includes("\\")
    && ![...path].some((char) => char.trim() === "" || char.charCodeAt(0) < 0x20);
}

/** The frame's address for `embed` opened at `path`. The embed link ends in `/`. */
export function frameAddress(embed: string, path: string): string {
  const base = embed.endsWith("/") ? embed.slice(0, -1) : embed;
  return goable(path) && path !== "/" ? `${base}${path}` : `${base}/`;
}

/** The page. It throws when `embed` is not an embed link. */
export function page(input: Page): string {
  const origin = frameOrigin(input.embed);
  if (origin === undefined) {
    throw new Error("the debug viewer's embed link is not an http or https link under /embed/");
  }
  const nonce = attribute(input.nonce);
  return [
    "<!doctype html>",
    '<html lang="en">',
    "<head>",
    '<meta charset="utf-8">',
    `<meta http-equiv="Content-Security-Policy" content="${attribute(policy(origin, input.nonce))}">`,
    `<style nonce="${nonce}">html, body, iframe { margin: 0; padding: 0; border: 0; width: 100%; height: 100%; overflow: hidden; display: block; }</style>`,
    "</head>",
    "<body>",
    `<iframe id="viewer" title="Novis Debug" src="${attribute(frameAddress(input.embed, input.path))}"></iframe>`,
    `<script nonce="${nonce}">${script(origin, input.theme)}</script>`,
    "</body>",
    "</html>",
  ].join("\n");
}

/**
 * The script. It passes `open` from the frame to the extension, and `theme` and `go` from the
 * extension to the frame, and nothing else.
 */
function script(origin: string, theme: Theme): string {
  const at = scriptString(origin);
  return [
    "(() => {",
    "  const vscode = acquireVsCodeApi();",
    '  const frame = document.getElementById("viewer");',
    `  let theme = ${scriptString(theme)};`,
    `  const send = (message) => frame.contentWindow && frame.contentWindow.postMessage(message, ${at});`,
    '  frame.addEventListener("load", () => send({ type: "theme", value: theme }));',
    '  addEventListener("message", (event) => {',
    "    const data = event.data;",
    '    if (typeof data !== "object" || data === null) return;',
    "    if (event.source === frame.contentWindow) {",
    `      if (event.origin === ${at} && data.type === "open") {`,
    '        vscode.postMessage({ type: "open", file: data.file, line: data.line, column: data.column });',
    "      }",
    "      return;",
    "    }",
    '    if (data.type === "theme") { theme = data.value; send({ type: "theme", value: data.value }); }',
    '    else if (data.type === "go") send({ type: "go", path: data.path });',
    "  });",
    "})();",
  ].join("\n");
}

/** A file and line the frame asked the editor to open. */
export interface Open {
  file: string;
  line: number;
  column?: number;
}

/** `message` as an `open`, or `undefined` when it is not one. Lines and columns count from 1. */
export function opened(message: unknown): Open | undefined {
  if (typeof message !== "object" || message === null) {
    return undefined;
  }
  const { type, file, line, column } = message as Record<string, unknown>;
  if (type !== "open" || typeof file !== "string" || file.length === 0 || file.length > 4096) {
    return undefined;
  }
  if (!isAbsolute(file) || !counted(line)) {
    return undefined;
  }
  if (column !== undefined && !counted(column)) {
    return undefined;
  }
  return column === undefined ? { file, line } : { file, line, column };
}

/** Whether `file` is inside one of `folders`, after `..` is resolved. */
export function within(file: string, folders: readonly string[]): boolean {
  const target = resolve(file);
  return folders.some((folder) => {
    const rest = relative(resolve(folder), target);
    return rest !== "" && !rest.startsWith("..") && !isAbsolute(rest);
  });
}

/** The viewer's theme for VS Code's `ColorThemeKind`: 1 and 4 are light, 2 and 3 are dark. */
export function themeOf(kind: number): Theme {
  return kind === 1 || kind === 4 ? "light" : "dark";
}

function counted(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= 10_000_000;
}

/** `value` as a JavaScript string literal that cannot close the `<script>` element it sits in. */
function scriptString(value: string): string {
  return JSON.stringify(value).replace(/</g, "\\u003c");
}

function attribute(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
}
