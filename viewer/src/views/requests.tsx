// The request list, the viewer's main view: newest first, following each watermark `/api/events`
// pushes, and paused by a key or by scrolling away from the newest row.
//
// The list is virtual. Every row has the same height, so the rows on screen follow from the scroll
// position alone, and only they and a few rows on each side are in the DOM. The browser's cost is the
// screen, not the index. Older pages load with `before` as the end comes near.
//
// A paused list never changes what is on screen: what arrives is kept aside and counted, and resuming
// puts it on top. A `since` page that is not complete (`older` set) means more arrived than one page,
// so the newest page is loaded again instead.
//
// What it spends: the rows loaded so far, at most `KEEP` while following, plus what arrived while
// paused. Each row is a few hundred bytes.

import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { requests, watermarks, type RequestPage, type RequestRow } from "../api.ts";
import { ignored } from "../keys.ts";
import { navigate, requestLink } from "../router.ts";

/** A row's height in pixels. `.row` in `style.css` has the same. */
export const ROW_HEIGHT = 28;
/** Rows kept in the DOM above and below the screen, so a short scroll shows no gap. */
const OVERSCAN = 6;
/** How close to the last loaded row the screen comes before the next older page loads. */
const NEAR_END = 40;
/** The rows a following list keeps. Older ones are dropped and load again with `before`. */
const KEEP = 2000;

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

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function RequestList() {
  const [rows, setRows] = createSignal<RequestRow[]>([]);
  const [older, setOlder] = createSignal<number | null>(null);
  const [loaded, setLoaded] = createSignal(false);
  const [loadingOlder, setLoadingOlder] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [paused, setPaused] = createSignal(false);
  /** What arrived while paused, newest first. */
  const [arrived, setArrived] = createSignal<RequestRow[]>([]);
  /** More arrived while paused than one page gave, so resuming loads the newest page again. */
  const [overflow, setOverflow] = createSignal(false);
  /** The selected row's id. An id, not a position, so rows put on top do not move the selection. */
  const [selected, setSelected] = createSignal<number | null>(null);
  const [scrollTop, setScrollTop] = createSignal(0);
  const [height, setHeight] = createSignal(0);
  let list!: HTMLDivElement;
  /** The last watermark this list is complete up to. */
  let seen = 0;
  /** Rises with every reload of the newest page, so an older page asked for before it is dropped. */
  let generation = 0;
  /** Loads and live updates run one after another, so two never interleave their rows. */
  let chain = Promise.resolve();
  const abort = new AbortController();
  const serial = (job: () => Promise<void>) => {
    chain = chain.then(job).catch((err) => {
      if (!abort.signal.aborted) setError(message(err));
    });
  };

  const index = createMemo(() => new Map(rows().map((row, i) => [row.id, i])));
  const current = () => {
    const id = selected();
    return id === null ? -1 : (index().get(id) ?? -1);
  };
  const first = createMemo(() => Math.max(0, Math.floor(scrollTop() / ROW_HEIGHT) - OVERSCAN));
  const end = createMemo(() => Math.min(rows().length, Math.ceil((scrollTop() + height()) / ROW_HEIGHT) + OVERSCAN));
  const visible = createMemo(() => rows().slice(first(), end()));
  /** `aria-setsize`: the loaded rows, or `-1` (unknown) while older pages remain. */
  const setSize = () => (older() === null ? rows().length : -1);
  const active = () => {
    const i = current();
    return i >= first() && i < end() ? `request-${selected()}` : undefined;
  };

  const keep = (next: RequestRow[]) => {
    if (next.length <= KEEP) return setRows(next);
    const kept = next.slice(0, KEEP);
    setRows(kept);
    setOlder(kept[KEEP - 1]!.id);
  };

  const showNewest = (page: RequestPage) => {
    generation++;
    seen = page.watermark;
    setRows(page.requests);
    setOlder(page.older);
    setArrived([]);
    setOverflow(false);
    if (current() < 0) setSelected(page.requests[0]?.id ?? null);
    setLoaded(true);
  };

  const onBatch = (batch: number) =>
    serial(async () => {
      if (batch <= seen) return;
      const page = await requests({ since: seen }, abort.signal);
      if (page.older !== null) {
        if (!paused()) return showNewest(await requests({}, abort.signal));
        seen = page.watermark;
        setOverflow(true);
        setArrived([...page.requests, ...arrived()]);
        return;
      }
      seen = Math.max(seen, page.watermark);
      const top = (arrived()[0] ?? rows()[0])?.id ?? 0;
      const fresh = page.requests.filter((row) => row.id > top);
      if (fresh.length === 0) return;
      if (paused()) setArrived([...fresh, ...arrived()]);
      else keep([...fresh, ...rows()]);
    });

  const resume = () => {
    setPaused(false);
    if (overflow()) serial(async () => showNewest(await requests({}, abort.signal)));
    else {
      keep([...arrived(), ...rows()]);
      setArrived([]);
    }
    list.scrollTop = 0;
  };
  const toggle = () => (paused() ? resume() : setPaused(true));

  const loadOlder = (before: number) => {
    const asked = generation;
    setLoadingOlder(true);
    requests({ before }, abort.signal)
      .then((page) => {
        if (asked !== generation) return;
        const bottom = rows().at(-1)?.id ?? Infinity;
        setRows([...rows(), ...page.requests.filter((row) => row.id < bottom)]);
        setOlder(page.older);
      })
      .catch((err) => {
        if (!abort.signal.aborted) setError(message(err));
      })
      .finally(() => setLoadingOlder(false));
  };

  createEffect(() => {
    const before = older();
    if (loaded() && before !== null && !loadingOlder() && !error() && end() >= rows().length - NEAR_END) loadOlder(before);
  });

  const onScroll = () => {
    const top = list.scrollTop;
    setScrollTop(top);
    if (top > ROW_HEIGHT / 2 && !paused()) setPaused(true);
    else if (top === 0 && paused()) resume();
  };

  /** Selects the row at position `to`, kept inside the list, and scrolls it onto the screen. */
  const move = (to: number) => {
    const all = rows();
    if (all.length === 0) return;
    const i = Math.min(all.length - 1, Math.max(0, to));
    setSelected(all[i]!.id);
    const top = i * ROW_HEIGHT;
    if (top < list.scrollTop) list.scrollTop = top;
    else if (top + ROW_HEIGHT > list.scrollTop + list.clientHeight) list.scrollTop = top + ROW_HEIGHT - list.clientHeight;
    list.focus({ preventScroll: true });
  };
  const open = (row: RequestRow) => navigate(requestLink(row));
  const screen = () => Math.max(1, Math.floor(list.clientHeight / ROW_HEIGHT) - 1);

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
        move(rows().length - 1);
        break;
      case "PageDown":
        move(current() + screen());
        break;
      case "PageUp":
        move(current() - screen());
        break;
      case "Enter": {
        const row = rows()[current()];
        if (!row) return;
        open(row);
        break;
      }
      case "p":
      case " ":
        toggle();
        break;
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
    serial(async () => showNewest(await requests({}, abort.signal)));
    watermarks(onBatch, abort.signal);
    onCleanup(() => {
      abort.abort();
      resize.disconnect();
      document.removeEventListener("keydown", onKey);
    });
  });

  const arrivedText = () => {
    const n = arrived().length;
    if (overflow()) return `More than ${n} new requests`;
    return n === 1 ? "1 new request" : `${n} new requests`;
  };

  return (
    <section class="requests" aria-label="Requests">
      <div class="follow">
        <span role="status" class="follow-state">
          {paused() ? "Paused." : "Following new requests."}
        </span>
        <Show
          when={paused() && arrived().length > 0}
          fallback={
            <button type="button" class="btn" onClick={toggle} aria-keyshortcuts="p">
              {paused() ? "Follow" : "Pause"}
            </button>
          }
        >
          <button type="button" class="btn arrived" onClick={resume} aria-keyshortcuts="p">
            {arrivedText()}
          </button>
        </Show>
      </div>
      <Show when={error()}>
        <p class="note bad" role="alert">
          The list could not load: {error()}
        </p>
      </Show>
      <Show when={loaded() && rows().length === 0}>
        <p class="note">No requests yet. They show here when your server handles them.</p>
      </Show>
      <div class="cols" aria-hidden="true">
        <span>Method</span>
        <span>Route</span>
        <span class="num">Status</span>
        <span class="num">Duration</span>
        <span class="num">Queries</span>
        <span class="num">Dumps</span>
        <span>Exception</span>
      </div>
      <div
        ref={list}
        class="list"
        role="listbox"
        tabindex="0"
        aria-label="Requests, newest first"
        aria-activedescendant={active()}
        data-rows={rows().length}
        onScroll={onScroll}
      >
        <div class="list-space" role="none" style={{ height: `${rows().length * ROW_HEIGHT}px` }}>
          <For each={visible()}>
            {(row, i) => {
              const at = () => first() + i();
              return (
                <div
                  id={`request-${row.id}`}
                  role="option"
                  class="row"
                  classList={{ threw: row.exception !== null }}
                  aria-selected={selected() === row.id ? "true" : "false"}
                  aria-posinset={at() + 1}
                  aria-setsize={setSize()}
                  style={{ transform: `translateY(${at() * ROW_HEIGHT}px)` }}
                  onClick={() => {
                    setSelected(row.id);
                    open(row);
                  }}
                >
                  <span class="method">{row.method}</span>
                  <span class="route" title={row.path}>
                    {row.route ?? row.path}
                  </span>
                  <span class={`num status ${statusClass(row.status)}`}>{row.status}</span>
                  <span class="num duration">{duration(row.duration_us)}</span>
                  <span class="num queries">{row.queries}</span>
                  <span class="num dumps">{row.dumps}</span>
                  <span class="exc" title={row.exception ? `${row.exception.file}:${row.exception.line}` : undefined}>
                    {row.exception?.class ?? ""}
                  </span>
                </div>
              );
            }}
          </For>
        </div>
      </div>
    </section>
  );
}
