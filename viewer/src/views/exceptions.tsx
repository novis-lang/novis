// The exceptions view: `/exceptions` lists the groups of uncaught throws, one class at one site.
// `/exceptions?class=&file=&line=` shows one group and its requests, newest first.
// Both lists work like the request list, and each new watermark loads their first page again.
// In one group's list, `m` marks a request to compare, as in the request list.
// What it spends: the rows loaded so far, a few hundred bytes each, while the view is open.

import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";
import {
  ApiError,
  exceptionKey,
  exceptionQuery,
  exceptionRequests,
  exceptions,
  watermarks,
  type ExceptionGroup,
  type ExceptionKey,
  type ExceptionPage,
  type ExceptionRequests,
  type RequestRow,
} from "../api.ts";
import { ignored } from "../keys.ts";
import { navigate, requestLink } from "../router.ts";
import { Site } from "../site.tsx";
import { isMarked, MarkState, openLink, toggleMark } from "./compare.tsx";
import { RequestCells, RequestColumns, ROW_HEIGHT } from "./requests.tsx";
import { SaveButton } from "./saved.tsx";
import { when } from "./stream.tsx";

/** A group row's height in pixels: two lines. `.row.exc-grid` in `style.css` has the same. */
const GROUP_HEIGHT = 52;
/** Rows kept in the DOM above and below the screen. */
const OVERSCAN = 6;
/** How close to the last loaded row the screen comes before the next older page loads. */
const NEAR_END = 40;

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** The view at `/exceptions`, with `query` the part of its link after `?`. */
export function ExceptionsView(props: { query: string }) {
  const key = exceptionKey(props.query);
  return key ? <GroupView key={key} /> : <GroupList />;
}

/**
 * A listbox in the WAI-ARIA pattern, with the keys of the request list: they move the selection, and
 * Enter opens the selected row. Every row has the same height, so only the rows on screen are in the
 * DOM, and older pages load as the end comes near. A click on a link inside a row, such as a site's
 * editor link, does not open the row.
 */
function Rows<T>(props: {
  rows: T[];
  id: (row: T) => string;
  height: number;
  label: string;
  grid: string;
  /** True while older pages remain to load. */
  more: boolean;
  loadMore: () => void;
  open: (row: T) => void;
  cells: (row: T) => JSX.Element;
  /** When set, `m` marks the selected row to compare, as in the request list. */
  mark?: { toggle: (row: T) => void; is: (row: T) => boolean };
  /** Called with the selected row each time the selection changes. */
  onSelect?: (row: T | undefined) => void;
}) {
  const [selected, setSelected] = createSignal<string | null>(null);
  const [scrollTop, setScrollTop] = createSignal(0);
  const [height, setHeight] = createSignal(0);
  let list!: HTMLDivElement;

  const index = createMemo(() => new Map(props.rows.map((row, i) => [props.id(row), i])));
  const current = () => {
    const id = selected();
    return id === null ? -1 : (index().get(id) ?? -1);
  };
  const first = createMemo(() => Math.max(0, Math.floor(scrollTop() / props.height) - OVERSCAN));
  const end = createMemo(() => Math.min(props.rows.length, Math.ceil((scrollTop() + height()) / props.height) + OVERSCAN));
  const visible = createMemo(() => props.rows.slice(first(), end()));
  const active = () => {
    const i = current();
    return i >= first() && i < end() ? (selected() ?? undefined) : undefined;
  };

  // The first row is selected until another is, and again when the selected row is gone.
  createEffect(() => {
    if (current() < 0 && props.rows.length > 0) setSelected(props.id(props.rows[0]!));
  });
  createEffect(() => {
    if (props.more && end() >= props.rows.length - NEAR_END) props.loadMore();
  });
  createEffect(() => props.onSelect?.(props.rows[current()]));

  const move = (to: number) => {
    const all = props.rows;
    if (all.length === 0) return;
    const i = Math.min(all.length - 1, Math.max(0, to));
    setSelected(props.id(all[i]!));
    const top = i * props.height;
    if (top < list.scrollTop) list.scrollTop = top;
    else if (top + props.height > list.scrollTop + list.clientHeight) list.scrollTop = top + props.height - list.clientHeight;
    list.focus({ preventScroll: true });
  };
  const screen = () => Math.max(1, Math.floor(list.clientHeight / props.height) - 1);

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e)) return;
    switch (e.key) {
      case "j":
      case "ArrowDown":
        move(current() + 1);
        break;
      case "k":
      case "ArrowUp":
        move(current() - 1);
        break;
      case "Home":
        move(0);
        break;
      case "End":
        move(props.rows.length - 1);
        break;
      case "PageDown":
        move(current() + screen());
        break;
      case "PageUp":
        move(current() - screen());
        break;
      case "Enter": {
        const row = props.rows[current()];
        if (!row) return;
        props.open(row);
        break;
      }
      case "m": {
        const row = props.rows[current()];
        if (!props.mark || !row) return;
        props.mark.toggle(row);
        break;
      }
      default:
        return;
    }
    e.preventDefault();
  };

  onMount(() => {
    setHeight(list.clientHeight);
    const resize = new ResizeObserver(() => setHeight(list.clientHeight));
    resize.observe(list);
    document.addEventListener("keydown", onKey);
    onCleanup(() => {
      resize.disconnect();
      document.removeEventListener("keydown", onKey);
    });
  });

  return (
    <div
      ref={list}
      class="list"
      role="listbox"
      tabindex="0"
      aria-label={props.label}
      aria-activedescendant={active()}
      data-rows={props.rows.length}
      onScroll={() => setScrollTop(list.scrollTop)}
    >
      <div class="list-space" role="none" style={{ height: `${props.rows.length * props.height}px` }}>
        <For each={visible()}>
          {(row, i) => {
            const at = () => first() + i();
            return (
              <div
                id={props.id(row)}
                role="option"
                class={`row ${props.grid}${props.mark?.is(row) ? " marked" : ""}`}
                aria-description={props.mark?.is(row) ? "Marked to compare" : undefined}
                aria-selected={selected() === props.id(row) ? "true" : "false"}
                aria-posinset={at() + 1}
                aria-setsize={props.more ? -1 : props.rows.length}
                style={{ transform: `translateY(${at() * props.height}px)` }}
                onClick={(e) => {
                  if (e.target instanceof Element && e.target.closest("a")) return;
                  setSelected(props.id(row));
                  props.open(row);
                }}
              >
                {props.cells(row)}
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}

/** A small bar chart of `counts`, one bar per equal span of time, oldest on the left. */
function Chart(props: { counts: number[] }) {
  const max = () => Math.max(1, ...props.counts);
  const bar = (n: number) => (n === 0 ? 0 : Math.max(2, Math.round((n / max()) * 20)));
  return (
    <svg class="exc-chart" viewBox="0 0 96 20" width="96" height="20" role="img" aria-label="Throws over time">
      <line x1="0" y1="19.5" x2="96" y2="19.5" class="exc-chart-base" />
      <For each={props.counts}>
        {(n, i) => <rect x={i() * 4} y={20 - bar(n)} width="3" height={bar(n)} />}
      </For>
    </svg>
  );
}

/** The cells of one group's row, on two lines. */
function GroupCells(props: { group: ExceptionGroup }) {
  const g = () => props.group;
  return (
    <>
      <span class="exc-class exc">{g().class}</span>
      <span class="site exc-at">
        <Site site={g()} tabindex={-1} />
      </span>
      <span class="num exc-count">{g().count}</span>
      <span class="exc-chart-cell">
        <Chart counts={g().chart} />
      </span>
      <span class="at exc-last">Last {when(g().last_us)}</span>
      <span class="exc-message">{g().message ?? "No message"}</span>
      <span class="note exc-routes">{g().routes.join(", ")}</span>
      <span class="at exc-first">First {when(g().first_us)}</span>
    </>
  );
}

/**
 * Loads one live, paged list: its first page now and on each new watermark, and older pages with
 * `before` on request. A group changes in place as throws arrive, so the first page is loaded whole
 * again, and goes over the rows already loaded. A first page that does not reach those rows replaces them.
 */
function livePages<P, T, C extends string | number>(load: {
  first: (signal: AbortSignal) => Promise<P>;
  older: (before: C, signal: AbortSignal) => Promise<P>;
  watermark: (page: P) => number;
  rows: (page: P) => T[];
  next: (page: P) => C | null;
  id: (row: T) => number;
}) {
  const [rows, setRows] = createSignal<T[]>([]);
  const [older, setOlder] = createSignal<C | null>(null);
  const [page, setPage] = createSignal<P | null>(null);
  const [error, setError] = createSignal<unknown>(null);
  const [loadingOlder, setLoadingOlder] = createSignal(false);
  let seen = -1;
  /** Older pages loaded since the first, so a new first page keeps their rows and their cursor. */
  let extended = false;
  let chain = Promise.resolve();
  const abort = new AbortController();

  const showFirst = (next: P) => {
    seen = load.watermark(next);
    const fresh = load.rows(next);
    const ids = new Set(fresh.map(load.id));
    const gap = load.next(next) !== null && !rows().some((row) => ids.has(load.id(row)));
    if (!extended || gap) {
      setRows(fresh);
      setOlder(() => load.next(next));
      extended = false;
    } else setRows([...fresh, ...rows().filter((row) => !ids.has(load.id(row)))]);
    setPage(() => next);
    setError(null);
  };

  const refresh = () => {
    chain = chain
      .then(async () => showFirst(await load.first(abort.signal)))
      .catch((err) => {
        if (!abort.signal.aborted) setError(err);
      });
  };

  const loadMore = () => {
    const before = older();
    // After a failure the view waits for the next watermark, so a broken service is not asked again
    // and again.
    if (before === null || loadingOlder() || error()) return;
    setLoadingOlder(true);
    load
      .older(before, abort.signal)
      .then((more) => {
        if (older() !== before) return;
        extended = true;
        const have = new Set(rows().map(load.id));
        setRows([...rows(), ...load.rows(more).filter((row) => !have.has(load.id(row)))]);
        setOlder(() => load.next(more));
      })
      .catch((err) => {
        if (!abort.signal.aborted) setError(err);
      })
      .finally(() => setLoadingOlder(false));
  };

  onMount(() => {
    refresh();
    watermarks((batch) => {
      if (batch > seen) refresh();
    }, abort.signal);
  });
  onCleanup(() => abort.abort());

  return { rows, older, page, error, loadMore };
}

/** `/exceptions`: every group, the group thrown last first. */
function GroupList() {
  const live = livePages<ExceptionPage, ExceptionGroup, string>({
    first: (signal) => exceptions(undefined, signal),
    older: (before, signal) => exceptions(before, signal),
    watermark: (page) => page.watermark,
    rows: (page) => page.groups,
    next: (page) => page.older,
    id: (group) => group.id,
  });
  const loaded = () => live.page() !== null;
  return (
    <section class="exceptions" aria-label="Exceptions">
      <Show when={live.error()}>
        <p class="note bad" role="alert">
          The exceptions could not load: {message(live.error())}
        </p>
      </Show>
      <Show when={loaded() && live.rows().length === 0}>
        <p class="note">No exceptions yet. They show here when a request throws an exception and does not catch it.</p>
      </Show>
      <div class="cols exc-grid" aria-hidden="true">
        <span>Exception</span>
        <span>Thrown at</span>
        <span class="num">Count</span>
        <span>Throws over time</span>
        <span>Seen</span>
      </div>
      <Rows
        rows={live.rows()}
        id={(group) => `exception-${group.id}`}
        height={GROUP_HEIGHT}
        label="Exceptions, the most recent first"
        grid="exc-grid"
        more={live.older() !== null}
        loadMore={live.loadMore}
        open={(group) => navigate(`/exceptions?${exceptionQuery(group)}`)}
        cells={(group) => <GroupCells group={group} />}
      />
    </section>
  );
}

/** `/exceptions?class=&file=&line=`: one group and the requests that threw it, newest first. A request
 *  opens its page at the exception (`#exception`), and Escape goes back to every group. */
function GroupView(props: { key: ExceptionKey }) {
  const key = props.key;
  const live = livePages<ExceptionRequests, RequestRow, number>({
    first: (signal) => exceptionRequests(key, undefined, signal),
    older: (before, signal) => exceptionRequests(key, before, signal),
    watermark: (page) => page.watermark,
    rows: (page) => page.requests,
    next: (page) => page.older,
    id: (row: RequestRow) => row.id,
  });
  const group = () => live.page()?.group ?? null;
  const [selectedRow, setSelectedRow] = createSignal<RequestRow | undefined>();
  const mark = () => {
    const row = selectedRow();
    if (row) toggleMark(row);
  };
  /** A request opens at its exception, or beside the marked request when one is marked. */
  const open = (row: RequestRow) => {
    const to = openLink(row);
    navigate(to === requestLink(row) ? `${to}#exception` : to);
  };
  const missing = () => {
    const err = live.error();
    return err instanceof ApiError && err.status === 404;
  };

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e) || e.key !== "Escape") return;
    e.preventDefault();
    navigate("/exceptions");
  };
  onMount(() => document.addEventListener("keydown", onKey));
  onCleanup(() => document.removeEventListener("keydown", onKey));

  return (
    <section class="exceptions exc-one" aria-label="Exception">
      <div class="exc-summary">
        <div class="toolbar-line">
          <a href="/exceptions" class="back">
            Back to all exceptions
          </a>
          <SaveButton view="exceptions" query={() => exceptionQuery(key)} />
        </div>
        <Show when={live.error() && !missing()}>
          <p class="note bad" role="alert">
            This exception could not load: {message(live.error())}
          </p>
        </Show>
        <Show when={missing()}>
          <p class="note">No request threw this exception at this line.</p>
        </Show>
        <Show when={!live.error() && !group()}>
          <p class="note">Loading.</p>
        </Show>
        <Show when={group()}>
          {(g) => (
            <>
              <h2 class="title exc-class bad">{g().class}</h2>
              <p class="exc-message">{g().message ?? "No message"}</p>
              <dl class="fields">
                <dt>Thrown at</dt>
                <dd class="site">
                  <Site site={g()} />
                </dd>
                <dt>Count</dt>
                <dd class="exc-count">{g().count}</dd>
                <dt>First</dt>
                <dd>{when(g().first_us)}</dd>
                <dt>Last</dt>
                <dd>{when(g().last_us)}</dd>
                <dt>Routes</dt>
                <dd class="exc-routes">{g().routes.join(", ")}</dd>
                <dt>Throws over time</dt>
                <dd>
                  <Chart counts={g().chart} />
                </dd>
              </dl>
              <Show when={g().sampled}>
                <p class="note">The chart and the routes count only the 1000 most recent throws.</p>
              </Show>
            </>
          )}
        </Show>
        <div class="toolbar-line">
          <button
            type="button"
            class="btn mark"
            onClick={mark}
            disabled={!selectedRow()?.span}
            aria-pressed={selectedRow() && isMarked(selectedRow()!) ? "true" : "false"}
            aria-keyshortcuts="m"
          >
            Mark to compare
          </button>
          <MarkState />
        </div>
      </div>
      <RequestColumns />
      <Rows
        rows={live.rows()}
        id={(row) => `request-${row.id}`}
        height={ROW_HEIGHT}
        label="Requests that threw this exception, newest first"
        grid=""
        more={live.older() !== null}
        loadMore={live.loadMore}
        open={open}
        cells={(row) => <RequestCells row={row} />}
        mark={{ toggle: toggleMark, is: isMarked }}
        onSelect={setSelectedRow}
      />
    </section>
  );
}
