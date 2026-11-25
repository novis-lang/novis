// The flat stream at `/stream?<filter>`: every record of every kind, newest first, filtered by kind,
// level, route and trace, with the filter in the link. The list is the record feed (`feed.tsx`) and
// the form is `filter-form.tsx`; this file has the stream's columns. The log view (`log.tsx`) is the
// same feed with `kind=log` and its own columns.
//
// A row shows the time, the kind with its coloured dot, the text and the request. A log line also
// shows its level, and a line the service could not read or a count of dropped records shows a badge
// and its fields.
// The settings page lists what the recording lost, and has the action that discards the index.
//
// What it spends: what the record feed spends (`feed.tsx`).

import { For, Show } from "solid-js";
import { streamFilter, type StreamRecord } from "../api.ts";
import { query } from "../router.ts";
import { clock, Feed, fieldsLine, flaw, KindDot, LevelChip, OwnerCell, when } from "./feed.tsx";
import { FilterForm } from "./filter-form.tsx";
import { SaveButton } from "./saved.tsx";

// search.tsx, exceptions.tsx and settings.tsx read the times the way the stream writes them.
export { clock, when };

/** The stream's column titles: time, kind, text and request. */
const cols = () => (
  <>
    <span>Time</span>
    <span>Kind</span>
    <span>Text</span>
    <span>Request</span>
  </>
);

/** The cells of one stream row. The second cell's text is the kind's name or the badge alone. */
function StreamCells(props: { record: StreamRecord }) {
  const row = () => props.record;
  return (
    <>
      <span class="at">{clock(row().at_us)}</span>
      <span class="kind">
        <Show when={flaw(row())} fallback={<KindDot kind={row().kind} />}>
          {(badge) => <span class="badge">{badge()}</span>}
        </Show>
        <Show when={!flaw(row())}>{row().kind}</Show>
      </span>
      <span class="text" title={row().text}>
        <Show when={row().level}>{(level) => <LevelChip level={level()} />}</Show>
        <span class="text-body">{row().text}</span>
        <Show when={flaw(row()) && fieldsLine(row().fields)}>
          <span class="note"> {fieldsLine(row().fields)}</span>
        </Show>
      </span>
      <OwnerCell record={row()} />
    </>
  );
}

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
            empty={{
              icon: "stream",
              title: "No records match this filter",
              text: "New records show here when your program writes them.",
            }}
            grid="stream-grid"
            cols={cols()}
            cells={(row) => <StreamCells record={row} />}
          />
        )}
      </For>
    </section>
  );
}
