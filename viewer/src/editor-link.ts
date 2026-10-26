// Builds the link that opens a source site in the developer's editor, from a preset or a template.
// Path mappings rewrite the start of a recorded path first.
// A link comes only from a record's structured file and line, never from text in a message.
// A template whose scheme runs script in the page gives no link.

import type { Editor, PathMapping, Settings } from "./api.ts";

/** A place in a source file, as a record carries it. */
export interface Site {
  file: string;
  line: number;
  /** 1 when the record has no column. */
  column?: number;
}

/** The part of the settings a link reads. */
export type EditorSettings = Pick<Settings, "editor" | "editor_template" | "path_mappings">;

/** The link template of every preset. */
export const PRESETS: Record<Exclude<Editor, "custom">, string> = {
  vscode: "vscode://file/{file}:{line}:{column}",
  "vscode-insiders": "vscode-insiders://file/{file}:{line}:{column}",
  vscodium: "vscodium://file/{file}:{line}:{column}",
  cursor: "cursor://file/{file}:{line}:{column}",
  zed: "zed://file/{file}:{line}:{column}",
  idea: "idea://open?file={file}&line={line}",
  phpstorm: "phpstorm://open?file={file}&line={line}",
};

/** What a link is built with before the settings load, or when they cannot. */
const DEFAULT: EditorSettings = { editor: "vscode", path_mappings: [] };

/** Schemes that run in the page instead of opening an editor. */
const REFUSED = new Set(["javascript", "data", "vbscript", "blob"]);

/** `path` with the longest `from` that starts it replaced by its `to`. */
export function mapPath(path: string, mappings: readonly PathMapping[]): string {
  let best: PathMapping | null = null;
  for (const mapping of mappings) {
    if (mapping.from && path.startsWith(mapping.from) && (!best || mapping.from.length > best.from.length)) best = mapping;
  }
  return best ? best.to + path.slice(best.from.length) : path;
}

/** The URL scheme `url` starts with, lower-cased, or `null` when it starts with none. */
function scheme(url: string): string | null {
  return /^([a-z][a-z0-9+.-]*):/i.exec(url)?.[1]?.toLowerCase() ?? null;
}

/**
 * The link that opens `site` in the editor `settings` name (`rule:testing/a-site-in-the-viewer-opens-in-the-editor`),
 * or `null` when the link has no scheme or one in `REFUSED`. The service checks a template when it saves
 * it, and this check is the second one. The path is mapped first, then its backslashes become `/`, and
 * it is percent-encoded with its `/` and `:` kept.
 *
 * It takes a `Site` and never a string. No function here reads `file:line` out of text, so a logged
 * message cannot make a link.
 */
export function editorLink(site: Site, settings: EditorSettings | null): string | null {
  const chosen = settings ?? DEFAULT;
  const template = chosen.editor === "custom" ? (chosen.editor_template ?? "") : PRESETS[chosen.editor];
  if (!template) return null;
  let path = mapPath(site.file, chosen.path_mappings).replaceAll("\\", "/");
  // `vscode://file/` and its kin already end in the `/` an absolute path starts with.
  if (chosen.editor !== "custom" && template.includes("://file/") && path.startsWith("/")) path = path.slice(1);
  const file = encodeURIComponent(path).replaceAll("%2F", "/").replaceAll("%3A", ":");
  const values: Record<string, string> = { file, line: String(site.line), column: String(site.column ?? 1) };
  const url = template.replace(/\{(file|line|column)\}/g, (_, key: string) => values[key]!);
  const at = scheme(url);
  return at === null || REFUSED.has(at) ? null : url;
}
