// The request list, the viewer's main view: newest first, following each watermark `/api/events`
// pushes, and paused by a key or by scrolling away from the newest row. The list, its keys and the
// follow bar are the shared `List` and `createFeed` (`ui/list.tsx`); this file has the columns.
//
// A row shows the method, the path with its route beside it, the status, the duration, the queries,
// the dumps and the age. A request that threw shows its exception's class and site after the path.
//
// `m` or the Mark button marks the selected request, and opening a second request then opens the two
// side by side (`compare.tsx`).
//
// What it spends: what `createFeed` keeps, a few hundred bytes a row, and one timer that redraws the
// ages every ten seconds.

import { createSignal, onCleanup, Show } from "solid-js";
import { requests, type RequestRow } from "../api.ts";
import { navigate } from "../router.ts";
import { Method, Status } from "../ui/badges.tsx";
import { Empty } from "../ui/empty.tsx";
import { createFeed, List, type RowKey } from "../ui/list.tsx";
import { Site } from "../ui/site.tsx";
import { isMarked, MarkState, openLink, toggleMark } from "./compare.tsx";

// TEMP(stage 3): stream.tsx and exceptions.tsx still import it from here until they move onto `List`.
export { ROW_HEIGHT } from "../ui/list.tsx";

/** A request row's height in pixels. */
export const REQUEST_ROW = 38;

/** `duration_us` the way a person reads it: microseconds, milliseconds or seconds. */
export function duration(us: number): string {
  if (us < 1000) return `${us} µs`;
  if (us < 1_000_000) return `${(us / 1000).toFixed(us < 10_000 ? 2 : 1)} ms`;
  return `${(us / 1_000_000).toFixed(2)} s`;
}

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

/** The column heads above a list of request rows. */
export function RequestColumns() {
  return (
    <>
      <span>Method</span>
      <span>Path</span>
      <span>Status</span>
      <span class="num">Duration</span>
      <span class="num">Queries</span>
      <span class="num">Dumps</span>
      <span class="num">Age</span>
    </>
  );
}

/** The cells of one request row. The site of its exception is a link to the editor, which the
 *  row's own click leaves alone. */
export function RequestCells(props: { row: RequestRow; now?: number }) {
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
      <span class="num duration">{duration(row().duration_us)}</span>
      <span class="num queries">{row().queries}</span>
      <span class="num dumps">{row().dumps}</span>
      <span class="num age">{age(row().start_us, props.now ?? Date.now())}</span>
    </>
  );
}

export function RequestList() {
  const feed = createFeed<RequestRow, number>({
    load: async (at, signal) => {
      const page = await requests(at, signal);
      return { rows: page.requests, older: page.older, watermark: page.watermark };
    },
    key: (row) => row.id,
    cursor: (row) => row.id,
  });
  const [selected, setSelected] = createSignal<RowKey | null>(null);
  const now = createNow();
  const selectedRow = () => feed.rows().find((row) => row.id === selected());
  const mark = (row: RequestRow | undefined) => {
    if (row) toggleMark(row);
  };

  return (
    <section class="requests" aria-label="Requests">
      <List
        rows={feed.rows()}
        key={(row) => row.id}
        idPrefix="request"
        label="Requests, newest first"
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
        empty={
          <Empty icon="requests" title="No requests yet">
            They show here when your server handles them.
          </Empty>
        }
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
