// The viewer's routing: the view is the page's path, so every view has a link that survives a reload.
// `nvs debug` serves the page for every path outside `/api/`, so the browser can open any of them
// directly. A move inside the viewer is `history.pushState` and the path signal below.

import { createSignal } from "solid-js";
import type { RequestRow } from "./api.ts";

const [path, setPath] = createSignal(location.pathname);

/** The path of the view on screen. */
export { path };

/** Shows the view at `to`. `replace` swaps the history entry instead of adding one. */
export function navigate(to: string, replace = false): void {
  if (to === location.pathname) return;
  if (replace) history.replaceState(null, "", to);
  else history.pushState(null, "", to);
  setPath(to);
}

addEventListener("popstate", () => setPath(location.pathname));

/** The link to one request: its trace id and its span id, or the trace id alone when it has no span. */
export function requestLink(row: Pick<RequestRow, "trace" | "span">): string {
  return row.span ? `/requests/${row.trace}/${row.span}` : `/requests/${row.trace}`;
}

/** A view the path names. */
export type Route = { view: "list" } | { view: "request"; trace: string; span?: string };

/** Reads a path into the view it names. Anything that is not a request's link is the list. */
export function route(at: string): Route {
  const m = /^\/requests\/([0-9a-f]{32})(?:\/([0-9a-f]{16}))?\/?$/.exec(at);
  if (!m) return { view: "list" };
  return m[2] ? { view: "request", trace: m[1]!, span: m[2] } : { view: "request", trace: m[1]! };
}
