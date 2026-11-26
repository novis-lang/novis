// One request's timeline as a waterfall: one row per item on one time axis from the request's start to
// its end. A call, a query, a spawn, a garbage-collection pause, an outbound call, a cache, session,
// file, mail or queue event is a bar from its start to its end in its row's lane, in its kind's tone (`.k-<kind>`); a
// dump or a log line is a point. Above the rows, an axis row has five ticks at each quarter of the span.
//
// A repeated statement is one row: query events with the same SQL text and the same non-null call site
// that ran two or more times share a row at the first one's time, with the gold wash, the `Repeat`
// count, every member's bar in its lane and the members' summed duration. A query with no site is never
// grouped, because two different callers could send the same text.
//
// The rows are the listbox (the WAI-ARIA listbox pattern, the way the request list is one). The keys
// move the selection, the selected row's bars are `tl-on`, and Enter or a double click on a dump or a
// log line calls `reveal`, which opens that record's tab. A dump's site is a link to the editor
// (`ui/site.tsx`), out of the tab order because the row is reached with the arrow keys. Each row keeps
// text cells for its kind, its start, its label, its detail and its duration; the kind and the start are
// for a screen reader, and the swatch and the bar's place show them on screen. An event's detail is
// its outcome (`request/outcome.ts`), and a cache `get` has a `Hit` or `Miss` chip.
//
// What it spends: one object per row and one per bar, and one map entry per query site while grouping,
// for lists the service already limited.

import { createMemo, createSignal, For, Show } from "solid-js";
import type { Dump, LogLine, RequestDetail, TraceEvent } from "../api.ts";
import { ignored } from "../keys.ts";
import { Hit, Repeat } from "../ui/badges.tsx";
import { Site } from "../ui/site.tsx";
import { outcomeText } from "./request/outcome.ts";
import { duration } from "./requests.tsx";

type Kind = TraceEvent["kind"] | "dump" | "log";

/** One bar in a row's lane, in microseconds on the request's clock. */
interface Bar {
  start: number;
  end: number;
}

/** One row of the timeline. */
interface Item {
  /** The option's DOM id. */
  id: string;
  kind: Kind;
  start: number;
  /** The last bar's end, or `null` for a point. */
  end: number | null;
  /** The bars in the lane: one for an event, one per member for a repeated statement, none for a point. */
  bars: Bar[];
  /** The time the row's bars sum to. */
  total: number;
  label: string;
  /** A short note after the label, such as a query's driver and rows. */
  detail: string;
  /** How many events the row is: above 1 for a repeated statement. */
  count: number;
  /** A cache `get`'s result: `true` for a hit, `false` for a miss. */
  hit?: boolean | undefined;
  dump?: Dump;
  log?: LogLine;
}

/** What a row calls each kind of item. */
const KIND_NAMES: Record<Kind, string> = {
  call: "Call",
  query: "Query",
  spawn: "Spawn",
  gc: "Garbage collection",
  http: "Outbound call",
  cache: "Cache",
  session: "Session",
  file: "File",
  mail: "Mail",
  queue: "Queue job",
  dump: "Dump",
  log: "Log line",
};

const rowsText = (rows: number) => (rows === 1 ? "1 row" : `${rows} rows`);

function bar(event: TraceEvent): Bar {
  return { start: event.start_us, end: Math.max(event.start_us, event.end_us) };
}

function eventItem(event: TraceEvent): Item {
  const query = event.query;
  const b = bar(event);
  return {
    id: `event-${event.id}`,
    kind: event.kind,
    start: b.start,
    end: b.end,
    bars: [b],
    total: b.end - b.start,
    label: query ? query.sql : event.target ? `${event.label} ${event.target}` : event.label,
    detail: query ? (query.rows === null ? query.driver : `${query.driver}, ${rowsText(query.rows)}`) : outcomeText(event),
    count: 1,
    hit: event.outcome?.hit,
  };
}

/** The row of a statement that ran `members.length` times at one site, at the first one's time. */
function groupItem(members: TraceEvent[]): Item {
  const first = members[0]!;
  const bars = members.map(bar);
  return {
    id: `group-${first.id}`,
    kind: "query",
    start: bars[0]!.start,
    end: Math.max(...bars.map((b) => b.end)),
    bars,
    total: bars.reduce((sum, b) => sum + (b.end - b.start), 0),
    label: first.query!.sql,
    detail: first.query!.driver,
    count: members.length,
  };
}

/** The key that groups a query: its SQL and its call site, or `null` when it has no site. */
function groupKey(event: TraceEvent): string | null {
  const q = event.query;
  if (event.kind !== "query" || !q || !q.site) return null;
  return `${q.site.file}:${q.site.line}\n${q.sql}`;
}

/** The event rows: each event alone, and each statement that ran twice or more at one site as one row. */
function eventItems(events: readonly TraceEvent[]): Item[] {
  const order = [...events].sort((a, b) => a.start_us - b.start_us || a.id - b.id);
  const groups = new Map<string, TraceEvent[]>();
  for (const event of order) {
    const key = groupKey(event);
    if (key === null) continue;
    const members = groups.get(key);
    if (members) members.push(event);
    else groups.set(key, [event]);
  }
  const out: Item[] = [];
  for (const event of order) {
    const key = groupKey(event);
    const members = key === null ? undefined : groups.get(key);
    if (!members || members.length < 2) out.push(eventItem(event));
    else if (members[0] === event) out.push(groupItem(members));
  }
  return out;
}

export function Timeline(props: { detail: RequestDetail; reveal: (kind: "dump" | "log", id: number) => void }) {
  /** Opens the tab of the dump or the log line a row names, at that record. */
  const reveal = (item: Item) => {
    if (item.dump) props.reveal("dump", item.dump.id);
    else if (item.log) props.reveal("log", item.log.id);
  };
  const t0 = () => props.detail.request.start_us;
  /** The axis's length in microseconds: the request's duration, or longer when an event ends later. */
  const span = createMemo(() => {
    const ends = props.detail.events.map((e) => e.end_us);
    const points = [...props.detail.dumps, ...props.detail.logs].map((p) => p.at_us);
    return Math.max(1, props.detail.request.duration_us, ...[...ends, ...points].map((end) => end - t0()));
  });
  const items = createMemo(() => {
    const d = props.detail;
    const all: Item[] = [
      ...eventItems(d.events),
      ...d.dumps.map((dump): Item => ({
        id: `point-dump-${dump.id}`,
        kind: "dump",
        start: dump.at_us,
        end: null,
        bars: [],
        total: 0,
        label: `${dump.file}:${dump.line}`,
        detail: "",
        count: 1,
        dump,
      })),
      ...d.logs.map((log): Item => ({
        id: `point-log-${log.id}`,
        kind: "log",
        start: log.at_us,
        end: null,
        bars: [],
        total: 0,
        label: log.message,
        detail: log.level,
        count: 1,
        log,
      })),
    ];
    return all.sort((a, b) => a.start - b.start || (b.end ?? b.start) - (a.end ?? a.start));
  });
  const [selected, setSelected] = createSignal(0);
  let list!: HTMLDivElement;

  /** Where `us` is on the axis, clamped to it, from 0 to 1. */
  const at = (us: number) => Math.min(Math.max(us - t0(), 0), span()) / span();
  const left = (us: number) => `${at(us) * 100}%`;
  const width = (b: Bar) => `${(at(b.end) - at(b.start)) * 100}%`;
  const ticks = () => [0, 0.25, 0.5, 0.75, 1].map((f) => duration(Math.round(f * span())));

  const move = (to: number) => {
    const n = items().length;
    if (n === 0) return;
    const i = Math.min(n - 1, Math.max(0, to));
    setSelected(i);
    document.getElementById(items()[i]!.id)?.scrollIntoView({ block: "nearest" });
  };

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e)) return;
    switch (e.key) {
      case "j":
      case "ArrowDown":
        move(selected() + 1);
        break;
      case "k":
      case "ArrowUp":
        move(selected() - 1);
        break;
      case "Home":
        move(0);
        break;
      case "End":
        move(items().length - 1);
        break;
      case "Enter": {
        const item = items()[selected()];
        if (!item?.dump && !item?.log) return;
        reveal(item);
        break;
      }
      default:
        return;
    }
    e.preventDefault();
  };

  const cutNotes = () => {
    const cut = props.detail.cut;
    const notes: string[] = [];
    if (cut.events) notes.push("The list of events stopped at its limit, so some events are not shown.");
    if (cut.dumps) notes.push("The list of dumps stopped at its limit, so some dumps are not shown.");
    if (cut.logs) notes.push("The list of log lines stopped at its limit, so some log lines are not shown.");
    return notes;
  };

  return (
    <section class="timeline" aria-label="Timeline">
      <For each={cutNotes()}>{(note) => <p class="note cut-note">{note}</p>}</For>
      <Show when={items().length > 0} fallback={<p class="note">This request recorded no events, dumps or log lines.</p>}>
        <div class="tl-axis" aria-hidden="true">
          <span />
          <div class="tl-ticks">
            <For each={ticks()}>{(tick) => <span class="tl-tick">{tick}</span>}</For>
          </div>
          <span />
        </div>
        <div
          ref={list}
          class="tl-list"
          role="listbox"
          tabindex="0"
          aria-label="Everything the request did, in time order"
          aria-activedescendant={items()[selected()]?.id}
          onKeyDown={onKey}
        >
          <For each={items()}>
            {(item, i) => {
              const on = () => selected() === i();
              return (
                <div
                  id={item.id}
                  role="option"
                  class={`tl-row tl-${item.kind}`}
                  classList={{ "tl-grp": item.count > 1 }}
                  data-kind={item.kind}
                  aria-selected={on() ? "true" : "false"}
                  onClick={() => {
                    setSelected(i());
                    list.focus({ preventScroll: true });
                  }}
                  onDblClick={() => reveal(item)}
                >
                  <div class="tl-lab">
                    <i class={`tl-swatch k-${item.kind}`} aria-hidden="true" />
                    <span class="tl-kind sr-only">{KIND_NAMES[item.kind]}</span>
                    <span class="tl-at sr-only">{duration(Math.max(0, item.start - t0()))}</span>
                    <span class="tl-label">
                      <Show when={item.dump} fallback={item.label}>
                        {(dump) => <Site site={dump()} tabindex={-1} />}
                      </Show>
                    </span>
                    <Show when={item.count > 1}>
                      <Repeat count={item.count} />
                    </Show>
                    <Show when={item.hit !== undefined}>
                      <Hit hit={item.hit!} />
                    </Show>
                    <span class="tl-detail">{item.detail}</span>
                  </div>
                  <div class="tl-lane" aria-hidden="true">
                    <Show
                      when={item.end !== null}
                      fallback={
                        <i class={`tl-point tl-${item.kind}`} classList={{ "tl-on": on() }} style={{ left: left(item.start) }} title={`${KIND_NAMES[item.kind]}: ${item.label}`} />
                      }
                    >
                      <For each={item.bars}>
                        {(b) => (
                          <i
                            class={`tl-bar k-${item.kind}`}
                            classList={{ "tl-on": on() }}
                            style={{ left: left(b.start), width: width(b) }}
                            title={`${KIND_NAMES[item.kind]}: ${item.label}, ${duration(b.end - b.start)}`}
                          />
                        )}
                      </For>
                    </Show>
                  </div>
                  <span class="tl-duration num">{item.end === null ? "" : duration(item.total)}</span>
                </div>
              );
            }}
          </For>
        </div>
      </Show>
    </section>
  );
}
