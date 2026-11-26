// The path the page is served under. An editor frames the viewer at `/embed/<token>/`, and `nvs debug`
// serves the same page, its files and its API under that prefix
// (`crates/nvs-cli/src/debug/listen.rs`). Everywhere else the prefix is empty.
//
// The views build their paths without it. The router puts it in front of what goes into the address
// bar and takes it off what comes out, and the API puts it in front of every fetch and the event stream.

/** `/embed/<token>` when the page was opened under it, and `""` otherwise. */
export const BASE: string =
  typeof location === "undefined" ? "" : (/^\/embed\/[A-Za-z0-9._~-]{16,}(?=\/|$)/.exec(location.pathname)?.[0] ?? "");

/** Whether an editor frames the page. */
export const embedded: boolean = BASE !== "";

/** `path`, which starts with `/`, under the page's base path. */
export function under(path: string): string {
  return BASE + path;
}

/** `path` without the page's base path, or `null` when it is not under it. A page with no base path
 *  gives `path` back as it is. */
export function within(path: string): string | null {
  if (!BASE) return path;
  if (path === BASE) return "/";
  return path.startsWith(`${BASE}/`) ? path.slice(BASE.length) : null;
}
