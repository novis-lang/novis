// The exceptions view: `/exceptions` lists the groups of uncaught throws, one class at one site.
// `/exceptions?class=&file=&line=` shows one group, the stack of its latest throw (`ui/stack.tsx`),
// the newest runs that threw it, and its requests, newest first.
// Both lists are the shared `List` (`ui/list.tsx`). They do not follow or pause: each new watermark
// loads their first page again, through `livePages` below.
// In one group's list, `m` or the Mark button marks a request to compare, as in the request list.
// What it spends: the rows loaded so far, a few hundred bytes each, while the view is open, and one
// timer that redraws the ages of one group's requests every ten seconds.

import { createEffect, createSignal, For, onCleanup, onMount, Show } from "solid-js";
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
import { navigate, requestLink, runLink } from "../router.ts";
import { Empty } from "../ui/empty.tsx";
import { List, type RowKey } from "../ui/list.tsx";
import { Site } from "../ui/site.tsx";
import { Stack } from "../ui/stack.tsx";
import { Spark } from "../ui/tile.tsx";
import { isMarked, MarkState, openLink, toggleMark } from "./compare.tsx";
import { age, createNow, duration, REQUEST_ROW, RequestCells, RequestColumns } from "./requests.tsx";
import { OF_LABEL } from "./runs.tsx";
import { SaveButton } from "./saved.tsx";
import { when } from "./stream.tsx";

/** A group row's height in pixels: two lines of text. */
const GROUP_HEIGHT = 52;

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** The view at `/exceptions`, with `query` the part of its link after `?`. */
export function ExceptionsView(props: { query: string }) {
  const key = exceptionKey(props.query);
  return key ? <GroupView key={key} /> : <GroupList />;
}

/** The throws over time: one bar per equal span of time, oldest on the left. A span with a throw is
 *  drawn in the error colour. */
function Throws(props: { counts: number[] }) {
  return (
    <span class="exc-chart">
      <Spark values={props.counts} bad={props.counts.map((n) => n > 0)} label="Throws over time" />
    </span>
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
      <Throws counts={g().chart} />
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

/** The selected row's key. When the selected row is gone from `rows`, nothing is selected, and `List`
 *  then selects the first row. */
function createSelection<T>(rows: () => T[], key: (row: T) => RowKey) {
  const [selected, setSelected] = createSignal<RowKey | null>(null);
  createEffect(() => {
    const at = selected();
    if (at !== null && !rows().some((row) => key(row) === at)) setSelected(null);
  });
  return [selected, setSelected] as const;
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
  const [selected, setSelected] = createSelection(live.rows, (group) => group.id);
  return (
    <section class="exceptions" aria-label="Exceptions">
      <List
        rows={live.rows()}
        key={(group) => group.id}
        idPrefix="exception"
        label="Exceptions, the most recent first"
        selected={selected()}
        onSelect={setSelected}
        onOpen={(group) => navigate(`/exceptions?${exceptionQuery(group)}`)}
        openOnClick
        grid="exc-grid"
        rowHeight={GROUP_HEIGHT}
        cols={
          <>
            <span>Exception</span>
            <span>Thrown at</span>
            <span class="num">Count</span>
            <span>Throws over time</span>
            <span>Seen</span>
          </>
        }
        more={live.older() !== null}
        onEnd={live.loadMore}
        loaded={live.page() !== null}
        error={live.error() ? message(live.error()) : null}
        empty={
          <Empty icon="errors" title="No exceptions yet">
            They show here when a request throws an exception and does not catch it.
          </Empty>
        }
      >
        {(group) => <GroupCells group={group} />}
      </List>
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
  const [selected, setSelected] = createSelection(live.rows, (row) => row.id);
  const selectedRow = () => live.rows().find((row) => row.id === selected());
  const now = createNow();
  const mark = (row: RequestRow | undefined) => {
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
        <div class="exc-bar">
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
                  <Throws counts={g().chart} />
                </dd>
              </dl>
              <Show when={g().sampled}>
                <p class="note">The chart and the routes count only the 1000 most recent throws.</p>
              </Show>
              <Show when={live.page()?.latest}>{(latest) => <Stack stack={latest()} />}</Show>
              <Show when={(live.page()?.runs.length ?? 0) > 0}>
                <h3 class="exc-runs-title">Runs that threw it</h3>
                <ul class="exc-runs">
                  <For each={live.page()!.runs}>
                    {(run) => (
                      <li>
                        <a href={`${runLink(run)}#exception`}>
                          {OF_LABEL[run.of]} <code>{run.name}</code>
                        </a>{" "}
                        <span class="note">
                          {duration(run.duration_us)} · {age(run.start_us, now())} ago
                        </span>
                      </li>
                    )}
                  </For>
                </ul>
              </Show>
            </>
          )}
        </Show>
      </div>
      <List
        rows={live.rows()}
        key={(row) => row.id}
        idPrefix="request"
        label="Requests that threw this exception, newest first"
        selected={selected()}
        onSelect={setSelected}
        onOpen={open}
        openOnClick
        grid="request-grid"
        rowHeight={REQUEST_ROW}
        rowClass={(row) => ({ threw: row.exception !== null, marked: isMarked(row) })}
        describe={(row) => (isMarked(row) ? "Marked to compare" : undefined)}
        cols={<RequestColumns />}
        more={live.older() !== null}
        onEnd={live.loadMore}
        loaded={live.page() !== null}
        keys={{ m: mark }}
        actions={
          <>
            <span class="spacer" />
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
        {(row) => <RequestCells row={row} now={now()} />}
      </List>
    </section>
  );
}
