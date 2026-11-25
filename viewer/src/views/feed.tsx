// The live list of stream records the stream (`stream.tsx`) and the log (`log.tsx`) draw: one filter,
// newest first, with the selected record's fields below the list. Loading, following and pausing are
// `createFeed`, and the list is `List` (`ui/list.tsx`); this file has the record's own parts: its
// cursor, its kind dot, its level chip, the request it belongs to and its detail.
//
// A row that is a line the service could not read, or a count of records it dropped, is shown with a
// badge and its fields. A click selects a row, and `Enter` opens the selected record's request.
//
// What it spends: what `createFeed` keeps, at most `KEEP` records while following plus what arrived
// while paused. Each record is a few hundred bytes and its fields.

import { createSignal, Show, type JSX } from "solid-js";
import { stream, type Json, type StreamFilter, type StreamRecord } from "../api.ts";
import { navigate, requestLink } from "../router.ts";
import { Method, Status } from "../ui/badges.tsx";
import { Empty } from "../ui/empty.tsx";
import type { IconName } from "../ui/icon.tsx";
import { createFeed, List, type RowKey } from "../ui/list.tsx";
import { Site } from "../ui/site.tsx";
import { ValueTree } from "./value-tree.tsx";

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

/** The badge of a row that is not a record of the program. */
export function flaw(record: StreamRecord): string | null {
  if (record.kind === "unparseable") return "Unreadable line";
  if (record.kind === "dropped") return "Records dropped";
  return null;
}

/** The `before` cursor of `record`: its time and its key. */
const cursor = (record: StreamRecord) => `${record.at_us}.${record.key}`;

/** A dot in the colour of a record's kind. It is decoration: the kind's name is always beside it. */
export function KindDot(props: { kind: string }) {
  return <span class={`kind-dot rk-${props.kind}`} aria-hidden="true" />;
}

/** The tone of a log level: `error` and `critical` red, `warning` orange, `info` blue and the rest
 *  grey. */
function levelTone(level: string): string {
  if (level === "error" || level === "critical") return "bad";
  if (level === "warning" || level === "warn") return "warn";
  if (level === "info") return "info";
  return "debug";
}

/** A log line's level as a small chip in the level's colour. */
export function LevelChip(props: { level: string }) {
  return <span class={`level-chip lv-${levelTone(props.level)}`}>{props.level}</span>;
}

/** The request a record belongs to: method, path and status, linked to the request's page. */
export function OwnerCell(props: { record: StreamRecord }) {
  return (
    <span class="owner">
      <Show when={props.record.request} fallback={<span class="note">No request</span>}>
        {(owner) => (
          <a href={requestLink(owner())} tabindex="-1">
            <Method value={owner().method} /> <span class="owner-path">{owner().path}</span> <Status code={owner().status} />
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
  /** The title and the text that show when the filter keeps nothing. */
  empty: { icon: IconName; title: string; text: string };
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
  const feed = createFeed<StreamRecord, string>({
    load: async (at, signal) => {
      const page = await stream(filter, at, signal);
      return { rows: page.records, older: page.older, watermark: page.watermark };
    },
    key: (record) => record.key,
    cursor,
  });
  const [selected, setSelected] = createSignal<RowKey | null>(null);
  const chosen = () => {
    const key = selected();
    return key === null ? undefined : feed.rows().find((record) => record.key === key);
  };

  return (
    <div class="feed">
      <List
        rows={feed.rows()}
        key={(record) => record.key}
        idPrefix="record"
        label={props.label}
        selected={selected()}
        onSelect={setSelected}
        onOpen={(record) => {
          if (record.request) navigate(requestLink(record.request));
        }}
        grid={props.grid}
        rowClass={(record) => ({ flawed: flaw(record) !== null })}
        rowKind={(record) => record.kind}
        describe={(record) => flaw(record) ?? undefined}
        cols={props.cols}
        feed={feed}
        noun={props.noun}
        empty={
          <Empty icon={props.empty.icon} title={props.empty.title}>
            {props.empty.text}
          </Empty>
        }
      >
        {props.cells}
      </List>
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
        <dd class="detail-kind">
          <KindDot kind={r().kind} />
          {r().kind}
        </dd>
        <dt>Time</dt>
        <dd class="detail-time">{when(r().at_us)}</dd>
        <dt>Text</dt>
        <dd class="detail-text">{r().text}</dd>
        <Show when={r().level}>
          {(level) => (
            <>
              <dt>Level</dt>
              <dd>
                <LevelChip level={level()} />
              </dd>
            </>
          )}
        </Show>
        <Show when={r().file}>
          <dt>File</dt>
          <dd class="site">
            {/* A dump's file is a source site. An unreadable line's file is a record file, so it stays text. */}
            <Show
              when={r().kind === "dump" && r().line !== null ? { file: r().file!, line: r().line! } : null}
              fallback={
                <>
                  {r().file}
                  {r().line !== null ? `:${r().line}` : ""}
                </>
              }
            >
              {(site) => <Site site={site()} />}
            </Show>
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
                <Status code={owner().status} />
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
