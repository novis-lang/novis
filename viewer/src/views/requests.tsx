// The request list, the viewer's main view: newest first, following each watermark `/api/events`
// pushes, and paused by a key or by scrolling away from the newest row. The list, its keys and the
// follow bar are the shared `List` and `createFeed` (`ui/list.tsx`); this file has the columns.
//
// Above the list, three tiles show the last five minutes from `/api/summary`: the requests, the
// duration and the errors. They load again when the watermark moves, at most once a second, and not
// while the list is paused. The follow bar has the filter chips: method, status, route, duration
// and whether the request threw. The filter is the view's query, such as `/?status=5xx`, so a
// filtered list has a link and can be saved.
//
// A row shows the method, the path with its route beside it, the status, the duration with one bar
// split by the kind of work, the queries with a gold count when one statement repeated, the dumps
// and the age. The bars are scaled to the slowest row on screen. A request that threw shows its
// exception's class and site after the path.
//
// At 1280 pixels and wider, a pane beside the list shows the selected request: its figures and a
// small timeline of its events. `Enter` opens its page.
//
// `m` or the Mark button marks the selected request, and opening a second request then opens the two
// side by side (`compare.tsx`).
//
// What it spends: what `createFeed` keeps, a few hundred bytes a row, the last summary, the selected
// request's records while the pane shows, and one timer that redraws the ages every ten seconds.

import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import {
  request,
  requests,
  routes,
  summary,
  type RequestDetail,
  type RequestFilter,
  type RequestRow,
  type Summary,
  type TimeByKind,
} from "../api.ts";
import { navigate, viewLink } from "../router.ts";
import { Chip, Method, Repeat, Status } from "../ui/badges.tsx";
import { Empty } from "../ui/empty.tsx";
import { createFeed, type Feed, List, type RowKey } from "../ui/list.tsx";
import { ChipMenu, type Choice } from "../ui/menu.tsx";
import { Site } from "../ui/site.tsx";
import { Meter, type Part, Spark, Tile } from "../ui/tile.tsx";
import { isMarked, MarkState, openLink, toggleMark } from "./compare.tsx";
import { SaveButton } from "./saved.tsx";
import { size } from "../format.ts";

/** A request row's height in pixels. */
export const REQUEST_ROW = 38;

/** `duration_us` the way a person reads it: microseconds, milliseconds or seconds. */
export function duration(us: number): string {
  if (us < 1000) return `${us} µs`;
  if (us < 1_000_000) return `${(us / 1000).toFixed(us < 10_000 ? 2 : 1)} ms`;
  return `${(us / 1_000_000).toFixed(2)} s`;
}

/** A number of bytes the way a person reads it, from `../format.ts`. */
export { size };

/** The class that colours a status: `s4` for a client error, `s5` for a server error. */
export function statusClass(status: number): string {
  return status >= 500 ? "s5" : status >= 400 ? "s4" : "";
}

/** How long ago `us` (microseconds since the epoch) was, at `now` (milliseconds): `4 s`, `2 min`,
 *  `3 h` or `5 d`. */
export function age(us: number, now: number): string {
  const s = Math.max(0, Math.floor((now - us / 1000) / 1000));
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86_400) return `${Math.floor(s / 3600)} h`;
  return `${Math.floor(s / 86_400)} d`;
}

/** The time now in milliseconds, updated every ten seconds while the owner lives. */
export function createNow() {
  const [now, setNow] = createSignal(Date.now());
  const timer = setInterval(() => setNow(Date.now()), 10_000);
  onCleanup(() => clearInterval(timer));
  return now;
}

/** What the request list filters on: the part of `RequestFilter` its link has. */
export interface ListFilter {
  method?: string;
  status?: string;
  route?: string;
  min_us?: number;
  min_memory?: number;
  threw?: boolean;
}

/** Reads the request list's query into its filter. A key with a value the list does not offer is
 *  left out, so a hand-edited link still shows a list. */
export function listFilter(query: string): ListFilter {
  const params = new URLSearchParams(query);
  const filter: ListFilter = {};
  const method = params.get("method");
  if (method && /^[A-Z]{1,16}$/.test(method)) filter.method = method;
  const status = params.get("status");
  if (status && /^([1-5]xx|[1-5]\d\d)$/.test(status)) filter.status = status;
  const route = params.get("route");
  if (route) filter.route = route;
  const min = params.get("min_us");
  if (min && /^\d{1,12}$/.test(min)) filter.min_us = Number(min);
  const memory = params.get("min_memory");
  if (memory && /^\d{1,15}$/.test(memory)) filter.min_memory = Number(memory);
  const threw = params.get("threw");
  if (threw === "true" || threw === "false") filter.threw = threw === "true";
  return filter;
}

/** The query of `filter`, without the `?`, its keys always in one order. */
export function listQuery(filter: ListFilter): string {
  const params = new URLSearchParams();
  if (filter.method) params.set("method", filter.method);
  if (filter.status) params.set("status", filter.status);
  if (filter.route) params.set("route", filter.route);
  if (filter.min_us !== undefined) params.set("min_us", String(filter.min_us));
  if (filter.min_memory !== undefined) params.set("min_memory", String(filter.min_memory));
  if (filter.threw !== undefined) params.set("threw", String(filter.threw));
  return params.toString();
}

/** The keys `GET /api/requests` takes for `filter`. */
function apiFilter(filter: ListFilter): RequestFilter {
  const out: RequestFilter = {};
  if (filter.method) out.method = filter.method;
  if (filter.status) out.status = /^\d+$/.test(filter.status) ? Number(filter.status) : (filter.status as `${1 | 2 | 3 | 4 | 5}xx`);
  if (filter.route) out.route = filter.route;
  if (filter.min_us !== undefined) out.min_us = filter.min_us;
  if (filter.min_memory !== undefined) out.min_memory = filter.min_memory;
  if (filter.threw !== undefined) out.threw = filter.threw;
  return out;
}

/** The link of the request list as it was last shown, so a request page goes back to it. */
let lastList = "/";

/** The request list's link with the filter it last had. */
export function listLink(): string {
  return lastList;
}

/** The kinds of work a request's time is split into, in the order a bar draws them. */
const KINDS: readonly { kind: keyof TimeByKind; label: string }[] = [
  { kind: "query", label: "Database" },
  { kind: "http", label: "HTTP calls" },
  { kind: "cache", label: "Cache" },
  { kind: "session", label: "Session" },
  { kind: "file", label: "Files" },
  { kind: "gc", label: "Garbage collection" },
  { kind: "spawn", label: "Spawns" },
];

/** A request's time split by kind, with the rest as `own`, its own code. Events of one kind can
 *  overlap, so when the kinds add up to more than the request took, each is scaled down to fit. */
export function split(row: RequestRow): Part[] {
  const sum = KINDS.reduce((total, one) => total + Math.max(0, row.time_us[one.kind]), 0);
  const fit = sum > row.duration_us && sum > 0 ? row.duration_us / sum : 1;
  const parts: Part[] = [];
  for (const one of KINDS) {
    const us = Math.max(0, row.time_us[one.kind]);
    if (us > 0) parts.push({ kind: one.kind, value: us * fit, label: `${one.label} ${duration(us)}` });
  }
  const own = Math.max(0, row.duration_us - sum);
  if (own > 0) parts.push({ kind: "own", value: own, label: `Your code ${duration(own)}` });
  return parts;
}

/** The column heads above a list of request rows. */
export function RequestColumns() {
  return (
    <>
      <span>Method</span>
      <span>Path</span>
      <span>Status</span>
      <span>Duration</span>
      <span class="num">Memory</span>
      <span class="num">Queries</span>
      <span class="num">Dumps</span>
      <span class="num">Age</span>
    </>
  );
}

/** The cells of one request row. The site of its exception is a link to the editor, which the
 *  row's own click leaves alone. `scale` is the duration a full bar stands for, the row's own when
 *  it is not set. */
export function RequestCells(props: { row: RequestRow; now?: number; scale?: number }) {
  const row = () => props.row;
  const route = () => {
    const r = row().route;
    return r && r !== row().path ? r : null;
  };
  return (
    <>
      <span class="method-cell">
        <Method value={row().method} />
      </span>
      <span class="path" title={row().route ?? row().path}>
        <span class="path-text">{row().path}</span>
        <Show when={route()}>{(r) => <small class="route">{r()}</small>}</Show>
        <Show when={row().exception}>
          {(threw) => (
            <>
              <span class="exc">{threw().class}</span>
              <span class="site exc-at">
                <Site site={threw()} tabindex={-1} />
              </span>
            </>
          )}
        </Show>
      </span>
      <span class="status-cell">
        <Status code={row().status} />
      </span>
      <span class="duration">
        <span class="num">{duration(row().duration_us)}</span>
        <Meter parts={split(row())} total={Math.max(1, props.scale ?? row().duration_us)} label="Time by kind" />
      </span>
      <span class="num memory" title={memoryTitle(row())}>
        {row().memory_peak === null ? "–" : size(row().memory_peak!)}
      </span>
      <span class="num queries">
        {row().queries}
        <Show when={row().repeats >= 2}>
          <Repeat count={row().repeats} />
        </Show>
      </span>
      <span class="num dumps">{row().dumps}</span>
      <span class="num age">{age(row().start_us, props.now ?? Date.now())}</span>
    </>
  );
}

/** The memory peak against its limit, as a tooltip. */
export function memoryTitle(row: RequestRow): string {
  if (row.memory_peak === null) return "No memory peak was recorded.";
  if (row.memory_limit === null) return `Peak ${size(row.memory_peak)}, no limit`;
  return `Peak ${size(row.memory_peak)} of a ${size(row.memory_limit)} limit`;
}

/** A tile with the memory peak, and a meter against the limit when there is one. */
export function MemoryTile(props: { row: RequestRow }) {
  const peak = () => props.row.memory_peak ?? 0;
  const limit = () => props.row.memory_limit;
  const kind = () => {
    const l = limit();
    if (l === null || l === 0) return "memory";
    const share = peak() / l;
    return share >= 0.9 ? "memory-full" : share >= 0.75 ? "memory-high" : "memory";
  };
  return (
    <Tile label="Memory peak" value={size(peak())} unit={limit() === null ? "no limit" : `of ${size(limit()!)}`}>
      <Show when={limit() !== null}>
        <Meter parts={[{ kind: kind(), value: Math.min(peak(), limit()!), label: memoryTitle(props.row) }]} total={Math.max(1, limit()!)} label="Memory peak against the limit" />
      </Show>
    </Tile>
  );
}

/** The last summary, loaded again when `feed`'s watermark moves, at most once a second, and never
 *  while it is paused, so nothing on screen moves then. */
function createSummary(feed: Feed<RequestRow>) {
  const [data, setData] = createSignal<Summary | null>(null);
  const abort = new AbortController();
  let asked = -1;
  let last = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const run = () => {
    timer = undefined;
    if (feed.paused()) {
      asked = -1;
      return;
    }
    last = Date.now();
    summary(undefined, abort.signal)
      .then(setData)
      .catch(() => {}); // The tiles stay as they were; the list shows its own errors.
  };
  createEffect(() => {
    const batch = feed.batch();
    if (feed.paused() || batch === asked || timer) return;
    asked = batch;
    timer = setTimeout(run, Math.max(0, last + 1000 - Date.now()));
  });
  onCleanup(() => {
    abort.abort();
    clearTimeout(timer);
  });
  return data;
}

const count = new Intl.NumberFormat("en-US");

/** The tiles of the last five minutes. */
function SummaryTiles(props: { summary: Summary }) {
  const s = () => props.summary;
  const rate = () => (s().count === 0 ? "0" : ((s().errors / s().count) * 100).toFixed(1));
  return (
    <div class="tiles" aria-label="The last five minutes" role="group">
      <Tile label="Requests, last 5 min" value={count.format(s().count)} unit={s().cut ? "or more" : undefined}>
        <Spark values={s().buckets.map((b) => b.count)} label="Requests per 25 seconds" />
      </Tile>
      <Tile label="Duration" value={s().p50_us === null ? "–" : duration(s().p50_us!)} unit="p50">
        <Show when={s().p95_us !== null}>
          <p class="tile-note">
            {duration(s().p95_us!)} p95 · {duration(s().max_us!)} max
          </p>
        </Show>
      </Tile>
      <Tile label="Errors" value={rate()} unit="%">
        <Spark values={s().buckets.map((b) => b.errors)} bad={s().buckets.map((b) => b.errors > 0)} label="Errors per 25 seconds" />
      </Tile>
    </div>
  );
}

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const STATUSES: Choice[] = ["2xx", "3xx", "4xx", "5xx"].map((value) => ({ label: value, value }));
const HEAVIER: Choice[] = [1, 8, 32, 128].map((mib) => ({ label: size(mib * 1024 * 1024), value: String(mib * 1024 * 1024) }));
const SLOWER: Choice[] = [10, 50, 100, 500, 1000].map((ms) => ({ label: duration(ms * 1000), value: String(ms * 1000) }));
const THREW: Choice[] = [
  { label: "Threw", value: "true" },
  { label: "Did not throw", value: "false" },
];

/** The filter chips. Each opens a menu of choices, and a choice opens the list's link with it. */
function Filters(props: { filter: ListFilter }) {
  const go = (next: ListFilter) => navigate(viewLink("requests", listQuery(next)));
  const set = <K extends keyof ListFilter>(key: K, value: ListFilter[K] | undefined) => {
    const next = { ...props.filter };
    if (value === undefined) delete next[key];
    else next[key] = value;
    go(next);
  };
  const [routeChoices, setRouteChoices] = createSignal<Choice[]>([]);
  const [routeNote, setRouteNote] = createSignal<string | null>(null);
  const loadRoutes = () => {
    if (routeChoices().length > 0) return;
    setRouteNote("Loading the routes.");
    routes()
      .then((answer) => {
        const seen = new Set<string>();
        for (const one of answer.routes) if (one.route !== null) seen.add(one.route);
        setRouteChoices([...seen].sort().map((route) => ({ label: route, value: route })));
        setRouteNote(seen.size === 0 ? "No route was recorded yet." : null);
      })
      .catch((err) => setRouteNote(`The routes could not load: ${err instanceof Error ? err.message : String(err)}`));
  };
  const any = () => listQuery(props.filter) !== "";
  return (
    <div class="filter-chips" role="group" aria-label="Filter">
      <ChipMenu
        label="Method"
        value={props.filter.method ?? null}
        choices={METHODS.map((value) => ({ label: value, value }))}
        onPick={(value) => set("method", value ?? undefined)}
      />
      <ChipMenu label="Status" value={props.filter.status ?? null} choices={STATUSES} onPick={(value) => set("status", value ?? undefined)} />
      <ChipMenu
        label="Route"
        value={props.filter.route ?? null}
        choices={routeChoices()}
        note={routeNote()}
        onOpen={loadRoutes}
        onPick={(value) => set("route", value ?? undefined)}
      />
      <ChipMenu
        label="Slower than"
        value={props.filter.min_us === undefined ? null : String(props.filter.min_us)}
        shown={props.filter.min_us === undefined ? undefined : duration(props.filter.min_us)}
        choices={SLOWER}
        onPick={(value) => set("min_us", value === null ? undefined : Number(value))}
      />
      <ChipMenu
        label="Memory over"
        value={props.filter.min_memory === undefined ? null : String(props.filter.min_memory)}
        shown={props.filter.min_memory === undefined ? undefined : size(props.filter.min_memory)}
        choices={HEAVIER}
        onPick={(value) => set("min_memory", value === null ? undefined : Number(value))}
      />
      <ChipMenu
        label="Exception"
        value={props.filter.threw === undefined ? null : String(props.filter.threw)}
        shown={props.filter.threw === undefined ? undefined : props.filter.threw ? "Threw" : "Did not throw"}
        choices={THREW}
        onPick={(value) => set("threw", value === null ? undefined : value === "true")}
      />
      <Show when={any()}>
        <Chip add onClick={() => go({})}>
          Clear
        </Chip>
      </Show>
    </div>
  );
}

/** True while the window is at least 1280 pixels wide. */
function createWide() {
  const query = matchMedia("(min-width: 1280px)");
  const [wide, setWide] = createSignal(query.matches);
  const change = () => setWide(query.matches);
  query.addEventListener("change", change);
  onCleanup(() => query.removeEventListener("change", change));
  return wide;
}

/** The events the pane's timeline draws at most. */
const PEEK_EVENTS = 24;

/** The pane beside the list: the selected request's figures and a small timeline. Its records load
 *  a moment after the selection settles, so moving through the list with `j` loads only the last. */
function Peek(props: { row: RequestRow }) {
  const [detail, setDetail] = createSignal<RequestDetail | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  createEffect(() => {
    const row = props.row;
    setDetail(null);
    setError(null);
    if (!row.span) return;
    const abort = new AbortController();
    const timer = setTimeout(() => {
      request({ trace: row.trace, span: row.span! }, abort.signal)
        .then(setDetail)
        .catch((err) => {
          if (!abort.signal.aborted) setError(err instanceof Error ? err.message : String(err));
        });
    }, 120);
    onCleanup(() => {
      clearTimeout(timer);
      abort.abort();
    });
  });
  const row = () => props.row;
  const events = () => {
    const d = detail();
    if (!d) return [];
    return d.events
      .filter((event) => event.kind !== "call")
      .slice(0, PEEK_EVENTS)
      .map((event) => ({ event, start: event.start_us - d.request.start_us, length: Math.max(0, event.end_us - event.start_us) }));
  };
  const span = () => Math.max(1, row().duration_us);
  const pct = (us: number) => `${Math.min(100, Math.max(0, (us / span()) * 100))}%`;
  return (
    <aside class="peek" aria-label="The selected request">
      <div class="peek-title">
        <Method value={row().method} />
        <span class="peek-path">{row().path}</span>
        <Status code={row().status} />
      </div>
      <div class="peek-tiles">
        <Tile label="Duration" value={duration(row().duration_us)} />
        <Tile label="Queries" value={String(row().queries)} unit={row().repeats >= 2 ? `· ${row().repeats}× at one site` : undefined} />
        <Tile label="Dumps" value={String(row().dumps)} />
        <Tile label="Logs" value={String(row().logs)} />
        <Show when={row().memory_peak !== null}>
          <MemoryTile row={row()} />
        </Show>
      </div>
      <Meter parts={split(row())} total={span()} label="Time by kind" />
      <ul class="peek-legend">
        <For each={split(row())}>
          {(part) => (
            <li>
              <i class={`k-${part.kind}`} aria-hidden="true" />
              {part.label}
            </li>
          )}
        </For>
      </ul>
      <h3 class="peek-head">Timeline</h3>
      <Show when={error()}>
        <p class="note bad">The records could not load: {error()}</p>
      </Show>
      <Show when={detail()} fallback={<Show when={!error()}><p class="note">{row().span ? "Loading." : "This request has no span id."}</p></Show>}>
        <Show when={events().length > 0} fallback={<p class="note">No event other than a function call was recorded.</p>}>
          <ol class="peek-events">
            <For each={events()}>
              {(one) => (
                <li>
                  <span class="peek-label" title={one.event.query?.sql ?? one.event.label}>
                    <i class={`k-${one.event.kind}`} aria-hidden="true" />
                    {one.event.query?.sql ?? one.event.label}
                  </span>
                  <span class="peek-lane">
                    <i class={`k-${one.event.kind}`} style={{ left: pct(one.start), width: pct(one.length) }} />
                  </span>
                  <span class="num">{duration(one.length)}</span>
                </li>
              )}
            </For>
          </ol>
        </Show>
      </Show>
      <p class="note peek-open">
        Press <kbd>Enter</kbd> to open this request.
      </p>
    </aside>
  );
}

export function RequestList(props: { query: string }) {
  const filter = listFilter(props.query);
  const query = listQuery(filter);
  lastList = viewLink("requests", query);
  const filtered = query !== "";
  const feed = createFeed<RequestRow, number>({
    load: async (at, signal) => {
      const page = await requests({ ...apiFilter(filter), ...at }, signal);
      return { rows: page.requests, older: page.older, watermark: page.watermark };
    },
    key: (row) => row.id,
    cursor: (row) => row.id,
  });
  const tiles = createSummary(feed);
  const [selected, setSelected] = createSignal<RowKey | null>(null);
  const [shown, setShown] = createSignal<RequestRow[]>([]);
  const scale = createMemo(() => Math.max(1, ...shown().map((row) => row.duration_us)));
  const now = createNow();
  const wide = createWide();
  const selectedRow = () => feed.rows().find((row) => row.id === selected());
  const mark = (row: RequestRow | undefined) => {
    if (row) toggleMark(row);
  };

  const empty = () => {
    if (!filtered) {
      return (
        <Empty icon="requests" title="No requests yet">
          <p>Your server records each request it handles when <code>nvs.toml</code> has these lines:</p>
          <pre class="empty-code">{"[debug]\nstore = true"}</pre>
        </Empty>
      );
    }
    if (feed.more()) return <p class="note">Reading older requests.</p>;
    return (
      <Empty icon="requests" title="No request matches this filter">
        <button type="button" class="btn small" onClick={() => navigate("/")}>
          Clear the filter
        </button>
      </Empty>
    );
  };

  return (
    <section class="requests" aria-label="Requests">
      <Show when={tiles()}>{(s) => <SummaryTiles summary={s()} />}</Show>
      <div class="requests-body">
        <List
          rows={feed.rows()}
          key={(row) => row.id}
          idPrefix="request"
          label={filtered ? "Requests that match the filter, newest first" : "Requests, newest first"}
          selected={selected()}
          onSelect={setSelected}
          onOpen={(row) => navigate(openLink(row))}
          openOnClick
          grid="request-grid"
          rowHeight={REQUEST_ROW}
          rowClass={(row) => ({ threw: row.exception !== null, marked: isMarked(row) })}
          describe={(row) => (isMarked(row) ? "Marked to compare" : undefined)}
          cols={<RequestColumns />}
          feed={feed}
          noun={["request", "requests"]}
          keys={{ m: mark }}
          onShown={setShown}
          empty={empty()}
          actions={
            <>
              <Filters filter={filter} />
              <span class="spacer" />
              <Show when={filtered}>
                <SaveButton view="requests" query={() => query} />
              </Show>
              <button
                type="button"
                class="btn small mark"
                onClick={() => mark(selectedRow())}
                disabled={!selectedRow()?.span}
                aria-pressed={selectedRow() && isMarked(selectedRow()!) ? "true" : "false"}
                aria-keyshortcuts="m"
              >
                Mark to compare
              </button>
              <MarkState />
            </>
          }
        >
          {(row) => <RequestCells row={row} now={now()} scale={scale()} />}
        </List>
        <Show when={wide() && selectedRow()}>{(row) => <Peek row={row()} />}</Show>
      </div>
    </section>
  );
}
