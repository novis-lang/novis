// The flat stream at `/stream?<filter>`: every record of every kind, newest first, filtered by kind,
// level, route and trace, with the filter in the link. The list is the record feed (`feed.tsx`) and
// the form is `filter-form.tsx`; this file has the stream's columns. The log view (`log.tsx`) is the
// same feed with `kind=log` and its own columns.
//
// A row shows the time, the kind with its coloured dot, the text and the request. A log line also
// shows its level, and a line the service could not read or a count of dropped records shows a badge
// and its fields.
//
// Below the stream, a panel lists the files the service stopped reading, and has the action that
// discards the index.
//
// What it spends: what the record feed spends (`feed.tsx`), and the quarantine's list of files.

import { createResource, For, Show } from "solid-js";
import { discardIndex, quarantine, streamFilter, type StreamRecord } from "../api.ts";
import { query, reloadView } from "../router.ts";
import { clock, Feed, fieldsLine, flaw, KindDot, LevelChip, OwnerCell, when } from "./feed.tsx";
import { FilterForm } from "./filter-form.tsx";
import { Confirm, SaveButton } from "./saved.tsx";

// search.tsx and exceptions.tsx read the times the way the stream writes them.
export { clock, when };

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
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
      <button type="button" class="btn small danger discard" onClick={() => ask()}>
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
      <Quarantine />
    </section>
  );
}
