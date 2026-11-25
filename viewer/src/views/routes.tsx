// The routes view: `/routes` shows one row per method and route, with its count, its share of errors,
// its p50 and p95 durations, its mean number of queries and when it was last seen.
// A column title sorts by that column, and a second click reverses it. The sort is in the link as
// `?sort=<column>&dir=asc|desc`, the most requests first when it is not set. `Enter` or a click opens
// the request list filtered to that method and route. Each new watermark loads the routes again.
// What it spends: the rows of one answer, at most 200, while the view is open, and one timer that
// redraws the ages every ten seconds.

import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { routes, watermarks, type RouteFigures, type Routes } from "../api.ts";
import { navigate, viewLink } from "../router.ts";
import { Method } from "../ui/badges.tsx";
import { Empty } from "../ui/empty.tsx";
import { List, type RowKey } from "../ui/list.tsx";
import { age, createNow, duration, listQuery } from "./requests.tsx";

/** A column the list sorts by. */
type SortColumn = "method" | "route" | "count" | "errors" | "p50" | "p95" | "queries" | "last";

/** The order of the rows: a column and a direction. */
interface Sort {
  col: SortColumn;
  dir: "asc" | "desc";
}

/** Each column: its key in the link, its title, whether it is a number, and the value it sorts by. */
const COLUMNS: { col: SortColumn; title: string; num: boolean; value: (r: RouteFigures) => string | number }[] = [
  { col: "method", title: "Method", num: false, value: (r) => r.method },
  { col: "route", title: "Route", num: false, value: (r) => r.route ?? "" },
  { col: "count", title: "Count", num: true, value: (r) => r.count },
  { col: "errors", title: "Errors", num: true, value: errorShare },
  { col: "p50", title: "p50", num: true, value: (r) => r.p50_us },
  { col: "p95", title: "p95", num: true, value: (r) => r.p95_us },
  { col: "queries", title: "Queries", num: true, value: (r) => r.mean_queries },
  { col: "last", title: "Last seen", num: true, value: (r) => r.last_us },
];

const DEFAULT_SORT: Sort = { col: "count", dir: "desc" };

/** The share of a route's requests that ended in an error, from 0 to 1. */
function errorShare(r: RouteFigures): number {
  return r.count === 0 ? 0 : r.errors / r.count;
}

/** The error share as a percent, such as `12%`, `0.4%` or `<0.1%`. */
function percent(share: number): string {
  const p = share * 100;
  if (p > 0 && p < 0.1) return "<0.1%";
  return `${p === 0 || p >= 10 ? p.toFixed(0) : p.toFixed(1)}%`;
}

/** The text of an error, for the note the list shows when it could not load. */
function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** Reads the sort from the view's query. A column or direction the view does not know gives the default. */
export function routeSort(query: string): Sort {
  const params = new URLSearchParams(query);
  const col = COLUMNS.find((c) => c.col === params.get("sort"));
  if (!col) return DEFAULT_SORT;
  const dir = params.get("dir");
  return { col: col.col, dir: dir === "asc" || dir === "desc" ? dir : col.num ? "desc" : "asc" };
}

/** The sort after a click on `col`: the same column reverses, and a new one starts with its usual direction. */
function nextSort(sort: Sort, col: (typeof COLUMNS)[number]): Sort {
  if (sort.col === col.col) return { col: col.col, dir: sort.dir === "asc" ? "desc" : "asc" };
  return { col: col.col, dir: col.num ? "desc" : "asc" };
}

/** The rows in the order of `sort`. Rows with the same value keep the most requests first. */
function sorted(rows: RouteFigures[], sort: Sort): RouteFigures[] {
  const value = COLUMNS.find((c) => c.col === sort.col)!.value;
  const sign = sort.dir === "asc" ? 1 : -1;
  return [...rows].sort((a, b) => {
    const x = value(a);
    const y = value(b);
    const by = typeof x === "number" && typeof y === "number" ? x - y : String(x).localeCompare(String(y));
    return by !== 0 ? sign * by : b.count - a.count;
  });
}

/** The key of one row: its method and its route. */
function rowKey(r: RouteFigures): RowKey {
  return `${r.method} ${r.route ?? "\u0000"}`;
}

/** The request list's link for one row. A row without a route filters on the method alone. */
function rowLink(r: RouteFigures): string {
  return viewLink("requests", listQuery(r.route === null ? { method: r.method } : { method: r.method, route: r.route }));
}

/** Loads the routes now and again on each new watermark. A watermark that arrives during a load
 *  starts one more load after it, not one per watermark. */
function liveRoutes() {
  const [answer, setAnswer] = createSignal<Routes | null>(null);
  const [error, setError] = createSignal<unknown>(null);
  const abort = new AbortController();
  let loading = false;
  let again = false;

  const load = () => {
    if (loading) {
      again = true;
      return;
    }
    loading = true;
    routes(abort.signal)
      .then((next) => {
        setAnswer(next);
        setError(null);
      })
      .catch((err) => {
        if (!abort.signal.aborted) setError(err);
      })
      .finally(() => {
        loading = false;
        if (again && !abort.signal.aborted) {
          again = false;
          load();
        }
      });
  };

  onMount(() => {
    load();
    watermarks(load, abort.signal);
  });
  onCleanup(() => abort.abort());
  return { answer, error };
}

/** The view at `/routes`, with `query` the part of its link after `?`. */
export function RoutesView(props: { query: string }) {
  const live = liveRoutes();
  const sort = createMemo(() => routeSort(props.query));
  const rows = createMemo(() => sorted(live.answer()?.routes ?? [], sort()));
  const now = createNow();
  const [selected, setSelected] = createSignal<RowKey | null>(null);
  // A selected row that is gone from the answer is let go, and `List` then selects the first row.
  createEffect(() => {
    const at = selected();
    if (at !== null && !rows().some((r) => rowKey(r) === at)) setSelected(null);
  });

  /** Sorts by `col`, and writes the sort into the link without a new history entry. */
  const sortBy = (col: (typeof COLUMNS)[number]) => {
    const next = nextSort(sort(), col);
    navigate(`/routes?sort=${next.col}&dir=${next.dir}`, true);
  };

  return (
    <section class="routes" aria-label="Routes">
      <h2 class="title">Routes</h2>
      <Show when={live.answer()?.partial}>
        <p class="note routes-note">Counted over the newest 10,000 requests.</p>
      </Show>
      <Show when={live.answer()?.cut}>
        <p class="note routes-note">Only the 200 routes with the most requests are shown.</p>
      </Show>
      <Show when={rows().length > 0}>
        <div class="cols routes-grid routes-head" role="row">
          <For each={COLUMNS}>
            {(col) => (
              <span
                role="columnheader"
                class={col.num ? "num" : undefined}
                aria-sort={sort().col === col.col ? (sort().dir === "asc" ? "ascending" : "descending") : "none"}
              >
                <button type="button" class="routes-sort" onClick={() => sortBy(col)}>
                  {col.title}
                </button>
              </span>
            )}
          </For>
        </div>
      </Show>
      <List
        rows={rows()}
        key={rowKey}
        idPrefix="route"
        label="Routes, with their requests, errors and durations"
        selected={selected()}
        onSelect={setSelected}
        onOpen={(r) => navigate(rowLink(r))}
        openOnClick
        grid="routes-grid"
        loaded={live.answer() !== null}
        error={live.error() ? message(live.error()) : null}
        empty={
          <Empty icon="routes" title="No routes recorded yet">
            A route shows here once your server answers a request.
          </Empty>
        }
      >
        {(r) => <RouteCells row={r} now={now()} />}
      </List>
    </section>
  );
}

/** The cells of one route's row. */
function RouteCells(props: { row: RouteFigures; now: number }) {
  const r = () => props.row;
  const basis = () => (r().durations === 1 ? "Over the newest request" : `Over the newest ${r().durations} requests`);
  return (
    <>
      <span class="method-cell">
        <Method value={r().method} />
      </span>
      <Show when={r().route} fallback={<span class="routes-route routes-none">No route</span>}>
        {(route) => (
          <span class="routes-route" title={route()}>
            {route()}
          </span>
        )}
      </Show>
      <span class="num">{r().count}</span>
      <span class="num" classList={{ bad: r().errors > 0 }}>
        {percent(errorShare(r()))}
      </span>
      <span class="num" title={basis()}>
        {duration(r().p50_us)}
      </span>
      <span class="num">{duration(r().p95_us)}</span>
      <span class="num">{r().mean_queries.toFixed(1)}</span>
      <span class="num routes-age">{age(r().last_us, props.now)}</span>
    </>
  );
}
