// The one list every view draws: `List`, and `createFeed`, which keeps a list's rows up to date.
//
// `List` is virtual. Every row has the same height, so the rows on screen follow from the scroll
// position alone, and only they and a few rows on each side are in the DOM. The browser's cost is the
// screen, not the index. It follows the WAI-ARIA listbox pattern: the list has the focus, and
// `aria-activedescendant` names the selected row. `j`, `k`, the arrows, `Home`, `End` and the page
// keys move the selection, `Enter` opens the selected row, and `p` or Space pause or follow when the
// list follows new rows. A view adds its own keys, such as `m`, through `keys`.
//
// `createFeed` loads the newest page, follows each watermark `/api/events` pushes, and loads older
// pages as the end comes near. A paused feed never changes what is on screen: what arrives is kept
// aside and counted, and resuming puts it on top. A `since` page that is not complete (`older` set)
// means more arrived than one page, so the newest page is loaded again instead.
//
// A filtered page can hold fewer rows than the screen and still have `older` set, because the
// service reads a bounded range of ids for it. The list then loads the next range at once, and goes
// on until the screen is full or no older range is left.
//
// What it spends: the rows loaded so far, at most `KEEP` while following, plus what arrived while
// paused. A list's DOM is the rows on screen and `OVERSCAN` rows on each side.

import { type Accessor, createEffect, createMemo, createSignal, For, type JSX, onCleanup, onMount, Show } from "solid-js";
import { watermarks } from "../api.ts";
import { ignored } from "../keys.ts";

/** A row's height in pixels, unless a list sets its own. */
export const ROW_HEIGHT = 32;
/** Rows kept in the DOM above and below the screen, so a short scroll shows no gap. */
const OVERSCAN = 6;
/** How close to the last loaded row the screen comes before the next older page loads. */
const NEAR_END = 40;
/** The rows a following feed keeps. Older ones are dropped and load again with `before`. */
export const KEEP = 2000;

/** What names a row: its id or its key. */
export type RowKey = string | number;

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** One page of a feed, newest first. */
export interface FeedPage<T, C> {
  rows: T[];
  /** The cursor of the next older page, or `null` when this page has the oldest row. */
  older: C | null;
  /** The last committed batch this page is complete up to. */
  watermark: number;
}

export interface FeedOptions<T, C> {
  /** Loads the newest page, the rows after `since`, or the page before `before`. */
  load: (at: { since?: number; before?: C }, signal: AbortSignal) => Promise<FeedPage<T, C>>;
  key: (row: T) => RowKey;
  /** The `before` that loads the rows older than `row`. */
  cursor: (row: T) => C;
}

/** A list's rows and how it follows new ones. `List` takes it as `feed`. */
export interface Feed<T> {
  rows: Accessor<T[]>;
  loaded: Accessor<boolean>;
  error: Accessor<string | null>;
  /** True while older pages remain to load. */
  more: Accessor<boolean>;
  /** True while an older page loads. */
  busy: Accessor<boolean>;
  /** The newest watermark the event stream pushed, or `0` before the first. */
  batch: Accessor<number>;
  paused: Accessor<boolean>;
  /** How many rows arrived while paused. */
  arrived: Accessor<number>;
  /** True when more arrived while paused than one page gave. */
  overflow: Accessor<boolean>;
  pause: () => void;
  resume: () => void;
  /** Loads the next older page, unless one is loading or none remains. */
  older: () => void;
}

/** Loads a feed and follows it until the owner is cleaned up. */
export function createFeed<T, C>(opts: FeedOptions<T, C>): Feed<T> {
  const [rows, setRows] = createSignal<T[]>([]);
  const [older, setOlder] = createSignal<C | null>(null);
  const [loaded, setLoaded] = createSignal(false);
  const [loadingOlder, setLoadingOlder] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [paused, setPaused] = createSignal(false);
  const [arrived, setArrived] = createSignal<T[]>([]);
  const [overflow, setOverflow] = createSignal(false);
  const [batch, setBatch] = createSignal(0);
  /** The last watermark this feed is complete up to. */
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
  const newest = () => opts.load({}, abort.signal);

  const keep = (next: T[]) => {
    if (next.length <= KEEP) return setRows(next);
    const kept = next.slice(0, KEEP);
    setRows(kept);
    setOlder(() => opts.cursor(kept[KEEP - 1]!));
  };

  const showNewest = (page: FeedPage<T, C>) => {
    generation++;
    seen = page.watermark;
    setRows(page.rows);
    setOlder(() => page.older);
    setArrived([]);
    setOverflow(false);
    setLoaded(true);
  };

  const onBatch = (batch: number) =>
    serial(async () => {
      if (batch <= seen) return;
      const page = await opts.load({ since: seen }, abort.signal);
      if (page.older !== null) {
        if (!paused()) return showNewest(await newest());
        seen = page.watermark;
        setOverflow(true);
        setArrived([...page.rows, ...arrived()]);
        return;
      }
      seen = Math.max(seen, page.watermark);
      const have = new Set([...arrived(), ...rows()].map(opts.key));
      const fresh = page.rows.filter((row) => !have.has(opts.key(row)));
      if (fresh.length === 0) return;
      if (paused()) setArrived([...fresh, ...arrived()]);
      else keep([...fresh, ...rows()]);
    });

  const resume = () => {
    setPaused(false);
    if (overflow()) serial(async () => showNewest(await newest()));
    else {
      keep([...arrived(), ...rows()]);
      setArrived([]);
    }
  };

  const loadOlder = () => {
    const before = older();
    if (!loaded() || before === null || loadingOlder() || error()) return;
    const asked = generation;
    setLoadingOlder(true);
    opts
      .load({ before }, abort.signal)
      .then((page) => {
        if (asked !== generation) return;
        const have = new Set(rows().map(opts.key));
        setRows([...rows(), ...page.rows.filter((row) => !have.has(opts.key(row)))]);
        setOlder(() => page.older);
      })
      .catch((err) => {
        if (!abort.signal.aborted) setError(message(err));
      })
      .finally(() => setLoadingOlder(false));
  };

  serial(async () => showNewest(await newest()));
  watermarks((next) => {
    setBatch(next);
    onBatch(next);
  }, abort.signal);
  onCleanup(() => abort.abort());

  return {
    rows,
    loaded,
    error,
    more: () => older() !== null,
    busy: loadingOlder,
    batch,
    paused,
    arrived: () => arrived().length,
    overflow,
    pause: () => setPaused(true),
    resume,
    older: loadOlder,
  };
}

export interface ListProps<T> {
  rows: T[];
  key: (row: T) => RowKey;
  /** The start of each row's element id, such as `request` for `request-7`. */
  idPrefix: string;
  /** The listbox's accessible name. */
  label: string;
  /** The selected row's key. A key, not a position, so rows put on top do not move the selection. */
  selected: RowKey | null;
  onSelect: (key: RowKey) => void;
  /** Runs on `Enter`, and on a click when `openOnClick` is set. */
  onOpen?: (row: T) => void;
  openOnClick?: boolean;
  /** The row's cells. */
  children: (row: T) => JSX.Element;
  /** The column titles, drawn above the list. */
  cols?: JSX.Element;
  /** The class that sets the grid of the columns and the rows. */
  grid?: string;
  rowHeight?: number;
  /** Extra classes of one row, such as `threw`. */
  rowClass?: (row: T) => Record<string, boolean>;
  /** A row's `data-kind`. */
  rowKind?: (row: T) => string | undefined;
  /** A row's `aria-description`. */
  describe?: (row: T) => string | undefined;
  /** True while older rows remain. The list then gives `aria-setsize` as `-1`, unknown. */
  more?: boolean;
  /** Runs as the screen comes near the last loaded row. */
  onEnd?: () => void;
  /** A followed list: the follow bar, pausing on scroll and the `p` key. */
  feed?: Feed<T>;
  /** The singular and the plural of what a row is, as in "1 new request". */
  noun?: readonly [string, string];
  /** Drawn in the follow bar after its own buttons. */
  actions?: JSX.Element;
  /** What shows when the list is loaded and empty. */
  empty?: JSX.Element;
  /** True once the first page loaded, so an empty list is shown as empty. */
  loaded?: boolean;
  error?: string | null;
  /** The view's own keys, by `KeyboardEvent.key`. Each runs with the selected row. */
  keys?: Record<string, (row: T | undefined) => void>;
  /** Runs with the rows in the DOM each time they change, such as to scale a bar to the slowest. */
  onShown?: (rows: T[]) => void;
}

/** The one virtual list. */
export function List<T>(props: ListProps<T>) {
  const height = () => props.rowHeight ?? ROW_HEIGHT;
  const [scrollTop, setScrollTop] = createSignal(0);
  const [screenHeight, setScreenHeight] = createSignal(0);
  let list!: HTMLDivElement;

  const index = createMemo(() => new Map(props.rows.map((row, i) => [props.key(row), i])));
  const current = () => (props.selected === null ? -1 : (index().get(props.selected) ?? -1));
  const chosen = () => props.rows[current()];
  const first = createMemo(() => Math.max(0, Math.floor(scrollTop() / height()) - OVERSCAN));
  const end = createMemo(() => Math.min(props.rows.length, Math.ceil((scrollTop() + screenHeight()) / height()) + OVERSCAN));
  const visible = createMemo(() => props.rows.slice(first(), end()));
  const active = () => {
    const i = current();
    return i >= first() && i < end() ? `${props.idPrefix}-${props.selected}` : undefined;
  };
  const feed = () => props.feed;
  const more = () => props.more ?? feed()?.more() ?? false;

  // The first row is selected once there is one and nothing is.
  createEffect(() => {
    const top = props.rows[0];
    if (props.selected === null && top !== undefined) props.onSelect(props.key(top));
  });

  // A list with no row yet and older rows left loads them too, as a filtered page can be empty.
  // Reading `busy` runs this again once a page has loaded, so a short page loads the next one.
  createEffect(() => {
    if (feed()?.busy()) return;
    if ((props.rows.length > 0 || more()) && end() >= props.rows.length - NEAR_END) (props.onEnd ?? feed()?.older)?.();
  });

  createEffect(() => props.onShown?.(visible()));

  const resume = () => {
    feed()?.resume();
    list.scrollTop = 0;
  };
  const toggle = () => {
    const f = feed();
    if (!f) return;
    if (f.paused()) resume();
    else f.pause();
  };

  const onScroll = () => {
    const top = list.scrollTop;
    setScrollTop(top);
    const f = feed();
    if (!f) return;
    if (top > height() / 2 && !f.paused()) f.pause();
    else if (top === 0 && f.paused()) resume();
  };

  /** Selects the row at position `to`, kept inside the list, and scrolls it onto the screen. */
  const move = (to: number) => {
    const all = props.rows;
    if (all.length === 0) return;
    const i = Math.min(all.length - 1, Math.max(0, to));
    props.onSelect(props.key(all[i]!));
    const top = i * height();
    if (top < list.scrollTop) list.scrollTop = top;
    else if (top + height() > list.scrollTop + list.clientHeight) list.scrollTop = top + height() - list.clientHeight;
    list.focus({ preventScroll: true });
  };
  const screen = () => Math.max(1, Math.floor(list.clientHeight / height()) - 1);

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e)) return;
    const own = props.keys?.[e.key];
    if (own) {
      own(chosen());
      e.preventDefault();
      return;
    }
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
        const row = chosen();
        if (!row || !props.onOpen) return;
        props.onOpen(row);
        break;
      }
      case "p":
      case " ":
        if (!feed()) return;
        toggle();
        break;
      default:
        return;
    }
    e.preventDefault();
  };

  onMount(() => {
    setScreenHeight(list.clientHeight);
    const resize = new ResizeObserver(() => setScreenHeight(list.clientHeight));
    resize.observe(list);
    document.addEventListener("keydown", onKey);
    onCleanup(() => {
      resize.disconnect();
      document.removeEventListener("keydown", onKey);
    });
  });

  const noun = () => props.noun ?? ["row", "rows"];
  const arrivedText = () => {
    const n = feed()?.arrived() ?? 0;
    const [one, many] = noun();
    if (feed()?.overflow()) return `More than ${n} new ${many}`;
    return n === 1 ? `1 new ${one}` : `${n} new ${many}`;
  };
  const error = () => props.error ?? feed()?.error() ?? null;
  const loaded = () => props.loaded ?? feed()?.loaded() ?? true;

  return (
    <div class="list-view">
      <Show when={feed() || props.actions}>
        <div class="follow">
          <Show when={feed()}>
            {(f) => (
              <>
                <span role="status" class="follow-state">
                  <span class="dot" classList={{ paused: f().paused() }} aria-hidden="true" />
                  {f().paused() ? "Paused." : `Following new ${noun()[1]}.`}
                </span>
                <Show
                  when={f().paused() && f().arrived() > 0}
                  fallback={
                    <button type="button" class="btn small" onClick={toggle} aria-keyshortcuts="p">
                      {f().paused() ? "Follow" : "Pause"}
                    </button>
                  }
                >
                  <button type="button" class="btn small arrived" onClick={resume} aria-keyshortcuts="p">
                    {arrivedText()}
                  </button>
                </Show>
              </>
            )}
          </Show>
          {props.actions}
        </div>
      </Show>
      <Show when={error()}>
        <p class="note bad" role="alert">
          The list could not load: {error()}
        </p>
      </Show>
      <Show when={loaded() && props.rows.length === 0 && props.empty}>{props.empty}</Show>
      <Show when={props.cols}>
        <div class={`cols ${props.grid ?? ""}`} aria-hidden="true">
          {props.cols}
        </div>
      </Show>
      <div
        ref={list}
        class="list"
        role="listbox"
        tabindex="0"
        aria-label={props.label}
        aria-activedescendant={active()}
        data-rows={props.rows.length}
        onScroll={onScroll}
      >
        <div class="list-space" role="none" style={{ height: `${props.rows.length * height()}px` }}>
          <For each={visible()}>
            {(row, i) => {
              const at = () => first() + i();
              const key = props.key(row);
              return (
                <div
                  id={`${props.idPrefix}-${key}`}
                  role="option"
                  class={`row ${props.grid ?? ""}`}
                  classList={props.rowClass?.(row)}
                  data-kind={props.rowKind?.(row)}
                  aria-description={props.describe?.(row)}
                  aria-selected={props.selected === key ? "true" : "false"}
                  aria-posinset={at() + 1}
                  aria-setsize={more() ? -1 : props.rows.length}
                  style={{ transform: `translateY(${at() * height()}px)`, height: `${height()}px` }}
                  onClick={() => {
                    props.onSelect(key);
                    if (props.openOnClick) props.onOpen?.(row);
                  }}
                >
                  {props.children(row)}
                </div>
              );
            }}
          </For>
        </div>
      </div>
    </div>
  );
}
