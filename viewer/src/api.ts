// The JSON API `nvs debug` serves under `/api/`. The Rust side owns these shapes
// (`crates/nvs-cli/src/debug/api.rs`, `stream.rs`, `search.rs`, `exceptions.rs` and `settings.rs`):
// `crates/nvs-cli/tests/debug_api.rs` writes the files under `test/fixtures/` from a real index, and
// `test/api.test.ts` makes `tsc` check those fixtures against the types here.

/** The exception a request did not catch: its class and the site of its `throw`. */
export interface Threw {
  class: string;
  file: string;
  line: number;
}

/** One request in the list. `id` rises with every request the index adds. */
export interface RequestRow {
  id: number;
  batch: number;
  trace: string;
  /** The server's span id for the request. `null` on a record an older build wrote. */
  span: string | null;
  start_us: number;
  duration_us: number;
  method: string;
  route: string | null;
  path: string;
  status: number;
  queries: number;
  dumps: number;
  logs: number;
  exception: Threw | null;
}

/** One page of `GET /api/requests`, newest first. */
export interface RequestPage {
  /** The last committed batch this page is complete up to. */
  watermark: number;
  requests: RequestRow[];
  /** The `before` of the next older page, or `null` when this page has the oldest request. */
  older: number | null;
}

/** What `GET /api/requests` filters on. Every key is optional. */
export interface RequestFilter {
  /** Only requests older than this id. */
  before?: number;
  /** Only requests committed after this batch. */
  since?: number;
  /** Only requests in this trace: 32 lower-case hex digits. */
  trace?: string;
  /** Only the request with this span id: 16 lower-case hex digits. */
  span?: string;
  /** At most this many rows, from 1 to 500. The service gives 100 when it is not set. */
  limit?: number;
}

/** Any JSON value. A dump's tree and a log line's fields are in the JSON rendering of a value. */
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

/** An uncaught exception: its class, its message and the site of its `throw`. */
export interface Exception extends Threw {
  message: string;
}

/** A `query` event's driver, SQL, rows and call site. */
export interface EventQuery {
  driver: string;
  sql: string;
  rows: number | null;
  site: number | null;
}

/** One trace event of a request. Its times are Unix microseconds. */
export interface TraceEvent {
  id: number;
  kind: "call" | "query" | "spawn" | "gc" | "http";
  label: string;
  start_us: number;
  end_us: number;
  /** A call's checked-return status. */
  status: number | null;
  query: EventQuery | null;
}

/** One `Core\Debug::dump`: its site and its value tree. */
export interface Dump {
  id: number;
  at_us: number;
  file: string;
  line: number;
  /** `null` when `elided` is set: the tree was too long to record. */
  tree: Json;
  elided: boolean;
}

/** One log line a request wrote. */
export interface LogLine {
  id: number;
  at_us: number;
  level: string;
  message: string;
  fields: Json;
  elided: boolean;
  lossy: boolean;
}

/** A profile file the request's `Core\Debug::startProfile` wrote. */
export interface ProfileFile {
  id: number;
  at_us: number;
  /** The absolute path on the machine that ran the request. */
  path: string;
}

/** That a request's server counted the lines it ran. `GET /api/coverage?id=` gives the counts. */
export interface CoverageRef {
  id: number;
  /** How many files it ran lines of. */
  files: number;
  /** The record was too long to keep, so it lists no line. */
  elided: boolean;
}

/** `GET /api/coverage`: the lines one request ran. */
export interface CoverageMap {
  id: number;
  trace: string;
  span: string | null;
  start_us: number;
  /** File path to line number to the times it ran. A line that did not run is absent. */
  files: Record<string, Record<string, number>>;
  elided: boolean;
}

/** `GET /api/source`: the text of one file a coverage record names. */
export interface SourceFile {
  file: string;
  text: string;
  /** The file was saved after the request started, so its lines may not match the counts. */
  changed: boolean;
}

/** `GET /api/request`: one request and everything it recorded, each list in time order. */
export interface RequestDetail {
  watermark: number;
  request: RequestRow;
  exception: Exception | null;
  events: TraceEvent[];
  dumps: Dump[];
  logs: LogLine[];
  profiles: ProfileFile[];
  coverage: CoverageRef | null;
  /** What it was sent and what it answered, when a development server recorded them. */
  input: RecordedInput | null;
  /** Which lists stopped at the most rows the service sends. */
  cut: { events: boolean; dumps: boolean; logs: boolean; profiles: boolean };
}

/** The value a recorded header has when the server replaced it. */
export const REDACTED = "[redacted]";

/** A recorded body: its bytes as text when they are UTF-8, as base64 otherwise. */
export interface RecordedBody {
  text: string | null;
  base64: string | null;
  /** The program read it. A body nobody read has no bytes. */
  read: boolean;
  /** It stopped at the recording limit, or a part of it is not in the index. */
  cut: boolean;
  /** It was not recorded because it has a secret. */
  withheld: boolean;
}

/** A request's recorded input and response. */
export interface RecordedInput {
  query: string;
  /** `[name, value]` pairs in the order they arrived. A redacted value is `REDACTED`. */
  headers: [string, string][];
  body: RecordedBody;
  response_headers: [string, string][] | null;
  response_body: RecordedBody | null;
}

/** `GET /api/replay`: where a replay is sent, or `null` when this service was started by hand. */
export interface ReplayTarget {
  target: string | null;
}

/** `POST /api/replay`: the replay's status, and its request when the server named it. */
export interface Replayed {
  status: number;
  trace: string | null;
  span: string | null;
}

/** Every kind of record the stream has. `dropped` and `unparseable` belong to no request. */
export const RECORD_KINDS = ["request", "call", "query", "spawn", "gc", "http", "dump", "log", "dropped", "unparseable"] as const;

export type RecordKind = (typeof RECORD_KINDS)[number];

/** The request a stream record belongs to. */
export interface RecordOwner {
  trace: string;
  span: string | null;
  method: string;
  path: string;
  status: number;
}

/** One record of `GET /api/stream`, of any kind. */
export interface StreamRecord {
  /** Unique across the stream, and the record's place in `before`. */
  key: number;
  kind: RecordKind;
  at_us: number;
  batch: number;
  trace: string | null;
  /** A request's method and path, an event's label, a dump's `file:line`, a log line's message, or
   *  the first bytes of an unparseable line. */
  text: string;
  /** A log line's level. */
  level: string | null;
  /** A request's status, or a call's checked-return status. */
  status: number | null;
  /** A dump's file, or the file an unparseable line was read from. */
  file: string | null;
  line: number | null;
  /** A log line's fields, a `dropped` row's counts, or why a line was unparseable. */
  fields: Json;
  request: RecordOwner | null;
}

/** One page of `GET /api/stream`, newest first. */
export interface StreamPage {
  watermark: number;
  records: StreamRecord[];
  /** The `before` of the next older page, `<time>.<key>`, or `null` when this page has the oldest. */
  older: string | null;
}

/** What the stream is filtered on: the part of a stream or log link after `?`. */
export interface StreamFilter {
  /** Only these kinds. Empty is every kind. */
  kinds: RecordKind[];
  /** Only log lines at this level: lower-case letters. */
  level?: string;
  /** Only the records of requests with this route. */
  route?: string;
  /** Only the records of this trace id. */
  trace?: string;
}

/** A page's place in the stream: what arrived after a watermark, or what is older than a record. */
export interface StreamCursor {
  since?: number;
  before?: string;
  limit?: number;
}

/**
 * The query string of `filter`, without the `?`, in a fixed key order. `kind` keeps its commas,
 * because the service splits it on a bare `,`.
 */
export function streamQuery(filter: StreamFilter, cursor: StreamCursor = {}): string {
  const parts: string[] = [];
  if (filter.kinds.length > 0) parts.push(`kind=${filter.kinds.join(",")}`);
  if (filter.level) parts.push(`level=${encodeURIComponent(filter.level)}`);
  if (filter.route) parts.push(`route=${encodeURIComponent(filter.route)}`);
  if (filter.trace) parts.push(`trace=${encodeURIComponent(filter.trace)}`);
  if (cursor.since !== undefined) parts.push(`since=${cursor.since}`);
  if (cursor.before !== undefined) parts.push(`before=${encodeURIComponent(cursor.before)}`);
  if (cursor.limit !== undefined) parts.push(`limit=${cursor.limit}`);
  return parts.join("&");
}

/** Reads a stream or log link's query into its filter. A kind the stream does not have is left out. */
export function streamFilter(query: string): StreamFilter {
  const params = new URLSearchParams(query);
  const kinds = (params.get("kind") ?? "")
    .split(",")
    .filter((kind): kind is RecordKind => (RECORD_KINDS as readonly string[]).includes(kind));
  const filter: StreamFilter = { kinds };
  for (const key of ["level", "route", "trace"] as const) {
    const value = params.get(key);
    if (value) filter[key] = value;
  }
  return filter;
}

/** One page of the stream. */
export function stream(filter: StreamFilter, cursor: StreamCursor = {}, signal?: AbortSignal): Promise<StreamPage> {
  const query = streamQuery(filter, cursor);
  return get<StreamPage>(`/api/stream${query ? `?${query}` : ""}`, signal);
}

/** A file the service stopped reading, and when it tries again. Times are Unix microseconds. */
export interface QuarantinedFile {
  path: string;
  at_us: number;
  retry_us: number;
  /** Failures in a row. */
  failures: number;
  reason: string;
}

/** `GET /api/quarantine`, the newest failure first. */
export function quarantine(signal?: AbortSignal): Promise<{ files: QuarantinedFile[] }> {
  return get<{ files: QuarantinedFile[] }>("/api/quarantine", signal);
}

/** One record a search matched. `snippet` marks each matched word with U+0002 before and U+0003 after. */
export interface SearchHit {
  kind: "request" | "query" | "dump" | "log" | "exception";
  id: number;
  at_us: number;
  snippet: string;
}

/** The matches of one request, or one match that belongs to no request. */
export interface SearchGroup {
  request: RequestRow | null;
  /** In time order. */
  hits: SearchHit[];
  /** The matches past the ones listed. */
  more: number;
}

/** One page of `GET /api/search`, newest group first. */
export interface SearchResults {
  watermark: number;
  groups: SearchGroup[];
  /** The `before` of the next page, or `null` when this page has the last group. */
  older: string | null;
  /** The request the search names, when it is a trace id alone and one request has it. */
  opens: { trace: string; span: string | null } | null;
}

/** One page of a search. A search the service cannot read is an `ApiError` with status `400`. */
export function search(q: string, before?: string, signal?: AbortSignal): Promise<SearchResults> {
  const params = new URLSearchParams({ q });
  if (before !== undefined) params.set("before", before);
  return get<SearchResults>(`/api/search?${params}`, signal);
}

/** A piece of a search snippet: plain text, or a matched word. */
export interface SnippetPart {
  text: string;
  match: boolean;
}

/** Splits a snippet at its U+0002 and U+0003 marks. An empty piece is left out. */
export function snippetParts(snippet: string): SnippetPart[] {
  const parts: SnippetPart[] = [];
  let match = false;
  let text = "";
  for (const ch of snippet) {
    if (ch === "\u0002" || ch === "\u0003") {
      if (text) parts.push({ text, match });
      text = "";
      match = ch === "\u0002";
    } else text += ch;
  }
  if (text) parts.push({ text, match });
  return parts;
}

/** One group of `GET /api/exceptions`: every uncaught throw of one class at one site. */
export interface ExceptionGroup {
  id: number;
  class: string;
  file: string;
  line: number;
  count: number;
  first_us: number;
  last_us: number;
  /** The newest throw's message. */
  message: string | null;
  /** Throws in 24 equal spans from `first_us` to `last_us`. */
  chart: number[];
  /** The routes of the newest throws, newest first, at most 5. */
  routes: string[];
  /** True when throws older than the newest 1000 are not in `chart` and `routes`. */
  sampled: boolean;
}

/** One page of `GET /api/exceptions`. A group changes in place, so a new watermark loads the first
 *  page again. */
export interface ExceptionPage {
  watermark: number;
  groups: ExceptionGroup[];
  /** The `before` of the next page, or `null` when this page has the last group. */
  older: string | null;
}

/** What names one group: its class and the site of its `throw`. */
export type ExceptionKey = Threw;

/** One page of `GET /api/exceptions?class=&file=&line=`: the group and its requests, newest first. */
export interface ExceptionRequests {
  watermark: number;
  group: ExceptionGroup;
  requests: RequestRow[];
  /** The `before` of the next page, or `null` when this page has the oldest request. */
  older: number | null;
}

/** The query string that names `key`, without the `?`, in a fixed key order. */
export function exceptionQuery(key: ExceptionKey): string {
  return `class=${encodeURIComponent(key.class)}&file=${encodeURIComponent(key.file)}&line=${key.line}`;
}

/** Reads an exceptions link's query into the group it names, or `null` when it names none. */
export function exceptionKey(query: string): ExceptionKey | null {
  const params = new URLSearchParams(query);
  const cls = params.get("class");
  const file = params.get("file");
  const line = Number(params.get("line"));
  if (!cls || !file || !Number.isInteger(line) || line < 1) return null;
  return { class: cls, file, line };
}

/** One page of the exception groups. */
export function exceptions(before?: string, signal?: AbortSignal): Promise<ExceptionPage> {
  return get<ExceptionPage>(before === undefined ? "/api/exceptions" : `/api/exceptions?before=${encodeURIComponent(before)}`, signal);
}

/** One page of the requests of the group `key` names. A group that is not there is an `ApiError`
 *  with status `404`. */
export function exceptionRequests(key: ExceptionKey, before?: number, signal?: AbortSignal): Promise<ExceptionRequests> {
  const tail = before === undefined ? "" : `&before=${before}`;
  return get<ExceptionRequests>(`/api/exceptions?${exceptionQuery(key)}${tail}`, signal);
}

/** The view a saved filter opens. */
export type SavedView = "requests" | "stream" | "log" | "search" | "exceptions";

/** A saved filter: its name, its view, and the part of the view's link after `?`. */
export interface SavedFilter {
  name: string;
  view: SavedView;
  query: string;
}

/** Every editor preset the settings offer, and `custom` for a template. */
export const EDITORS = ["vscode", "vscode-insiders", "vscodium", "cursor", "zed", "idea", "phpstorm", "custom"] as const;

export type Editor = (typeof EDITORS)[number];

/** A path prefix the program recorded, and the prefix the editor's machine has instead. */
export interface PathMapping {
  from: string;
  to: string;
}

/** The viewer's settings file. `PUT` replaces it whole. */
export interface Settings {
  editor: Editor;
  /** The link template of `custom`, with `{file}`, `{line}` and `{column}`. Only with `custom`. */
  editor_template?: string;
  /** At most 20. */
  path_mappings: PathMapping[];
  theme: "system" | "dark" | "light";
  filters: SavedFilter[];
}

/** `GET /api/settings`. */
export function settings(signal?: AbortSignal): Promise<Settings> {
  return get<Settings>("/api/settings", signal);
}

/** `PUT /api/settings` with the whole settings. Gives the settings as the service kept them. */
export function saveSettings(next: Settings): Promise<Settings> {
  return send<Settings>("PUT", "/api/settings", next);
}

/** `POST /api/index/discard`: deletes the index and starts an empty one. */
export function discardIndex(): Promise<{ watermark: number }> {
  return send<{ watermark: number }>("POST", "/api/index/discard", {});
}

/** An answer that is not `200`, with the message the service sent. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

/** The query string of `filter`, without the `?`. */
export function requestQuery(filter: RequestFilter): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filter)) {
    if (value !== undefined) params.set(key, String(value));
  }
  return params.toString();
}

/** One page of the request list. */
export function requests(filter: RequestFilter = {}, signal?: AbortSignal): Promise<RequestPage> {
  const query = requestQuery(filter);
  return get<RequestPage>(`/api/requests${query ? `?${query}` : ""}`, signal);
}

/** One request and its records. Without `span`, the trace id must name one request. */
export function request(key: TraceKey, signal?: AbortSignal): Promise<RequestDetail> {
  return get<RequestDetail>(`/api/request?${requestQuery(key)}`, signal);
}

/** Where the browser downloads the profile `id` names. */
export function profileDownload(id: number): string {
  return `/api/profile?id=${id}`;
}

/** The lines the coverage record `id` names. */
export function coverage(id: number, signal?: AbortSignal): Promise<CoverageMap> {
  return get<CoverageMap>(`/api/coverage?id=${id}`, signal);
}

/** The text of `file`, which the coverage record `id` names. */
export function source(id: number, file: string, signal?: AbortSignal): Promise<SourceFile> {
  return get<SourceFile>(`/api/source?coverage=${id}&file=${encodeURIComponent(file)}`, signal);
}

/** Where a replay is sent, or a `null` target when there is nowhere to send one. */
export function replayTarget(signal?: AbortSignal): Promise<ReplayTarget> {
  return get<ReplayTarget>("/api/replay", signal);
}

/** Sends the request `key` names again. `headers` has a value for each redacted header the developer typed. */
export function replay(key: { trace: string; span: string }, headers: Record<string, string>): Promise<Replayed> {
  return send<Replayed>("POST", "/api/replay", { trace: key.trace, span: key.span, headers });
}

/** `GET path` as JSON, or an `ApiError` with the service's message. */
function get<T>(path: string, signal?: AbortSignal): Promise<T> {
  return answer<T>(fetch(path, signal ? { signal } : {}));
}

/** `method path` with `body` as JSON, or an `ApiError` with the service's message. */
function send<T>(method: "PUT" | "POST", path: string, body: unknown): Promise<T> {
  return answer<T>(fetch(path, { method, headers: { "content-type": "application/json" }, body: JSON.stringify(body) }));
}

/** The JSON body of `response`, or an `ApiError` when it is not `200`. */
async function answer<T>(response: Promise<Response>): Promise<T> {
  const res = await response;
  const body: unknown = await res.json().catch(() => null);
  if (!res.ok) {
    const message = typeof body === "object" && body && "error" in body ? String(body.error) : res.statusText;
    throw new ApiError(res.status, message);
  }
  return body as T;
}

/** Calls `onBatch` with each watermark `/api/events` pushes, until `signal` aborts. */
export function watermarks(onBatch: (batch: number) => void, signal: AbortSignal): void {
  const source = new EventSource("/api/events");
  source.onmessage = (event) => {
    const batch = Number(event.data);
    if (Number.isInteger(batch)) onBatch(batch);
  };
  signal.addEventListener("abort", () => source.close(), { once: true });
}

/** A trace id and maybe a span id, read from a pasted `traceresponse` value or a bare trace id. */
export interface TraceKey {
  trace: string;
  span?: string;
}

/**
 * Reads `text` as a `traceresponse` (or `traceparent`) value, `00-<trace>-<span>-<flags>`, or as a
 * bare 32-digit trace id. Gives `null` for anything else. Upper-case hex is accepted and lowered.
 */
export function traceKey(text: string): TraceKey | null {
  const value = text.trim().toLowerCase();
  const full = /^[0-9a-f]{2}-([0-9a-f]{32})-([0-9a-f]{16})-[0-9a-f]{2}$/.exec(value);
  if (full) return { trace: full[1]!, span: full[2]! };
  if (/^[0-9a-f]{32}$/.test(value)) return { trace: value };
  return null;
}
