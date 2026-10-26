// Two requests side by side, at `/compare/<trace>/<span>/<trace>/<span>`. The original is on the left.
// Each side loads with its own `GET /api/request`, and every row that differs is marked.
// What it spends: the records of the two requests, for as long as the view is open.

import { createMemo, createResource, createSignal, For, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import { REDACTED, request, type Json, type RecordedBody, type RequestDetail, type TraceEvent, type RequestRow } from "../api.ts";
import { ignored } from "../keys.ts";
import { compareLink, navigate, requestLink, type SpanKey } from "../router.ts";
import { duration, statusClass } from "./requests.tsx";
import { ValueTree } from "./value-tree.tsx";

/** A request marked in a list, to compare with the next request opened. */
export interface Marked extends SpanKey {
  method: string;
  path: string;
}

const [marked, setMarked] = createSignal<Marked | null>(null);

/** The marked request, or `null`. It lasts until a second request is opened or the mark is cleared. */
export { marked, setMarked };

/** Marks `row`, or clears the mark when `row` is the one marked. A row without a span id cannot be marked. */
export function toggleMark(row: RequestRow): void {
  const m = marked();
  if (m && m.trace === row.trace && m.span === row.span) setMarked(null);
  else if (row.span) setMarked({ trace: row.trace, span: row.span, method: row.method, path: row.path });
}

/** True when `row` is the marked request. */
export function isMarked(row: Pick<RequestRow, "trace" | "span">): boolean {
  const m = marked();
  return !!m && m.trace === row.trace && m.span === row.span;
}

/** Where a link to `row` goes: the compare view when another request is marked, and the request's own
 *  page otherwise. The compare view clears the mark when it opens. */
export function markLink(row: Pick<RequestRow, "trace" | "span">): string {
  const m = marked();
  if (m && row.span && !isMarked(row)) return compareLink(m, { trace: row.trace, span: row.span });
  return requestLink(row);
}

/** Where opening `row` goes, as `markLink`, and clears the mark when that is the compare view. */
export function openLink(row: RequestRow): string {
  const to = markLink(row);
  if (to !== requestLink(row)) setMarked(null);
  return to;
}

/** The marked request's method and path, with a button that clears the mark. Shown while one is marked. */
export function MarkState() {
  return (
    <Show when={marked()}>
      {(m) => (
        <span class="mark-state" role="status">
          Marked: {m().method} {m().path}. Open another request to compare the two.{" "}
          <button type="button" class="btn small" onClick={() => setMarked(null)}>
            Clear
          </button>
        </span>
      )}
    </Show>
  );
}

/** One side of a row: its text, and how it shows. */
interface Cell {
  text: string;
  /** A note about the value, such as "Not sent", shown muted. */
  note?: boolean;
  redacted?: boolean;
  /** Code or data, shown in the monospace font. */
  code?: boolean;
  /** A dump's value, shown as a tree under `text`. */
  tree?: Json;
}

interface Row {
  label: string;
  left: Cell;
  right: Cell;
  differs: boolean;
}

interface Part {
  id: string;
  title: string;
  /** The head of the label column. */
  head: string;
  rows: Row[];
  /** What the part says when neither request has a row. */
  empty: string;
}

const note = (text: string): Cell => ({ text, note: true });
const code = (text: string): Cell => ({ text, code: true });
/** True when two cells show different things. */
const same = (a: Cell, b: Cell) => a.text === b.text && !!a.note === !!b.note;
const row = (label: string, left: Cell, right: Cell): Row => ({ label, left, right, differs: !same(left, right) });

const NOT_RECORDED = "Not recorded.";

/** The rows of two header lists, by name in the order they first appear. A name sent twice shows each
 *  value on its own line. A header recorded as `REDACTED` on either side shows as redacted on both and
 *  is never a difference, since the hidden value cannot be compared. */
function headerRows(left: [string, string][] | null, right: [string, string][] | null): Row[] {
  const names = new Map<string, string>();
  const group = (list: [string, string][] | null) => {
    const by = new Map<string, string[]>();
    for (const [name, value] of list ?? []) {
      const key = name.toLowerCase();
      if (!names.has(key)) names.set(key, name);
      by.set(key, [...(by.get(key) ?? []), value]);
    }
    return by;
  };
  const l = group(left);
  const r = group(right);
  return [...names].map(([key, name]) => {
    const lv = l.get(key);
    const rv = r.get(key);
    if (lv?.includes(REDACTED) || rv?.includes(REDACTED)) {
      const hidden: Cell = { text: "Redacted", redacted: true };
      return { label: name, left: hidden, right: hidden, differs: false };
    }
    const cell = (list: [string, string][] | null, values: string[] | undefined) =>
      list === null ? note(NOT_RECORDED) : values ? code(values.join("\n")) : note("Not sent");
    return row(name, cell(left, lv), cell(right, rv));
  });
}

/** A body as the view compares it: a note when its bytes cannot be compared as text, its JSON value
 *  when its text parses, and its text otherwise. `raw` lets two notes about different bytes differ. */
type Body = { note: string; raw: string } | { json: Json } | { text: string };

function body(recorded: RecordedBody | null | undefined): Body {
  if (recorded === undefined) return { note: "Not recorded. A server records this in development only.", raw: "" };
  if (recorded === null) return { note: NOT_RECORDED, raw: "" };
  if (recorded.withheld) return { note: "Not recorded, because it contains a secret.", raw: "" };
  if (!recorded.read) return { note: "The program did not read this body.", raw: "" };
  if (recorded.cut) return { note: "Too long to record in full, so it is not compared.", raw: recorded.text ?? recorded.base64 ?? "" };
  if (recorded.base64 !== null) return { note: "Binary data, so it is not compared as text.", raw: recorded.base64 };
  const text = recorded.text ?? "";
  if (text === "") return { note: "Empty.", raw: "" };
  try {
    return { json: JSON.parse(text) as Json };
  } catch {
    return { text };
  }
}

/** Every leaf of `value` by its path, such as `items[0].name`, with its JSON text. */
function flatten(value: Json, path: string, out: Map<string, string>): Map<string, string> {
  if (Array.isArray(value) && value.length > 0) value.forEach((item, i) => flatten(item, `${path}[${i}]`, out));
  else if (value !== null && typeof value === "object" && !Array.isArray(value) && Object.keys(value).length > 0) {
    for (const [key, item] of Object.entries(value)) flatten(item, path ? `${path}.${key}` : key, out);
  } else out.set(path || "Value", JSON.stringify(value));
  return out;
}

/** The rows of two bodies: one per JSON leaf when both are JSON, so a reordered object is not a
 *  difference, and one row otherwise. A body that was withheld, cut, not read or is binary is a note. */
function bodyRows(left: Body, right: Body): Row[] {
  if ("json" in left && "json" in right) {
    const l = flatten(left.json, "", new Map());
    const r = flatten(right.json, "", new Map());
    const paths = [...l.keys(), ...[...r.keys()].filter((path) => !l.has(path))];
    const cell = (text: string | undefined) => (text === undefined ? note("Missing") : code(text));
    return paths.map((path) => row(path, cell(l.get(path)), cell(r.get(path))));
  }
  const cell = (b: Body): Cell => ("note" in b ? note(b.note) : "json" in b ? code(JSON.stringify(b.json, null, 2)) : code(b.text));
  const key = (b: Body) => ("note" in b ? `note ${b.note} ${b.raw}` : "json" in b ? `json ${JSON.stringify(b.json)}` : `text ${b.text}`);
  return [{ label: "Body", left: cell(left), right: cell(right), differs: key(left) !== key(right) }];
}

/** The rows of two lists aligned by position: the first with the first, and so on. */
function aligned<T>(left: T[], right: T[], show: (item: T) => Cell, compare: (item: T) => string, none: string): Row[] {
  return Array.from({ length: Math.max(left.length, right.length) }, (_, i) => {
    const l = left[i];
    const r = right[i];
    const differs = l === undefined || r === undefined || compare(l) !== compare(r);
    return { label: String(i + 1), left: l === undefined ? note(none) : show(l), right: r === undefined ? note(none) : show(r), differs };
  });
}

/** What one call site ran in one request: each statement once, with how many times it ran, in the
 *  order it first ran. */
function ranAt(events: TraceEvent[]): Cell {
  const counts = new Map<string, number>();
  for (const event of events) {
    const sql = event.query?.sql ?? event.label;
    counts.set(sql, (counts.get(sql) ?? 0) + 1);
  }
  return code([...counts].map(([sql, n]) => (n === 1 ? sql : `${sql} (${n} times)`)).join("; "));
}

/** The queries of two requests aligned by the call that sent them: one row per call site, in the
 *  order the left request and then the right one first reached it. A query with no recorded site
 *  is in the row "No call site". */
function bySite(left: TraceEvent[], right: TraceEvent[]): Row[] {
  const label = (event: TraceEvent) => (event.query?.site ? `${event.query.site.file}:${event.query.site.line}` : "No call site");
  const sites: string[] = [];
  const group = (events: TraceEvent[]) => {
    const at = new Map<string, TraceEvent[]>();
    for (const event of events) {
      const site = label(event);
      if (!sites.includes(site)) sites.push(site);
      const ran = at.get(site);
      if (ran) ran.push(event);
      else at.set(site, [event]);
    }
    return at;
  };
  const l = group(left);
  const r = group(right);
  return sites.map((site) => {
    const a = l.get(site);
    const b = r.get(site);
    return row(site, a ? ranAt(a) : note("Not reached"), b ? ranAt(b) : note("Not reached"));
  });
}

/** Every part of the comparison of `l` and `r`, in the order the view shows them. */
function parts(l: RequestDetail, r: RequestDetail): Part[] {
  const query = (d: RequestDetail) => (d.input === null ? note(NOT_RECORDED) : d.input.query ? code(d.input.query) : note("None"));
  const queries = (d: RequestDetail) => d.events.filter((event) => event.kind === "query");
  const dump = (d: RequestDetail["dumps"][number]): Cell =>
    d.elided ? { text: `${d.file}:${d.line}. This value was too long to record.`, code: true } : { text: `${d.file}:${d.line}`, code: true, tree: d.tree };
  const log = (line: RequestDetail["logs"][number]): Cell => code(`${line.level}: ${line.message}`);
  return [
    {
      id: "request",
      title: "Request",
      head: "Field",
      rows: [
        row("Method", code(l.request.method), code(r.request.method)),
        row("Path", code(l.request.path), code(r.request.path)),
        row("Query", query(l), query(r)),
        row("Duration", { text: duration(l.request.duration_us) }, { text: duration(r.request.duration_us) }),
      ],
      empty: "",
    },
    {
      id: "request-headers",
      title: "Request headers",
      head: "Header",
      rows: headerRows(l.input?.headers ?? null, r.input?.headers ?? null),
      empty: "No request headers were recorded.",
    },
    { id: "request-body", title: "Request body", head: "Key", rows: bodyRows(body(l.input?.body), body(r.input?.body)), empty: "" },
    {
      id: "response",
      title: "Response",
      head: "Field",
      rows: [row("Status", code(String(l.request.status)), code(String(r.request.status)))],
      empty: "",
    },
    {
      id: "response-headers",
      title: "Response headers",
      head: "Header",
      rows: headerRows(l.input?.response_headers ?? null, r.input?.response_headers ?? null),
      empty: "No response headers were recorded.",
    },
    {
      id: "response-body",
      title: "Response body",
      head: "Key",
      rows: bodyRows(body(l.input ? l.input.response_body : undefined), body(r.input ? r.input.response_body : undefined)),
      empty: "",
    },
    {
      id: "queries",
      title: "Queries",
      head: "Call",
      rows: bySite(queries(l), queries(r)),
      empty: "Neither request ran a query.",
    },
    {
      id: "dumps",
      title: "Dumps",
      head: "Order",
      rows: aligned(l.dumps, r.dumps, dump, (d) => `${d.file}:${d.line} ${d.elided} ${JSON.stringify(d.tree)}`, "No dump"),
      empty: "Neither request dumped a value.",
    },
    {
      id: "logs",
      title: "Log lines",
      head: "Order",
      rows: aligned(l.logs, r.logs, log, (line) => `${line.level} ${line.message} ${JSON.stringify(line.fields)}`, "No log line"),
      empty: "Neither request wrote a log line.",
    },
  ];
}

/** The view: the recorded input, the response, the duration, the queries, the dumps and the log lines of
 *  `left` and `right`, one table per part. A list marks a request (`marked`), and opening a second one
 *  opens this view (`openLink`, `markLink`). */
export function CompareView(props: { left: SpanKey; right: SpanKey }) {
  const [left] = createResource(
    () => props.left,
    (key) => request(key),
  );
  const [right] = createResource(
    () => props.right,
    (key) => request(key),
  );
  // A resource that failed throws when it is read, so its error is checked first.
  const both = () => (left.error || right.error || !left() || !right() ? null : { left: left()!, right: right()! });

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e) || e.key !== "Escape") return;
    e.preventDefault();
    navigate("/");
  };
  onMount(() => {
    // A link from a list opens this view while the left request is still marked.
    if (isMarked(props.left)) setMarked(null);
    document.addEventListener("keydown", onKey);
  });
  onCleanup(() => document.removeEventListener("keydown", onKey));

  return (
    <section class="request compare" aria-label="Compare two requests">
      <a href="/" class="back">
        Back to the request list
      </a>
      <h2 class="title">Compare two requests</h2>
      <Switch>
        <Match when={left.error || right.error}>
          <Show when={left.error}>
            <p class="note bad" role="alert">
              The original request could not load: {String(left.error?.message ?? left.error)}
            </p>
          </Show>
          <Show when={right.error}>
            <p class="note bad" role="alert">
              The other request could not load: {String(right.error?.message ?? right.error)}
            </p>
          </Show>
        </Match>
        <Match when={left.loading || right.loading}>
          <p class="note">Loading.</p>
        </Match>
        <Match when={both()}>{(b) => <Comparison left={b().left} right={b().right} />}</Match>
      </Switch>
    </section>
  );
}

function Comparison(props: { left: RequestDetail; right: RequestDetail }) {
  const all = createMemo(() => parts(props.left, props.right));
  const count = () => all().reduce((n, part) => n + part.rows.filter((r) => r.differs).length, 0);
  const side = (d: RequestDetail) => (
    <>
      <a href={requestLink(d.request)}>
        {d.request.method} {d.request.path}
      </a>{" "}
      <span class={statusClass(d.request.status)}>{d.request.status}</span>{" "}
      <span class="note">started {new Date(d.request.start_us / 1000).toLocaleString()}</span>
    </>
  );
  return (
    <>
      <dl class="fields">
        <dt>Original</dt>
        <dd class="cmp-left">{side(props.left)}</dd>
        <dt>Other</dt>
        <dd class="cmp-right">{side(props.right)}</dd>
      </dl>
      <Show when={props.left.input === null}>
        <p class="note cmp-unrecorded">The input of the original request was not recorded. A server records it in development only.</p>
      </Show>
      <Show when={props.right.input === null}>
        <p class="note cmp-unrecorded">The input of the other request was not recorded. A server records it in development only.</p>
      </Show>
      <p class="cmp-count" role="status">
        {count() === 0 ? "The two requests are the same." : count() === 1 ? "1 row differs." : `${count()} rows differ.`}
      </p>
      <For each={all()}>
        {(part) => (
          <section class="cmp-part" id={`cmp-${part.id}`} aria-labelledby={`cmp-${part.id}-title`}>
            <h3 id={`cmp-${part.id}-title`}>{part.title}</h3>
            <Show when={part.rows.length > 0} fallback={<p class="note">{part.empty}</p>}>
              <table class="cmp">
                <thead>
                  <tr>
                    <th scope="col">{part.head}</th>
                    <th scope="col">Original</th>
                    <th scope="col">Other</th>
                  </tr>
                </thead>
                <tbody>
                  <For each={part.rows}>
                    {(r, i) => (
                      <tr classList={{ "cmp-differs": r.differs }}>
                        <th scope="row">
                          {r.label}
                          <Show when={r.differs}>
                            {" "}
                            <span class="badge cmp-badge">Differs</span>
                          </Show>
                        </th>
                        <td>
                          <CellView cell={r.left} prefix={`cmp-${part.id}-${i()}-l`} />
                        </td>
                        <td>
                          <CellView cell={r.right} prefix={`cmp-${part.id}-${i()}-r`} />
                        </td>
                      </tr>
                    )}
                  </For>
                </tbody>
              </table>
            </Show>
          </section>
        )}
      </For>
    </>
  );
}

function CellView(props: { cell: Cell; prefix: string }) {
  const c = () => props.cell;
  return (
    <>
      <span classList={{ note: !!c().note, "cmp-redacted": !!c().redacted, "cmp-code": !!c().code }}>{c().text}</span>
      <Show when={c().tree !== undefined}>
        <ValueTree value={c().tree!} prefix={props.prefix} name={`Value dumped at ${c().text}`} />
      </Show>
    </>
  );
}
