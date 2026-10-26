// One request's timeline: its calls, queries, spawns, garbage-collection pauses and outbound calls as
// bars from their start to their end, and its dumps and log lines as points, on one time axis from the
// request's start to its end.
//
// The chart places bars in lanes, so a call nested in another call sits under it and no two bars
// overlap: each bar, earliest first and the longer one first at one start, takes the first lane whose
// last bar has ended. Dumps and log lines share one lane of points below the bars.
//
// Under the chart, every item is one row of a listbox in time order (the WAI-ARIA listbox pattern, the
// way the request list is one). The keys move the selection, the chart highlights the selected item,
// and Enter on a dump or a log line moves to it on the page. A dump's site is a link to the editor
// (`site.tsx`), out of the tab order because the row is reached with the arrow keys.
//
// What it spends: one object per item and one lane number per bar, for lists the service already
// limited.

import { createMemo, createSignal, For, Show } from "solid-js";
import type { Dump, LogLine, RequestDetail, TraceEvent } from "../api.ts";
import { ignored } from "../keys.ts";
import { Site } from "../site.tsx";
import { duration } from "./requests.tsx";

type Kind = TraceEvent["kind"] | "dump" | "log";

/** One row of the timeline. */
interface Item {
  /** The option's DOM id. */
  id: string;
  kind: Kind;
  start: number;
  /** `null` for a point. */
  end: number | null;
  label: string;
  /** A short note after the label, such as a query's driver and rows. */
  detail: string;
  /** The trace event's id, for a bar. */
  event?: number;
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
  dump: "Dump",
  log: "Log line",
};

function eventItem(event: TraceEvent): Item {
  const query = event.query;
  let detail = "";
  if (query) detail = query.rows === null ? query.driver : `${query.driver}, ${query.rows === 1 ? "1 row" : `${query.rows} rows`}`;
  return {
    id: `event-${event.id}`,
    kind: event.kind,
    start: event.start_us,
    end: Math.max(event.start_us, event.end_us),
    label: query ? query.sql : event.label,
    detail,
    event: event.id,
  };
}

/**
 * Gives each event the lane its bar goes in, by its id. Events are taken earliest first, and at one
 * start the longer first, so an outer call is above the calls it makes. Each takes the first lane whose
 * last bar ended at or before its start.
 */
export function lanes(events: readonly Pick<TraceEvent, "id" | "start_us" | "end_us">[]): Map<number, number> {
  const order = [...events].sort((a, b) => a.start_us - b.start_us || b.end_us - a.end_us || a.id - b.id);
  const ends: number[] = [];
  const lane = new Map<number, number>();
  for (const event of order) {
    let at = ends.findIndex((end) => end <= event.start_us);
    if (at < 0) at = ends.push(0) - 1;
    ends[at] = Math.max(event.start_us, event.end_us);
    lane.set(event.id, at);
  }
  return lane;
}

/** Moves focus to the item a timeline row names on the page: a dump's tree or a log line. */
function reveal(item: Item) {
  const el = item.dump
    ? document.querySelector<HTMLElement>(`#dump-${item.dump.id} [role=treeitem][tabindex="0"]`)
    : item.log
      ? document.getElementById(`log-${item.log.id}`)
      : null;
  el?.focus();
}

export function Timeline(props: { detail: RequestDetail }) {
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
      ...d.events.map(eventItem),
      ...d.dumps.map((dump): Item => ({
        id: `point-dump-${dump.id}`,
        kind: "dump",
        start: dump.at_us,
        end: null,
        label: `${dump.file}:${dump.line}`,
        detail: "",
        dump,
      })),
      ...d.logs.map((log): Item => ({
        id: `point-log-${log.id}`,
        kind: "log",
        start: log.at_us,
        end: null,
        label: log.message,
        detail: log.level,
        log,
      })),
    ];
    return all.sort((a, b) => a.start - b.start || (b.end ?? b.start) - (a.end ?? a.start));
  });
  const laneOf = createMemo(() => lanes(props.detail.events));
  const laneCount = () => Math.max(0, ...laneOf().values()) + (props.detail.events.length > 0 ? 1 : 0);
  const [selected, setSelected] = createSignal(0);
  let list!: HTMLDivElement;

  const left = (us: number) => `${(Math.min(Math.max(us - t0(), 0), span()) / span()) * 100}%`;
  const width = (item: Item) => `${((Math.min((item.end ?? item.start) - t0(), span()) - Math.min(Math.max(item.start - t0(), 0), span())) / span()) * 100}%`;
  const ticks = () => [0, 0.25, 0.5, 0.75, 1].map((f) => ({ at: `${f * 100}%`, text: duration(Math.round(f * span())) }));

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

  const isOn = (item: Item) => items()[selected()]?.id === item.id;

  return (
    <section class="timeline" aria-labelledby="timeline-title">
      <h3 id="timeline-title">Timeline</h3>
      <For each={cutNotes()}>{(note) => <p class="note cut-note">{note}</p>}</For>
      <Show when={items().length > 0} fallback={<p class="note">This request recorded no events, dumps or log lines.</p>}>
        <div class="tl-chart" aria-hidden="true">
          <div class="tl-axis">
            <For each={ticks()}>
              {(tick) => (
                <span class="tl-tick" style={{ left: tick.at }}>
                  {tick.text}
                </span>
              )}
            </For>
          </div>
          <For each={Array.from({ length: laneCount() }, (_, i) => i)}>
            {(lane) => (
              <div class="tl-lane" data-lane={lane}>
                <For each={items().filter((item) => item.event !== undefined && laneOf().get(item.event) === lane)}>
                  {(item) => (
                    <div
                      class={`tl-bar tl-${item.kind}`}
                      classList={{ "tl-on": isOn(item) }}
                      style={{ left: left(item.start), width: width(item) }}
                      title={`${KIND_NAMES[item.kind]}: ${item.label}, ${duration(item.end! - item.start)}`}
                      onClick={() => move(items().indexOf(item))}
                    >
                      {item.label}
                    </div>
                  )}
                </For>
              </div>
            )}
          </For>
          <div class="tl-lane tl-points">
            <For each={items().filter((item) => item.end === null)}>
              {(item) => (
                <div
                  class={`tl-point tl-${item.kind}`}
                  classList={{ "tl-on": isOn(item) }}
                  style={{ left: left(item.start) }}
                  title={`${KIND_NAMES[item.kind]}: ${item.label}`}
                  onClick={() => move(items().indexOf(item))}
                />
              )}
            </For>
          </div>
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
            {(item, i) => (
              <div
                id={item.id}
                role="option"
                class={`tl-row tl-${item.kind}`}
                data-kind={item.kind}
                aria-selected={selected() === i() ? "true" : "false"}
                onClick={() => {
                  setSelected(i());
                  list.focus({ preventScroll: true });
                }}
                onDblClick={() => reveal(item)}
              >
                <span class="tl-kind">{KIND_NAMES[item.kind]}</span>
                <span class="tl-at num">{duration(Math.max(0, item.start - t0()))}</span>
                <span class="tl-label">
                  <Show when={item.dump} fallback={item.label}>
                    {(dump) => <Site site={dump()} tabindex={-1} />}
                  </Show>
                </span>
                <span class="tl-detail">{item.detail}</span>
                <span class="tl-duration num">{item.end === null ? "" : duration(item.end - item.start)}</span>
              </div>
            )}
          </For>
        </div>
      </Show>
    </section>
  );
}
