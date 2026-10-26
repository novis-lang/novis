// The flat stream at `/stream?<filter>`: every record of every kind, newest first, filtered by kind,
// level, route and trace, with the filter in the link. The log view (`log.tsx`) is this feed with
// `kind=log` and its own columns.
//
// The feed follows each watermark `/api/events` pushes with `since`, pauses at a key or when the
// developer scrolls away from the newest row, and counts what arrived while paused, the way the
// request list does (`requests.tsx`). Older pages load with `before` as the end comes near. Every row
// has the same height and only the rows on screen are in the DOM; the selected record's fields show
// below the list. A row that is a line the service could not read, or a count of records it dropped,
// is shown with a badge and its fields.
//
// Below the stream, a panel lists the files the service stopped reading, and has the action that
// discards the index.
//
// What it spends: the records loaded so far, at most `KEEP` while following, plus what arrived while
// paused. Each record is a few hundred bytes and its fields.

import { createEffect, createMemo, createResource, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";
import {
  discardIndex,
  quarantine,
  RECORD_KINDS,
  stream,
  streamFilter,
  streamQuery,
  watermarks,
  type Json,
  type RecordKind,
  type StreamFilter,
  type StreamPage,
  type StreamRecord,
} from "../api.ts";
import { ignored } from "../keys.ts";
import { navigate, query, reloadView, requestLink } from "../router.ts";
import { ROW_HEIGHT, statusClass } from "./requests.tsx";
import { Confirm, SaveButton } from "./saved.tsx";
import { ValueTree } from "./value-tree.tsx";

/** Rows kept in the DOM above and below the screen. */
const OVERSCAN = 6;
/** How close to the last loaded row the screen comes before the next older page loads. */
const NEAR_END = 40;
/** The rows a following feed keeps. Older ones are dropped and load again with `before`. */
const KEEP = 2000;

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

const pad = (n: number, width = 2) => String(n).padStart(width, "0");

/** A Unix time in microseconds as the local time of day, to the millisecond. */
export function clock(us: number): string {
  const d = new Date(Math.floor(us / 1000));
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${pad(d.getMilliseconds(), 3)}`;
}

/** A Unix time in microseconds as the local date and time. */
export function when(us: number): string {
  const d = new Date(Math.floor(us / 1000));
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${clock(us)}`;
}

/** A record's fields on one line, or nothing when it has none. */
export function fieldsLine(fields: Json): string {
  if (fields === null) return "";
  if (typeof fields === "object" && Object.keys(fields).length === 0) return "";
  return JSON.stringify(fields);
}

/** The `before` cursor of `record`: its time and its key. */
const cursor = (record: StreamRecord) => `${record.at_us}.${record.key}`;

/** The badge of a row that is not a record of the program. */
function flaw(record: StreamRecord): string | null {
  if (record.kind === "unparseable") return "Unreadable line";
  if (record.kind === "dropped") return "Records dropped";
  return null;
}

/** The request a record belongs to: method, path and status, linked to the request's page. */
export function OwnerCell(props: { record: StreamRecord }) {
  return (
    <span class="owner">
      <Show when={props.record.request} fallback={<span class="note">No request</span>}>
        {(owner) => (
          <a href={requestLink(owner())} tabindex="-1">
            {owner().method} {owner().path} <span class={`status ${statusClass(owner().status)}`}>{owner().status}</span>
          </a>
        )}
      </Show>
    </span>
  );
}

/** What a feed shows: its filter, its words and its columns. */
export interface FeedProps {
  filter: StreamFilter;
  /** The listbox's name. */
  label: string;
  /** The singular and the plural of what a row is, as in "1 new record". */
  noun: readonly [string, string];
  /** What shows when the filter keeps nothing. */
  empty: string;
  /** The class that sets the grid of the columns and the rows. */
  grid: string;
  /** The column titles. */
  cols: JSX.Element;
  /** A row's cells. */
  cells: (record: StreamRecord) => JSX.Element;
}

/** A live, virtual list of stream records with one filter. A new filter is a new feed. */
export function Feed(props: FeedProps) {
  const filter = props.filter;
  const [rows, setRows] = createSignal<StreamRecord[]>([]);
  const [older, setOlder] = createSignal<string | null>(null);
  const [loaded, setLoaded] = createSignal(false);
  const [loadingOlder, setLoadingOlder] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [paused, setPaused] = createSignal(false);
  const [arrived, setArrived] = createSignal<StreamRecord[]>([]);
  const [overflow, setOverflow] = createSignal(false);
  const [selected, setSelected] = createSignal<number | null>(null);
  const [scrollTop, setScrollTop] = createSignal(0);
  const [height, setHeight] = createSignal(0);
  let list!: HTMLDivElement;
  let seen = 0;
  let generation = 0;
  let chain = Promise.resolve();
  const abort = new AbortController();
  const serial = (job: () => Promise<void>) => {
    chain = chain.then(job).catch((err) => {
      if (!abort.signal.aborted) setError(message(err));
    });
  };
  const newest = () => stream(filter, {}, abort.signal);

  const index = createMemo(() => new Map(rows().map((row, i) => [row.key, i])));
  const current = () => {
    const key = selected();
    return key === null ? -1 : (index().get(key) ?? -1);
  };
  const chosen = () => rows()[current()];
  const first = createMemo(() => Math.max(0, Math.floor(scrollTop() / ROW_HEIGHT) - OVERSCAN));
  const end = createMemo(() => Math.min(rows().length, Math.ceil((scrollTop() + height()) / ROW_HEIGHT) + OVERSCAN));
  const visible = createMemo(() => rows().slice(first(), end()));
  const setSize = () => (older() === null ? rows().length : -1);
  const active = () => {
    const i = current();
    return i >= first() && i < end() ? `record-${selected()}` : undefined;
  };

  const keep = (next: StreamRecord[]) => {
    if (next.length <= KEEP) return setRows(next);
    const kept = next.slice(0, KEEP);
    setRows(kept);
    setOlder(cursor(kept[KEEP - 1]!));
  };

  const showNewest = (page: StreamPage) => {
    generation++;
    seen = page.watermark;
    setRows(page.records);
    setOlder(page.older);
    setArrived([]);
    setOverflow(false);
    if (current() < 0) setSelected(page.records[0]?.key ?? null);
    setLoaded(true);
  };

  const onBatch = (batch: number) =>
    serial(async () => {
      if (batch <= seen) return;
      const page = await stream(filter, { since: seen }, abort.signal);
      if (page.older !== null) {
        if (!paused()) return showNewest(await newest());
        seen = page.watermark;
        setOverflow(true);
        setArrived([...page.records, ...arrived()]);
        return;
      }
      seen = Math.max(seen, page.watermark);
      const have = new Set([...arrived(), ...rows()].map((row) => row.key));
      const fresh = page.records.filter((row) => !have.has(row.key));
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
    list.scrollTop = 0;
  };
  const toggle = () => (paused() ? resume() : setPaused(true));

  const loadOlder = (before: string) => {
    const asked = generation;
    setLoadingOlder(true);
    stream(filter, { before }, abort.signal)
      .then((page) => {
        if (asked !== generation) return;
        const have = new Set(rows().map((row) => row.key));
        setRows([...rows(), ...page.records.filter((row) => !have.has(row.key))]);
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

  const move = (to: number) => {
    const all = rows();
    if (all.length === 0) return;
    const i = Math.min(all.length - 1, Math.max(0, to));
    setSelected(all[i]!.key);
    const top = i * ROW_HEIGHT;
    if (top < list.scrollTop) list.scrollTop = top;
    else if (top + ROW_HEIGHT > list.scrollTop + list.clientHeight) list.scrollTop = top + ROW_HEIGHT - list.clientHeight;
    list.focus({ preventScroll: true });
  };
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
        const owner = chosen()?.request;
        if (!owner) return;
        navigate(requestLink(owner));
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
    serial(async () => showNewest(await newest()));
    watermarks(onBatch, abort.signal);
    onCleanup(() => {
      abort.abort();
      resize.disconnect();
      document.removeEventListener("keydown", onKey);
    });
  });

  const arrivedText = () => {
    const n = arrived().length;
    const [one, many] = props.noun;
    if (overflow()) return `More than ${n} new ${many}`;
    return n === 1 ? `1 new ${one}` : `${n} new ${many}`;
  };

  return (
    <div class="feed">
      <div class="follow">
        <span role="status" class="follow-state">
          {paused() ? "Paused." : `Following new ${props.noun[1]}.`}
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
        <p class="note">{props.empty}</p>
      </Show>
      <div class={`cols ${props.grid}`} aria-hidden="true">
        {props.cols}
      </div>
      <div
        ref={list}
        class="list"
        role="listbox"
        tabindex="0"
        aria-label={props.label}
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
                  id={`record-${row.key}`}
                  role="option"
                  class={`row ${props.grid}`}
                  classList={{ flawed: flaw(row) !== null }}
                  data-kind={row.kind}
                  aria-selected={selected() === row.key ? "true" : "false"}
                  aria-posinset={at() + 1}
                  aria-setsize={setSize()}
                  style={{ transform: `translateY(${at() * ROW_HEIGHT}px)` }}
                  onClick={() => setSelected(row.key)}
                >
                  {props.cells(row)}
                </div>
              );
            }}
          </For>
        </div>
      </div>
      <Show when={chosen()}>{(record) => <RecordDetail record={record()} />}</Show>
    </div>
  );
}

/** The selected record in full: its kind, time, text, site, request and fields. */
function RecordDetail(props: { record: StreamRecord }) {
  const r = () => props.record;
  return (
    <section class="record-detail" aria-label="Selected record">
      <Show when={r().kind === "unparseable"}>
        <p class="note bad">This line in a record file could not be read. The fields below say why.</p>
      </Show>
      <Show when={r().kind === "dropped"}>
        <p class="note bad">Some records were dropped before they were written. The fields below say how many.</p>
      </Show>
      <dl class="fields">
        <dt>Kind</dt>
        <dd class="detail-kind">{r().kind}</dd>
        <dt>Time</dt>
        <dd>{when(r().at_us)}</dd>
        <dt>Text</dt>
        <dd class="detail-text">{r().text}</dd>
        <Show when={r().level}>
          <dt>Level</dt>
          <dd>{r().level}</dd>
        </Show>
        <Show when={r().file}>
          <dt>File</dt>
          <dd class="site">
            {r().file}
            {r().line !== null ? `:${r().line}` : ""}
          </dd>
        </Show>
        <Show when={r().request}>
          {(owner) => (
            <>
              <dt>Request</dt>
              <dd>
                <a href={requestLink(owner())} class="detail-request">
                  {owner().method} {owner().path}
                </a>{" "}
                <span class={`status ${statusClass(owner().status)}`}>{owner().status}</span>
              </dd>
            </>
          )}
        </Show>
        <Show when={fieldsLine(r().fields)}>
          <dt>Fields</dt>
          <dd>
            <ValueTree value={r().fields} prefix={`record-${r().key}-fields`} name="Fields of the selected record" />
          </dd>
        </Show>
      </dl>
    </section>
  );
}

/** The filter form of the stream and the log: kind (the stream only), level, route and trace. */
export function FilterForm(props: { view: "stream" | "log"; filter: StreamFilter }) {
  let level!: HTMLInputElement;
  let route!: HTMLInputElement;
  let trace!: HTMLInputElement;
  const go = (next: StreamFilter) => {
    // The log's link has no `kind`: the log is always `kind=log`.
    const q = streamQuery(props.view === "log" ? { ...next, kinds: [] } : next);
    navigate(q ? `/${props.view}?${q}` : `/${props.view}`);
  };
  const typed = (): StreamFilter => {
    const next: StreamFilter = { kinds: props.filter.kinds };
    const l = level.value.trim().toLowerCase();
    const r = route.value.trim();
    const t = trace.value.trim().toLowerCase();
    if (l) next.level = l;
    if (r) next.route = r;
    if (t) next.trace = t;
    return next;
  };
  const toggleKind = (kind: RecordKind, on: boolean) => {
    const kinds = RECORD_KINDS.filter((one) => (one === kind ? on : props.filter.kinds.includes(one)));
    go({ ...typed(), kinds });
  };
  return (
    <form
      class="filters"
      aria-label="Filter"
      onSubmit={(e) => {
        e.preventDefault();
        go(typed());
      }}
    >
      <Show when={props.view === "stream"}>
        <fieldset class="kinds">
          <legend>Kinds</legend>
          <For each={RECORD_KINDS}>
            {(kind) => (
              <label>
                <input
                  type="checkbox"
                  name="kind"
                  value={kind}
                  checked={props.filter.kinds.includes(kind)}
                  onChange={(e) => toggleKind(kind, e.currentTarget.checked)}
                />
                {kind}
              </label>
            )}
          </For>
        </fieldset>
      </Show>
      <label>
        Level <input ref={level} name="level" list="levels" size="8" value={props.filter.level ?? ""} autocomplete="off" />
      </label>
      <datalist id="levels">
        <For each={["debug", "info", "warning", "error", "critical"]}>{(one) => <option value={one} />}</For>
      </datalist>
      <label>
        Route <input ref={route} name="route" size="16" placeholder="/items/{id}" value={props.filter.route ?? ""} autocomplete="off" />
      </label>
      <label>
        Trace id <input ref={trace} name="trace" size="20" value={props.filter.trace ?? ""} autocomplete="off" spellcheck={false} />
      </label>
      <button type="submit" class="btn">
        Apply
      </button>
    </form>
  );
}

/** The files the service stopped reading, and the action that discards the index. */
export function Quarantine() {
  const [files] = createResource(() => quarantine());
  let ask!: () => void;
  const list = () => (files.error ? [] : (files()?.files ?? []));
  return (
    <section class="quarantine" aria-labelledby="quarantine-title">
      <h2 id="quarantine-title">Files that could not be read</h2>
      <Show when={files.error}>
        <p class="note bad" role="alert">
          The list of files could not load: {message(files.error)}
        </p>
      </Show>
      <Show when={!files.loading && !files.error && list().length === 0}>
        <p class="note">Every record file was read.</p>
      </Show>
      <Show when={list().length > 0}>
        <table class="quarantined">
          <thead>
            <tr>
              <th scope="col">File</th>
              <th scope="col">Error</th>
              <th scope="col" class="num">
                Failures
              </th>
              <th scope="col">Next try</th>
            </tr>
          </thead>
          <tbody>
            <For each={list()}>
              {(file) => (
                <tr>
                  <td class="site">{file.path}</td>
                  <td class="bad">{file.reason}</td>
                  <td class="num">{file.failures}</td>
                  <td>{when(file.retry_us)}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>
      <button type="button" class="btn danger discard" onClick={() => ask()}>
        Discard the index
      </button>
      <Confirm
        ref={(open) => (ask = open)}
        title="Discard the index?"
        action="Discard"
        onConfirm={async () => {
          await discardIndex();
          reloadView();
        }}
      >
        <p>This deletes every record the viewer has read. Your saved filters are kept.</p>
      </Confirm>
    </section>
  );
}

/** The stream's column titles: time, kind, text and request. */
const cols = () => (
  <>
    <span>Time</span>
    <span>Kind</span>
    <span>Text</span>
    <span>Request</span>
  </>
);

export function StreamView() {
  return (
    <section class="stream" aria-label="Stream">
      <div class="toolbar">
        <FilterForm view="stream" filter={streamFilter(query())} />
        <SaveButton view="stream" query={query} />
      </div>
      {/* A new link is a new filter, and a new filter is a new feed. */}
      <For each={[query()]}>
        {(q) => (
          <Feed
            filter={streamFilter(q)}
            label="Records, newest first"
            noun={["record", "records"]}
            empty="No records match this filter. New records show here when your program writes them."
            grid="stream-grid"
            cols={cols()}
            cells={(row) => (
              <>
                <span class="at">{clock(row.at_us)}</span>
                <span class="kind">
                  <Show when={flaw(row)} fallback={row.kind}>
                    {(badge) => <span class="badge bad">{badge()}</span>}
                  </Show>
                </span>
                <span class="text" title={row.text}>
                  {row.text}
                  <Show when={flaw(row) && fieldsLine(row.fields)}>
                    <span class="note"> {fieldsLine(row.fields)}</span>
                  </Show>
                </span>
                <OwnerCell record={row} />
              </>
            )}
          />
        )}
      </For>
      <Quarantine />
    </section>
  );
}
