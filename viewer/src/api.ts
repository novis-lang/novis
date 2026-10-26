// The JSON API `nvs debug` serves under `/api/`. The Rust side owns these shapes
// (`crates/nvs-cli/src/debug/api.rs`): `crates/nvs-cli/tests/debug_api.rs` writes
// `test/fixtures/requests.json` and `test/fixtures/request.json` from a real index, and
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

/** `GET /api/request`: one request and everything it recorded, each list in time order. */
export interface RequestDetail {
  watermark: number;
  request: RequestRow;
  exception: Exception | null;
  events: TraceEvent[];
  dumps: Dump[];
  logs: LogLine[];
  /** Which lists stopped at the most rows the service sends. */
  cut: { events: boolean; dumps: boolean; logs: boolean };
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

/** `GET path` as JSON, or an `ApiError` with the service's message. */
async function get<T>(path: string, signal?: AbortSignal): Promise<T> {
  const res = await fetch(path, signal ? { signal } : {});
  const body: unknown = await res.json();
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
