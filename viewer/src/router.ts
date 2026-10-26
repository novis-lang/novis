// The viewer's routing: the view is the page's path and its filter is the query, so every view has a
// link that survives a reload. `nvs debug` serves the page for every path outside `/api/`, so the
// browser can open any of them directly. A move inside the viewer is `history.pushState` and the
// location signal below.

import { createSignal } from "solid-js";
import type { RecordOwner, RequestRow, SavedView } from "./api.ts";

const here = () => location.pathname + location.search;
const [at, setAt] = createSignal(here());
/** Rises each time the view on screen is asked to load again. */
const [version, setVersion] = createSignal(0);

/** The path of the view on screen, with its query. */
export function path(): string {
  return at().split("?")[0]!;
}

/** The query of the view on screen, without the `?`. */
export function query(): string {
  const i = at().indexOf("?");
  return i < 0 ? "" : at().slice(i + 1);
}

export { version };

/** Shows the view at `to`, a path with or without a query. `replace` swaps the history entry
 *  instead of adding one. */
export function navigate(to: string, replace = false): void {
  if (to === here()) return;
  if (replace) history.replaceState(null, "", to);
  else history.pushState(null, "", to);
  setAt(here());
}

/** Loads the view on screen again, as a reload of the tab would. */
export function reloadView(): void {
  setVersion(version() + 1);
}

addEventListener("popstate", () => setAt(here()));

/** The link to one request: its trace id and its span id, or the trace id alone when it has no span. */
export function requestLink(row: Pick<RequestRow | RecordOwner, "trace" | "span">): string {
  return row.span ? `/requests/${row.trace}/${row.span}` : `/requests/${row.trace}`;
}

/** The link to `view` with `query`, the part after `?`. */
export function viewLink(view: SavedView, query: string): string {
  const base = view === "requests" ? "/" : `/${view}`;
  return query ? `${base}?${query}` : base;
}

/** A view the path names. */
export type Route =
  | { view: "list" }
  | { view: "request"; trace: string; span?: string }
  | { view: "stream" }
  | { view: "log" }
  | { view: "search" }
  | { view: "saved"; name: string };

/** Reads a path into the view it names. A path the viewer does not have is the request list. */
export function route(to: string): Route {
  const at = to.split("?")[0]!;
  const m = /^\/requests\/([0-9a-f]{32})(?:\/([0-9a-f]{16}))?\/?$/.exec(at);
  if (m) return m[2] ? { view: "request", trace: m[1]!, span: m[2] } : { view: "request", trace: m[1]! };
  const view = /^\/(stream|log|search)\/?$/.exec(at)?.[1];
  if (view === "stream" || view === "log" || view === "search") return { view };
  const saved = /^\/saved\/([^/]+)\/?$/.exec(at);
  if (saved) {
    try {
      return { view: "saved", name: decodeURIComponent(saved[1]!) };
    } catch {
      return { view: "list" };
    }
  }
  return { view: "list" };
}
