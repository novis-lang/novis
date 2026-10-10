// `bun nv bench-compare`: Novis, PHP-FPM, Bun and Go, each behind the same nginx terminating TLS 1.3.
//
//     bun nv bench-compare --smoke                               # every peer and case, a few hundred requests
//     bun nv bench-compare --scenarios --smoke                   # every feature row against every peer
//     bun nv bench-compare --scenarios --smoke --row config      # one row
//     bun nv bench-compare --record benches/compare.json         # the fair arm, recorded
//     bun nv bench-compare --arm deployed --record PATH          # four cpus to each backend
//     bun nv bench-compare --down                                # tear every stack down
//
// `benches/compare/README.md` is why the comparison is built the way it is, and this file owns only how a
// run is carried out. The Docker half is `tools/nv/cmd/bench-docker.ts`'s, shared with `bench-proxied`.
//
// # What a run does
//
// The peers are data: `PEERS` names each one's compose file, nginx config, published port, image and the
// settings written into its rows, and a fifth peer is a new entry and a compose file. One stack is up at a
// time, and every (case, peer) pair gets a fresh one, so `memory.peak` is that arm's alone. A full run
// first brings every pair up only to check that the peers answer alike, and times nothing until they do.
// `--smoke` is one pass at a few hundred requests that checks the answers and records nothing.
//
// `--scenarios` runs the feature rows in `SCENARIOS` instead of the cases. A row brings each peer's stack
// up fresh over the row's own programs, runs its probe, and maps what the peer did to one of the four
// results in `Verdict`, with the evidence beside it.

import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { rel, ROOT } from "../lib/paths.ts";
import { ArgError, fixed, parseArgs, pyInt, pyRepr } from "../lib/py.ts";
import { Fail, grouped, host, stamp, version } from "./bench.ts";
import { type Arm, compose, down, type Load, logsOf, messageOf, ohaArm, requireDocker, up } from "./bench-docker.ts";
import { effectivePhp } from "./bench-proxied.ts";

export const summary = "Novis, PHP-FPM, Bun and Go behind one TLS nginx, in containers: nv bench-compare [--smoke] [--scenarios] [--arm fair|deployed] [--record PATH]";

export const COMPARE_DIR = join(ROOT, "benches", "compare");
const GENERATED = join(COMPARE_DIR, "generated");
const TLS_DIR = join(GENERATED, "tls");

export const NGINX_IMAGE = "nginx:1.29.3-alpine";
export const OHA_IMAGE = "ghcr.io/hatoo/oha@sha256:3ec3dbf549ea197793482d47a6324797411406bbf438c2fe8b91f244ec641a2f";
const CERT_IMAGE = "alpine/openssl:3.5.7";
const NGINX_CPUS = 4;

// ---------------------------------------------------------------------------------------------------
// The peers.
// ---------------------------------------------------------------------------------------------------

/** What a peer's entry is given to say how it is started: the backend budget and the case's load. */
export interface PeerCtx {
  cpus: number;
  concurrency: number;
  /** The folder under `benches/compare/` that holds `case`: `cases` or `scenarios`. */
  tree: string;
  case: string;
  commit: string;
  /** The backend container's memory cap, such as `512m`. Unset is no cap. */
  memory?: string | undefined;
}

export interface Peer {
  id: string;
  label: string;
  /** The compose file under `benches/compare/`. */
  stack: string;
  /** The nginx config under `benches/compare/nginx/`: `http.conf` or `fastcgi.conf`. */
  nginx: string;
  /** nginx's published TLS port on the loopback, one per peer so a stack left up names itself. */
  port: number;
  image: string;
  /** The variables the peer's compose file reads beyond the ones every stack reads. */
  env(ctx: PeerCtx): Record<string, string>;
  /** Written into every row, so a figure is tied to the settings that produced it. */
  settings(ctx: PeerCtx): Record<string, string | number | boolean>;
  /** Run before the stack comes up, for a setting that has to be rendered into a file. */
  prepare?(ctx: PeerCtx): void;
}

/** The FPM pool, sized by concurrency as `benches/proxied/php/pool.conf.in` explains. */
export const phpChildren = (concurrency: number) => Math.max(2 * concurrency, 96);

export const PEERS: Peer[] = [
  {
    id: "nvs",
    label: "Novis",
    stack: "compose.nvs.yaml",
    nginx: "http.conf",
    port: 18441,
    image: "nvs-bench-serve:local (built from this checkout)",
    env: (ctx) => ({ NVS_BUILD_COMMIT: ctx.commit }),
    settings: () => ({ server: "nvs serve", workers: "one per visible cpu", config: "defaults" }),
  },
  {
    id: "php",
    label: "PHP",
    stack: "compose.php.yaml",
    nginx: "fastcgi.conf",
    port: 18442,
    image: "php:8.5.9-fpm",
    env: () => ({}),
    settings: (ctx) => ({ server: "php-fpm", pm: "static", "pm.max_children": phpChildren(ctx.concurrency), ...effectivePhp() }),
    prepare: (ctx) => renderPool(phpChildren(ctx.concurrency)),
  },
  {
    id: "bun",
    label: "Bun",
    stack: "compose.bun.yaml",
    nginx: "http.conf",
    port: 18443,
    image: "oven/bun:1.3.5-slim",
    env: () => ({}),
    settings: (ctx) => ({ server: "Bun.serve", processes: ctx.cpus, reusePort: true }),
  },
  {
    id: "go",
    label: "Go",
    stack: "compose.go.yaml",
    nginx: "http.conf",
    port: 18444,
    image: "golang:1.27.2-alpine, run on alpine:3.23.0",
    env: () => ({}),
    settings: (ctx) => ({ server: "net/http", GOMAXPROCS: ctx.cpus }),
  },
];

/** Every variable a peer's compose file reads, so `up`, `exec`, `logs` and `down` all see the same ones. */
export function stackEnv(peer: Peer, ctx: PeerCtx): Record<string, string> {
  return {
    TREE: ctx.tree,
    CASE: ctx.case,
    BACKEND_CPUS: String(ctx.cpus),
    BACKEND_MEMORY: ctx.memory ?? "0",
    NGINX_CONF: peer.nginx,
    NGINX_PORT: String(peer.port),
    ...peer.env(ctx),
  };
}

const stackPath = (peer: Peer) => join(COMPARE_DIR, peer.stack);

// ---------------------------------------------------------------------------------------------------
// The cases, and how their answers are compared.
// ---------------------------------------------------------------------------------------------------

export interface Input {
  method: string;
  path: string;
  body?: string;
  contentType?: string;
}

export interface Case {
  id: string;
  /** Connections `oha` holds open. */
  concurrency: number;
  /** The request `oha` times. */
  timed: Input;
  /** Every request whose answer must agree across the peers before anything is timed. */
  inputs: Input[];
  /** Whether bodies are compared after `htmlEntities`, for a case that escapes text into HTML. */
  html: boolean;
}

/** The json-api case's order: one customer and 20 items, about 2 KB, the same bytes on every run. */
const ORDER = readFileSync(join(COMPARE_DIR, "cases", "json-api", "order.json"), "utf8").trim();
const orderInput = (body: string): Input => ({ method: "POST", path: "/orders", body, contentType: "application/json" });

/** The html-page case's search term: every character an HTML escaper must replace, and no `+`, which `html/template` also escapes. */
const SEARCH_TERM = `<b>"Tea" & 'Cups'</b>`;
const productsInput = (term: string): Input => ({ method: "GET", path: `/products?q=${encodeURIComponent(term)}` });

export const CASES: Case[] = [
  {
    id: "hello",
    concurrency: 64,
    timed: { method: "GET", path: "/" },
    inputs: [{ method: "GET", path: "/" }],
    html: false,
  },
  {
    id: "json-api",
    concurrency: 64,
    timed: orderInput(ORDER),
    inputs: [
      orderInput(ORDER),
      // Each of these fails one check, so every peer must answer 422 with the same body.
      orderInput(ORDER.replace('"qty":2', '"qty":0')),
      orderInput(ORDER.replace('"qty":2', '"qty":"2"')),
      orderInput('{"customer":'),
      orderInput('{"customer":{"id":1,"name":"A","email":"a@example.com"},"items":[]}'),
    ],
    html: false,
  },
  {
    id: "html-page",
    concurrency: 64,
    timed: productsInput(SEARCH_TERM),
    inputs: [
      productsInput(SEARCH_TERM),
      { method: "GET", path: "/products" },
      productsInput("tea"),
      productsInput("café"),
    ],
    html: true,
  },
  {
    id: "wait",
    concurrency: 1000,
    timed: { method: "GET", path: "/wait" },
    inputs: [{ method: "GET", path: "/wait" }],
    html: false,
  },
];

/** One answer, as the agreement check compares it. */
export interface Answer {
  status: number;
  contentType: string;
  body: string;
}

/**
 * Maps the spellings of `'` and `"` that differ between the peers' escapers to one each, and nothing else:
 * `&#039;`, `&#x27;` and `&apos;` become `&#39;`, and `&#34;` becomes `&quot;`.
 */
export function htmlEntities(body: string): string {
  return body.replace(/&#0*39;|&#x0*27;|&apos;/gi, "&#39;").replace(/&#0*34;/g, "&quot;");
}

/** `text/html; charset=UTF-8` and `text/html;charset=utf-8` are one type. */
export const contentTypeOf = (raw: string | null) => (raw ?? "").toLowerCase().replace(/\s+/g, "");

/** An answer as it is compared: the type normalised, and the body mapped when the case is HTML. */
export function comparable(c: Case, a: Answer): Answer {
  return { status: a.status, contentType: contentTypeOf(a.contentType), body: c.html ? htmlEntities(a.body) : a.body };
}

export interface Disagreement {
  peer: string;
  input: number;
  why: string;
}

/**
 * The first peer whose answer to some input differs from the first peer's, or null when all agree.
 * `answers` maps a peer's id to its answers, in the order of `c.inputs`.
 */
export function agreement(c: Case, answers: Map<string, Answer[]>): Disagreement | null {
  const peers = [...answers.keys()];
  const reference = peers[0];
  if (reference === undefined) return null;
  const want = answers.get(reference)!.map((a) => comparable(c, a));
  for (const peer of peers.slice(1)) {
    const got = answers.get(peer)!.map((a) => comparable(c, a));
    for (let i = 0; i < c.inputs.length; i++) {
      const w = want[i];
      const g = got[i];
      if (w === undefined || g === undefined) return { peer, input: i, why: "no answer" };
      if (g.status !== w.status) return { peer, input: i, why: `status ${g.status}, ${reference} sent ${w.status}` };
      if (g.contentType !== w.contentType) return { peer, input: i, why: `Content-Type ${pyRepr(g.contentType)}, ${reference} sent ${pyRepr(w.contentType)}` };
      if (g.body !== w.body) return { peer, input: i, why: `body ${pyRepr(g.body.slice(0, 200))}, ${reference} sent ${pyRepr(w.body.slice(0, 200))}` };
    }
  }
  return null;
}

// ---------------------------------------------------------------------------------------------------
// The feature scenarios, and how what a peer did becomes a result.
// ---------------------------------------------------------------------------------------------------

/** What a row says of one peer: the feature is there, one setting away, code the developer writes, or absent. */
export type Verdict = "built-in" | "one-setting" | "your-code" | "no";

/** One peer's result in one row, with the measured evidence beside it. */
export interface RowResult {
  result: Verdict;
  /** The setting a `one-setting` result names. */
  setting?: string;
  evidence: Record<string, string | number | boolean | null>;
}

/** One request's reply, with its header names in lower case. */
export interface Reply {
  status: number;
  headers: Record<string, string>;
  body: string;
}

/** What a row's probe can do against one peer's running stack. */
export interface Probe {
  /** One `GET`, given up after `timeoutMs`, which is 30 seconds unless the row names its own. */
  get(path: string, timeoutMs?: number): Promise<Reply>;
  /** One request with its own method and headers, for a row that is about more than a `GET`. */
  send(method: string, path: string, headers: Record<string, string>): Promise<Reply>;
  /** Whether the backend container is still running. No stack sets a restart policy, so a crash stays down. */
  running(): boolean;
  /** `nvs check` over a file of the row's folder, in a one-off app container: the exit code and what it printed. */
  check(file: string): { code: number; output: string };
  /** A command run in the backend container, as an operator would run it there. */
  exec(command: string[]): { code: number; output: string };
  /** The backend's last log lines. */
  logs(): string;
  /** `oha` against `path` from inside the stack for `seconds`, at the row's connections, while the row sends its own requests. */
  steady(path: string, seconds: number): Promise<Steady>;
  /** The peer's `id` in `PEERS`. */
  peer: string;
  /** The host folder the stack serves, which a row that edits the program writes to. */
  dir: string;
}

/** What steady load did: the requests answered, the ones that failed or timed out and how, and the slowest 1% in ms. */
export interface Steady {
  requests: number;
  failed: number;
  failures: string | null;
  p99_ms: number | null;
}

/** What a row that times a case can do against one peer. */
export interface Timer {
  /** The peer's `id` in `PEERS`. */
  peer: string;
  /** Requests each pass sends, and connections the case holds open. */
  requests: number;
  concurrency: number;
  /** One fresh, timed pass of the row's case at a backend budget of `cpus`, with its memory. */
  pass(cpus: number): Promise<PeerResult>;
}

export interface Scenario {
  id: string;
  /** The folder under `benches/compare/` and the program folder in it the stack serves. */
  tree: string;
  case: string;
  /** `false` for a row that only checks a program, so no stack is brought up. */
  serves?: boolean;
  /** Serve a fresh copy of the folder under `generated/`, for a row that edits the program. */
  copy?: boolean;
  /** The backend container's memory cap, for a row whose program allocates without end. */
  memory?: string;
  /** The result of a peer that has nothing to run for this row, with the documentation that says so. */
  fixed?: Record<string, RowResult>;
  /** A row about requests: sent to one stack the driver brings up. */
  probe?(p: Probe): Promise<RowResult>;
  /** A row about a figure: times the case in `cases/` named by `case`, each pass on a fresh stack. */
  timed?(t: Timer): Promise<RowResult>;
}

/** The factor from one cpu to four below which a peer is not using the extra cores. */
export const CORES_SCALING = 2;

/** The code each peer's program carries only to use more than one core, where it carries any. */
export const CORES_IN_CODE: Record<string, string> = { bun: "a launcher that starts one reusePort process per cpu" };

/**
 * Throughput that grows by `CORES_SCALING` or more from one cpu to four uses the cores: built in, or
 * `your-code` where the peer needs a launcher of its own for it. Less than that is `no`.
 */
export function coresResult(one: PeerResult, four: PeerResult, inCode: string | undefined): RowResult {
  const scaling = Number((four.requests_per_sec / one.requests_per_sec).toFixed(2));
  const evidence = {
    requests_per_sec_1_cpu: Math.round(one.requests_per_sec),
    requests_per_sec_4_cpus: Math.round(four.requests_per_sec),
    scaling,
    errors: one.errors + four.errors,
    code: inCode ?? null,
  };
  if (!(scaling >= CORES_SCALING)) return { result: "no", evidence };
  return { result: inCode === undefined ? "built-in" : "your-code", evidence };
}

/** The setting each peer needs before it holds the wait case's connections, where it needs one. */
export const WAITING_SETTING: Record<string, string> = { php: "pm.max_children" };

/**
 * Every request answered `200` is built in, or `one-setting` where the peer needs `WAITING_SETTING` for
 * it. A failed request is `no`. The memory each request in flight costs is the evidence beside it:
 * `memory.peak` less the idle `memory.current`, over the requests that can be in flight at once.
 */
export function waitingResult(r: PeerResult, inFlight: number, setting: string | undefined): RowResult {
  const failed = r.errors + Object.entries(r.status_codes).reduce((n, [code, count]) => (code === "200" ? n : n + count), 0);
  const kib = r.memory_peak_bytes === null || r.memory_idle_bytes === null ? null : Number(((r.memory_peak_bytes - r.memory_idle_bytes) / inFlight / 1024).toFixed(1));
  const evidence = { in_flight: inFlight, kib_per_request: kib, failed_requests: failed };
  if (failed > 0) return { result: "no", evidence };
  return setting === undefined ? { result: "built-in", evidence } : { result: "one-setting", setting, evidence };
}

/** Requests the isolation row sends, one after another, each writing its own value. */
export const ISOLATION_REQUESTS = 20;

/** Each body is `<value found> <value written>`. A request found clean state when the first is `0`. */
export function isolationResult(bodies: string[]): RowResult {
  const leaked = bodies.filter((b) => !b.startsWith("0 ")).length;
  return { result: leaked === 0 ? "built-in" : "no", evidence: { requests: bodies.length, saw_another_request: leaked } };
}

/** How long the runaway row's steady load runs, and how long after the load starts the runaway requests are sent. */
export const RUNAWAY_LOAD_SECONDS = 15;
export const RUNAWAY_START_MS = 3_000;

/** How long a runaway request may take to be stopped: under nginx's 60-second read timeout, so a reply is the backend's. */
export const RUNAWAY_WAIT_MS = 55_000;

/** The backend container's memory cap in the runaway row, so an allocation without end stops there and not at the host. */
export const RUNAWAY_MEMORY = "512m";

/** The statuses nginx gives when the backend did not answer: a reply with one of them is not the backend stopping the request. */
const NGINX_STATUSES = new Set([502, 504]);

/** A runaway request's reply: its status and the seconds it took, or `null` for both when it was never answered. */
export interface Stopped {
  status: number | null;
  seconds: number | null;
}

/**
 * Built in when the steady load saw no failed request, both runaway requests were stopped by the backend,
 * and the backend still runs. `one-setting` where the peer needs a setting for it. Anything else is `no`.
 */
export function runawayResult(load: Steady, loop: Stopped, alloc: Stopped, running: boolean, setting: string | undefined): RowResult {
  const stopped = (s: Stopped) => s.status !== null && !NGINX_STATUSES.has(s.status);
  const evidence = {
    requests: load.requests,
    failed_requests: load.failed,
    failures: load.failures,
    p99_ms: load.p99_ms,
    loop_status: loop.status,
    loop_seconds: loop.seconds,
    alloc_status: alloc.status,
    alloc_seconds: alloc.seconds,
    backend_running: running,
  };
  if (load.failed > 0 || !running || !stopped(loop) || !stopped(alloc)) return { result: "no", evidence };
  return setting === undefined ? { result: "built-in", evidence } : { result: "one-setting", setting, evidence };
}

/**
 * The limits a peer needs set before it stops a runaway request, where its defaults set none: the command
 * that sets them inside the container, and the setting's name in the row. Novis's `[limits]` keys are
 * commented out in its data folder's `nvs.toml`, so the edit uncomments them inside that table. `nvs serve`
 * admits only as many requests as the container's memory affords at `memory` each, so the value leaves
 * room for the row's load and its two runaways: at `64M` a 512 MiB container admits six.
 */
export const RUNAWAY_LIMITS: Record<string, { edit: string[]; name: string }> = {
  nvs: {
    edit: ["sed", "-i", '/^\\[limits\\]$/,/^\\[/{s|^#memory = .*|memory = "16M"|;s|^#cpu_time = .*|cpu_time = "1s"|}', "/usr/local/bin/.nvsdata/nvs.toml"],
    name: "[limits]",
  },
};

/** Sends one runaway request and waits up to `RUNAWAY_WAIT_MS` for its reply. */
async function stoppedAt(p: Probe, path: string): Promise<Stopped> {
  const start = performance.now();
  try {
    const reply = await p.get(path, RUNAWAY_WAIT_MS);
    return { status: reply.status, seconds: Number(((performance.now() - start) / 1000).toFixed(1)) };
  } catch {
    return { status: null, seconds: null };
  }
}

/** Polls the backend's log until it carries `line`: the seconds it took, or `null` after `CHANGE_WAIT_MS`. */
async function logUntil(p: Probe, line: string): Promise<number | null> {
  const start = performance.now();
  while (performance.now() - start < CHANGE_WAIT_MS) {
    if (p.logs().includes(line)) return Number(((performance.now() - start) / 1000).toFixed(1));
    await Bun.sleep(250);
  }
  return null;
}

/** The request after a crash must be answered by a backend that is still up. */
export function crashResult(crashStatus: number, nextStatus: number, running: boolean): RowResult {
  return {
    result: nextStatus === 200 && running ? "built-in" : "no",
    evidence: { crash_status: crashStatus, next_status: nextStatus, backend_running: running },
  };
}

/** All three headers, or the developer writes the missing ones. */
export function headersResult(headers: Record<string, string>): RowResult {
  const nosniff = (headers["x-content-type-options"] ?? "").trim().toLowerCase() === "nosniff";
  const framePolicy = headers["x-frame-options"] !== undefined || /frame-ancestors/i.test(headers["content-security-policy"] ?? "");
  const referrerPolicy = headers["referrer-policy"] !== undefined;
  return {
    result: nosniff && framePolicy && referrerPolicy ? "built-in" : "your-code",
    evidence: { nosniff, frame_policy: framePolicy, referrer_policy: referrerPolicy },
  };
}

/** The term the escape row prints. A body that carries it raw would run it in a browser. */
export const ESCAPE_TERM = "<script>alert(1)</script>";

/** Printed the documented way, the term arrives escaped, or the developer calls an escaper. */
export function escapeResult(body: string): RowResult {
  const raw = body.includes("<script>");
  return { result: raw ? "your-code" : "built-in", evidence: { raw_script_in_body: raw } };
}

/** The `Origin` the csrf row's cross-site request names. */
export const OTHER_ORIGIN = "https://other.example";

/**
 * The same-site `POST` must change state, or the row cannot say anything. The cross-site one is rejected
 * when its reply does not carry `changed`, whatever status says so.
 */
export function csrfResult(sameSite: Reply, crossSite: Reply): RowResult {
  if (sameSite.status !== 200 || !sameSite.body.includes("changed")) {
    throw new Fail(`the same-site POST: status ${sameSite.status}, body ${pyRepr(sameSite.body.slice(0, 200))}`);
  }
  const rejected = !crossSite.body.includes("changed");
  return { result: rejected ? "built-in" : "your-code", evidence: { cross_site_status: crossSite.status, rejected } };
}

/** `nvs check` refused the program with the error `code`, or it has no such check. */
export function checkResult(code: number, output: string, expected: string): RowResult {
  const refused = code !== 0 && output.includes(`error[${expected}]`);
  return { result: refused ? "built-in" : "no", evidence: { check_exit: code, error: refused ? expected : null } };
}

/** A peer with no check for the row's mistake: `no`, and the page it documents the mistake on. */
const noCheck = (docs: string): RowResult => ({ result: "no", evidence: { check: "none", docs } });

/** How long a row that waits for a change polls before it says the change never came. */
export const CHANGE_WAIT_MS = 20_000;

/**
 * An edit that shows is built in. One that never shows is `one-setting` where the peer has the setting that
 * would show it, and `no` where it has none. The seconds stay `null` in the evidence either way.
 */
export function liveEditResult(seconds: number | null, setting: string | undefined): RowResult {
  const evidence = { seconds_to_new_answer: seconds, restarts: 0 };
  if (seconds !== null) return { result: "built-in", evidence };
  return setting === undefined ? { result: "no", evidence } : { result: "one-setting", setting, evidence };
}

/** The setting each peer's documentation names for showing an edit without a restart, where it has one. */
export const LIVE_EDIT_SETTING: Record<string, string> = { php: "opcache.validate_timestamps=1", bun: "bun --hot" };

/** The line the live-edit program prints, and the line the driver edits it to. */
export const LIVE_EDIT_FROM = "version 1";
export const LIVE_EDIT_TO = "version 2";

/**
 * A changed config file that applies on its own is built in. One that applies after a reload command is
 * `one-setting`, naming the command. One that applies only after a restart is `no`.
 */
export function configResult(withoutCommand: number | null, afterCommand: number | null, command: string | undefined): RowResult {
  const evidence = { seconds_without_command: withoutCommand, seconds_after_command: afterCommand, command: command ?? null, restarts: 0 };
  if (withoutCommand !== null) return { result: "built-in", evidence };
  if (afterCommand !== null && command !== undefined) return { result: "one-setting", setting: command, evidence };
  return { result: "no", evidence };
}

/** One peer's config change: the command that edits its config file, the reply that shows it, and its reload command. */
export interface ConfigChange {
  edit: string[];
  applied(r: Reply): boolean;
  /** The command the peer's documentation gives for a reload, and its name in the row. */
  reload?: { command: string[]; name: string };
}

/**
 * The edits are made inside the container. A bind mount shows every file as world-writable, and Novis
 * refuses a config file anybody can write, so its file is the data folder's `nvs.toml` the image wrote.
 * That file already has an `[http.headers]` table with the key commented out, so the edit uncomments it.
 */
export const CONFIG_CHANGES: Record<string, ConfigChange> = {
  nvs: {
    edit: ["sed", "-i", 's|^#referrer_policy = .*|referrer_policy = "same-origin"|', "/usr/local/bin/.nvsdata/nvs.toml"],
    applied: (r) => r.headers["referrer-policy"] === "same-origin",
  },
  php: {
    edit: ["sh", "-c", "echo 'expose_php = Off' > /usr/local/etc/php/conf.d/zz-config.ini"],
    applied: (r) => r.headers["x-powered-by"] === undefined,
    reload: { command: ["sh", "-c", "kill -USR2 1"], name: "php-fpm reload" },
  },
};

/** A peer whose response headers are written in the program, with no config file that sets them. */
const headersInCode: RowResult = { result: "your-code", evidence: { config_file: "none", headers_set_in: "the program" } };

/** Polls `/` until `test` holds: the seconds it took, or `null` after `CHANGE_WAIT_MS`. A failed request is polled again. */
async function secondsUntil(p: Probe, test: (r: Reply) => boolean): Promise<number | null> {
  const start = performance.now();
  while (performance.now() - start < CHANGE_WAIT_MS) {
    try {
      if (test(await p.get("/"))) return Number(((performance.now() - start) / 1000).toFixed(1));
    } catch {
      // nginx answers for a backend that is reloading; a reset connection is polled again like a 502.
    }
    await Bun.sleep(250);
  }
  return null;
}

/** A reply the row needs to be a 200, or the row cannot say anything. */
function ok(reply: Reply, what: string): Reply {
  if (reply.status !== 200) throw new Fail(`${what}: status ${reply.status}, body ${pyRepr(reply.body.slice(0, 200))}`);
  return reply;
}

/** The rows, in the order the page shows them. `benches/compare/README.md` § *The feature scenarios* is why each is built so. */
export const SCENARIOS: Scenario[] = [
  {
    id: "cores",
    tree: "cases",
    case: "hello",
    timed: async (t) => coresResult(await t.pass(1), await t.pass(4), CORES_IN_CODE[t.peer]),
  },
  {
    id: "isolation",
    tree: "scenarios",
    case: "isolation",
    probe: async (p) => {
      const bodies: string[] = [];
      for (let v = 1; v <= ISOLATION_REQUESTS; v++) bodies.push(ok(await p.get(`/?v=${v}`), `?v=${v}`).body);
      return isolationResult(bodies);
    },
  },
  {
    id: "runaway",
    tree: "scenarios",
    case: "runaway",
    memory: RUNAWAY_MEMORY,
    probe: async (p) => {
      const limits = RUNAWAY_LIMITS[p.peer];
      if (limits !== undefined) {
        const wrote = p.exec(limits.edit);
        if (wrote.code !== 0) throw new Fail(`${limits.edit.join(" ")}: ${wrote.output}`);
        if ((await logUntil(p, "configuration reloaded")) === null) throw new Fail(`${limits.name} was written, and the configuration was not reloaded`);
      }
      ok(await p.get("/"), "the first request");
      const load = p.steady("/", RUNAWAY_LOAD_SECONDS);
      await Bun.sleep(RUNAWAY_START_MS);
      const runaways = Promise.all([stoppedAt(p, "/loop"), stoppedAt(p, "/alloc")]);
      const steady = await load;
      const [loop, alloc] = await runaways;
      return runawayResult(steady, loop, alloc, p.running(), limits?.name);
    },
  },
  {
    id: "crash",
    tree: "scenarios",
    case: "crash",
    probe: async (p) => {
      const crash = await p.get("/crash");
      const next = await p.get("/");
      return crashResult(crash.status, next.status, p.running());
    },
  },
  {
    id: "escape",
    tree: "scenarios",
    case: "escape",
    probe: async (p) => escapeResult(ok(await p.get(`/?q=${encodeURIComponent(ESCAPE_TERM)}`), "the term").body),
  },
  {
    id: "taint",
    tree: "scenarios",
    case: "taint",
    serves: false,
    fixed: {
      php: noCheck("https://www.php.net/manual/en/security.database.sql-injection.php"),
      bun: noCheck("https://bun.sh/docs/api/sql"),
      go: noCheck("https://go.dev/doc/database/sql-injection"),
    },
    probe: async (p) => {
      const c = p.check("main.nvs");
      return checkResult(c.code, c.output, "E0401");
    },
  },
  {
    id: "secret",
    tree: "scenarios",
    case: "secret",
    serves: false,
    fixed: {
      php: noCheck("https://www.php.net/manual/en/class.sensitiveparameter.php"),
      bun: noCheck("https://bun.sh/docs/runtime/env"),
      go: noCheck("https://pkg.go.dev/log/slog#LogValuer"),
    },
    probe: async (p) => {
      const c = p.check("main.nvs");
      return checkResult(c.code, c.output, "E0790");
    },
  },
  {
    id: "csrf",
    tree: "scenarios",
    case: "csrf",
    probe: async (p) => csrfResult(await p.send("POST", "/change", {}), await p.send("POST", "/change", { Origin: OTHER_ORIGIN })),
  },
  {
    id: "headers",
    tree: "cases",
    case: "hello",
    probe: async (p) => headersResult(ok(await p.get("/"), "hello").headers),
  },
  {
    id: "waiting",
    tree: "cases",
    case: "wait",
    timed: async (t) => waitingResult(await t.pass(1), Math.min(t.requests, t.concurrency), WAITING_SETTING[t.peer]),
  },
  {
    id: "live-edit",
    tree: "scenarios",
    case: "live-edit",
    copy: true,
    fixed: { go: { result: "no", evidence: { compiled_into: "the image", restarts: 1 } } },
    probe: async (p) => {
      const before = ok(await p.get("/"), "the first version").body;
      if (!before.includes(LIVE_EDIT_FROM)) throw new Fail(`the first version: body ${pyRepr(before.slice(0, 200))}`);
      for (const name of readdirSync(p.dir)) {
        const file = join(p.dir, name);
        writeFileSync(file, readFileSync(file, "utf8").replace(LIVE_EDIT_FROM, LIVE_EDIT_TO));
      }
      return liveEditResult(await secondsUntil(p, (r) => r.status === 200 && r.body.includes(LIVE_EDIT_TO)), LIVE_EDIT_SETTING[p.peer]);
    },
  },
  {
    id: "config",
    tree: "cases",
    case: "hello",
    fixed: { bun: headersInCode, go: headersInCode },
    probe: async (p) => {
      const change = CONFIG_CHANGES[p.peer]!;
      if (change.applied(ok(await p.get("/"), "hello"))) throw new Fail("the header already has the value the change sets");
      const wrote = p.exec(change.edit);
      if (wrote.code !== 0) throw new Fail(`${change.edit.join(" ")}: ${wrote.output}`);
      const without = await secondsUntil(p, change.applied);
      if (without !== null || change.reload === undefined) return configResult(without, null, undefined);
      const reloaded = p.exec(change.reload.command);
      if (reloaded.code !== 0) throw new Fail(`${change.reload.name}: ${reloaded.output}`);
      return configResult(null, await secondsUntil(p, change.applied), change.reload.name);
    },
  },
];

/** `built-in`, or `one-setting (limits.memory)` for a result that names its setting. */
export const verdictText = (r: RowResult) => (r.setting === undefined ? r.result : `${r.result} (${r.setting})`);

/** The line `--scenarios` prints for a row: `<id>: <peer> <result>, ...` in `PEERS` order. */
export function rowLine(id: string, byPeer: Map<string, RowResult>): string {
  const parts = PEERS.filter((p) => byPeer.has(p.id)).map((p) => `${p.label} ${verdictText(byPeer.get(p.id)!)}`);
  return `${id}: ${parts.join(", ")}`;
}

// ---------------------------------------------------------------------------------------------------
// The record.
// ---------------------------------------------------------------------------------------------------

/** One peer's figures in one case. Memory is the backend container's cgroup, in bytes. */
export interface PeerResult {
  requests_per_sec: number;
  median_requests_per_sec: number;
  p50_ms: number | null;
  p99_ms: number | null;
  errors: number;
  status_codes: Record<string, number>;
  memory_idle_bytes: number | null;
  memory_peak_bytes: number | null;
  settings: Record<string, string | number | boolean>;
}

export interface RunInfo {
  date: string;
  commit: string;
  arm: "fair" | "deployed";
  backendCpus: number;
  requests: number;
  reps: number;
  machine: Record<string, unknown>;
  digests: Record<string, string>;
}

/** The figures `oha` gave for one arm, in the record's units. */
export function resultOf(arm: Arm, memory: { idle: number | null; peak: number | null }, settings: PeerResult["settings"]): PeerResult {
  const errors = Object.values((arm.errors ?? {}) as Record<string, number>).reduce((a, b) => a + b, 0);
  return {
    requests_per_sec: arm.requests_per_sec,
    median_requests_per_sec: arm.median_requests_per_sec,
    p50_ms: arm.latency_ms?.p50 ?? null,
    p99_ms: arm.latency_ms?.p99 ?? null,
    errors,
    status_codes: arm.status_codes ?? {},
    memory_idle_bytes: memory.idle,
    memory_peak_bytes: memory.peak,
    settings,
  };
}

/**
 * The whole record `--record` writes: the machine, every peer's image, and per case every peer's figures
 * in `PEERS` order. Stage 5's scenario rows join it under their own key; nothing joins it to
 * `benches/serve-proxied.json`.
 */
export function buildRecord(run: RunInfo, results: Map<string, Map<string, PeerResult>>) {
  const cases: Record<string, { concurrency: number; peers: Record<string, PeerResult> }> = {};
  for (const c of CASES) {
    const byPeer = results.get(c.id);
    if (byPeer === undefined) continue;
    const peers: Record<string, PeerResult> = {};
    for (const p of PEERS) {
      const r = byPeer.get(p.id);
      if (r !== undefined) peers[p.id] = r;
    }
    cases[c.id] = { concurrency: c.concurrency, peers };
  }
  return {
    date: run.date,
    commit: run.commit,
    arm: run.arm,
    backend_cpus: run.backendCpus,
    nginx_cpus: NGINX_CPUS,
    tls: "TLSv1.3",
    requests: run.requests,
    reps: run.reps,
    machine: run.machine,
    peers: Object.fromEntries(PEERS.map((p) => [p.id, { label: p.label, image: p.image }])),
    images: { nginx: NGINX_IMAGE, oha: OHA_IMAGE },
    cases,
    config_digests: run.digests,
  };
}

/** Every file whose contents could move a number, digested into the record. */
const TUNABLES = [
  ...PEERS.map((p) => join(COMPARE_DIR, p.stack)),
  join(COMPARE_DIR, "compose.common.yaml"),
  join(COMPARE_DIR, "nginx", "http.conf"),
  join(COMPARE_DIR, "nginx", "fastcgi.conf"),
  join(COMPARE_DIR, "bun", "serve.ts"),
  join(COMPARE_DIR, "go", "Dockerfile"),
  join(ROOT, "benches", "proxied", "php", "opcache.ini"),
  join(ROOT, "benches", "proxied", "php", "pool.conf.in"),
];

function digests(): Record<string, string> {
  const out: Record<string, string> = {};
  for (const path of TUNABLES) {
    if (existsSync(path) && statSync(path).isFile()) {
      out[rel(path).replaceAll("\\", "/")] = createHash("sha256").update(readFileSync(path)).digest("hex").slice(0, 16);
    }
  }
  return out;
}

// ---------------------------------------------------------------------------------------------------
// Rendering, the certificate, and reading a stack.
// ---------------------------------------------------------------------------------------------------

/** Writes `generated/pool.conf` from the proxied leg's template, which says why it is rendered. */
function renderPool(children: number): void {
  mkdirSync(GENERATED, { recursive: true });
  const template = readFileSync(join(ROOT, "benches", "proxied", "php", "pool.conf.in"), "utf8");
  writeFileSync(join(GENERATED, "pool.conf"), template.replaceAll("@PHP_CHILDREN@", String(children)), "utf8");
}

/** Makes the self-signed certificate nginx serves, once: the folder is git-ignored and kept between runs. */
function ensureCert(): void {
  if (existsSync(join(TLS_DIR, "cert.pem")) && existsSync(join(TLS_DIR, "key.pem"))) return;
  mkdirSync(TLS_DIR, { recursive: true });
  const out = Bun.spawnSync(
    [
      "docker", "run", "--rm", "-v", `${TLS_DIR}:/out`, CERT_IMAGE,
      "req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256", "-nodes", "-days", "3650",
      "-subj", "/CN=localhost", "-addext", "subjectAltName=DNS:localhost,DNS:nginx,IP:127.0.0.1",
      "-keyout", "/out/key.pem", "-out", "/out/cert.pem",
    ],
    { stdout: "pipe", stderr: "pipe", timeout: 300_000 },
  );
  if (out.exitCode !== 0) throw new Fail(`making the TLS certificate failed: ${out.stderr.toString().trim()}`);
}

/** A cgroup v2 memory file of the backend container, in bytes, or null where the kernel has none. */
function cgroupBytes(peer: Peer, env: Record<string, string>, file: "memory.current" | "memory.peak"): number | null {
  const done = compose(stackPath(peer), ["exec", "-T", "app", "cat", `/sys/fs/cgroup/${file}`], env, true);
  if (done.code !== 0) return null;
  return pyInt(done.stdout.trim());
}

/** One request from the host through nginx, for the agreement check. Never timed. */
async function answerOf(peer: Peer, input: Input): Promise<Answer> {
  const headers: Record<string, string> = { "Accept-Encoding": "identity" };
  if (input.contentType !== undefined) headers["Content-Type"] = input.contentType;
  const res = await fetch(`https://127.0.0.1:${peer.port}${input.path}`, {
    method: input.method,
    headers,
    body: input.body,
    tls: { rejectUnauthorized: false },
    signal: AbortSignal.timeout(30_000),
  });
  return { status: res.status, contentType: res.headers.get("content-type") ?? "", body: await res.text() };
}

// ---------------------------------------------------------------------------------------------------
// One peer in one case.
// ---------------------------------------------------------------------------------------------------

interface Pass {
  answers: Answer[];
  result: PeerResult | null;
}

/**
 * Brings the peer's stack up fresh for one case, takes its answers, and when `load` is given times it,
 * reading `memory.current` before the first request and `memory.peak` after the last. Always torn down.
 */
async function onePass(peer: Peer, c: Case, ctx: PeerCtx, load: Load | null): Promise<Pass> {
  const env = stackEnv(peer, ctx);
  const stack = stackPath(peer);
  peer.prepare?.(ctx);
  try {
    up(stack, env, "app");
    const idle = cgroupBytes(peer, env, "memory.current");
    up(stack, env, "nginx");
    const answers: Answer[] = [];
    for (const input of c.inputs) answers.push(await answerOf(peer, input));
    if (load === null) return { answers, result: null };
    const flags = ["--insecure", "-m", c.timed.method];
    if (c.timed.body !== undefined) flags.push("-d", c.timed.body);
    if (c.timed.contentType !== undefined) flags.push("-T", c.timed.contentType);
    const arm = ohaArm(stack, `https://nginx${c.timed.path}`, load, env, `${c.id}-${peer.id}`, peer.label, c.id, flags);
    const peak = cgroupBytes(peer, env, "memory.peak");
    return { answers, result: resultOf(arm, { idle, peak }, peer.settings(ctx)) };
  } catch (e) {
    throw new Fail(`${peer.label}, case ${c.id}: ${messageOf(e)}\n${peer.id} said:\n${logsOf(stack, "app", env)}`);
  } finally {
    down(stack, env);
  }
}

/** One request from the host through nginx, for a scenario's probe. */
async function replyOf(peer: Peer, path: string, method = "GET", extra: Record<string, string> = {}, timeoutMs = 30_000): Promise<Reply> {
  const res = await fetch(`https://127.0.0.1:${peer.port}${path}`, {
    method,
    headers: { "Accept-Encoding": "identity", ...extra },
    tls: { rejectUnauthorized: false },
    signal: AbortSignal.timeout(timeoutMs),
  });
  const headers: Record<string, string> = {};
  res.headers.forEach((value, name) => {
    headers[name.toLowerCase()] = value;
  });
  return { status: res.status, headers, body: await res.text() };
}

/**
 * One `oha` run for `seconds` from inside the stack's network, started without waiting for it, so the row
 * sends its own requests while it runs. A request unanswered after five seconds counts as failed.
 */
async function steadyOf(stack: string, env: Record<string, string>, path: string, seconds: number): Promise<Steady> {
  const args = ["run", "--rm", "--no-deps", "oha", "-z", `${seconds}s`, "-c", String(SCENARIO_CONCURRENCY), "-t", "5s"];
  const proc = Bun.spawn(["docker", "compose", "-f", stack, ...args, "--no-tui", "--output-format", "json", "--insecure", `https://nginx${path}`], {
    env: { ...process.env, ...env },
    stdout: "pipe",
    stderr: "pipe",
  });
  const [out, err, code] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
  if (code !== 0) throw new Error(`oha failed against ${path} (${code}): ${err.trim()}`);
  return steadyResult(JSON.parse(out));
}

/** `oha`'s name for a request still open when `-z` ends. The run cut it, so it is not a failure. */
const DEADLINE = "aborted due to deadline";

/** The figures `oha`'s JSON gives for a steady run. Every status but `200` and every error but `DEADLINE` failed. */
export function steadyResult(json: { statusCodeDistribution?: Record<string, number>; errorDistribution?: Record<string, number>; metrics?: { latency_ms?: { p99?: number } } }): Steady {
  const failures = [
    ...Object.entries(json.statusCodeDistribution ?? {}).filter(([status]) => status !== "200"),
    ...Object.entries(json.errorDistribution ?? {}).filter(([error]) => error !== DEADLINE),
  ];
  const answered = Object.values(json.statusCodeDistribution ?? {}).reduce((a, b) => a + b, 0);
  const p99 = json.metrics?.latency_ms?.p99;
  return {
    requests: answered,
    failed: failures.reduce((n, [, count]) => n + count, 0),
    failures: failures.map(([what, count]) => `${what} x${count}`).join("; ") || null,
    p99_ms: p99 === undefined ? null : Number(p99.toFixed(1)),
  };
}

/**
 * Brings the peer's stack up fresh for one row, runs the row's probe against it, and always tears it down.
 * A peer the row gives a fixed result runs nothing, and a row that does not serve brings nothing up.
 */
async function scenarioPass(peer: Peer, s: Scenario, ctx: PeerCtx, load: Load): Promise<RowResult> {
  const fixed = s.fixed?.[peer.id];
  if (fixed !== undefined) return fixed;
  if (s.timed !== undefined) {
    const c = CASES.find((k) => k.id === s.case)!;
    const loadOf = { ...load, concurrency: c.concurrency };
    return await s.timed({
      peer: peer.id,
      requests: load.requests,
      concurrency: c.concurrency,
      pass: async (cpus) => (await onePass(peer, c, { ...ctx, cpus, concurrency: c.concurrency, tree: "cases" }, loadOf)).result!,
    });
  }
  if (s.copy === true) {
    rmSync(join(GENERATED, s.case), { recursive: true, force: true });
    cpSync(join(COMPARE_DIR, s.tree, s.case), join(GENERATED, s.case), { recursive: true });
    ctx = { ...ctx, tree: "generated" };
  }
  const env = stackEnv(peer, ctx);
  const stack = stackPath(peer);
  peer.prepare?.(ctx);
  try {
    if (s.serves !== false) up(stack, env, "app", "nginx");
    return await s.probe!({
      peer: peer.id,
      dir: join(COMPARE_DIR, ctx.tree, ctx.case),
      exec: (command) => {
        const done = compose(stack, ["exec", "-T", "app", ...command], env, true);
        return { code: done.code, output: done.stdout + done.stderr };
      },
      get: (path, timeoutMs) => replyOf(peer, path, "GET", {}, timeoutMs),
      logs: () => logsOf(stack, "app", env),
      steady: (path, seconds) => steadyOf(stack, env, path, seconds),
      send: (method, path, headers) => replyOf(peer, path, method, headers),
      running: () => {
        const ps = compose(stack, ["ps", "--status", "running", "--services"], env, true);
        return ps.code === 0 && ps.stdout.split(/\r?\n/).includes("app");
      },
      check: (file) => {
        const done = compose(stack, ["run", "--rm", "--no-deps", "--build", "app", "check", `/app/${file}`], env, true);
        return { code: done.code, output: done.stdout + done.stderr };
      },
    });
  } catch (e) {
    throw new Fail(`${peer.label}, row ${s.id}: ${messageOf(e)}\n${peer.id} said:\n${logsOf(stack, "app", env)}`);
  } finally {
    down(stack, env);
  }
}

// ---------------------------------------------------------------------------------------------------
// The command.
// ---------------------------------------------------------------------------------------------------

const USAGE =
  "usage: nv bench-compare [-h] [--smoke] [--scenarios] [--arm {fair,deployed}] [--backend-cpus N]\n" +
  "                        [--row ID] [--requests REQUESTS] [--reps REPS] [--record PATH] [--down]";

interface Options {
  smoke: boolean;
  scenarios: boolean;
  arm: "fair" | "deployed";
  backendCpus: number | null;
  requests: number | null;
  reps: number | null;
  record: string | null;
  down: boolean;
  row: string | null;
}

function options(args: string[]): Options | null {
  const p = parseArgs(args, { flags: ["--smoke", "--scenarios", "--down"], valued: ["--arm", "--backend-cpus", "--requests", "--reps", "--record", "--row"] });
  if (p.flags.has("--help")) return null;
  const int = (o: string): number | null => {
    const word = p.values.get(o);
    if (word === undefined) return null;
    const n = pyInt(word);
    if (n === null) throw new ArgError(`argument ${o}: invalid int value: ${pyRepr(word)}`);
    return n;
  };
  const arm = p.values.get("--arm") ?? "fair";
  if (arm !== "fair" && arm !== "deployed") throw new ArgError(`argument --arm: invalid choice: ${pyRepr(arm)} (choose from 'fair', 'deployed')`);
  return {
    smoke: p.flags.has("--smoke"),
    scenarios: p.flags.has("--scenarios"),
    arm,
    backendCpus: int("--backend-cpus"),
    requests: int("--requests"),
    reps: int("--reps"),
    record: p.values.get("--record") ?? null,
    down: p.flags.has("--down"),
    row: p.values.get("--row") ?? null,
  };
}

function help(): string {
  return [
    USAGE,
    "",
    "Novis, PHP-FPM, Bun and Go, each behind the same nginx terminating TLS 1.3, in containers.",
    "",
    "options:",
    "  -h, --help            show this help message and exit",
    "  --smoke               every peer and case at a few hundred requests: check the answers agree,",
    "                        print one line per case, record nothing",
    "  --scenarios           run the feature rows instead of the cases: every row against every peer,",
    "                        one line per row with each peer's result, record nothing",
    "  --arm {fair,deployed}",
    "                        fair: one cpu to each backend. deployed: --backend-cpus to each backend",
    "  --backend-cpus N      the CPU budget of EACH backend container (default 1 for fair, 4 for deployed)",
    "  --requests REQUESTS   requests per timed pass (default 20000, 300 with --smoke)",
    "  --reps REPS           timed passes per arm, best wins (default 3, 1 with --smoke)",
    "  --record PATH         write this run to PATH as one JSON object",
    "  --row ID              with --scenarios, run only the row ID",
    "  --down                tear every peer's stack down and stop",
    "",
    "benches/compare/README.md is the design; this tool is only how it is run. It needs bun and Docker.",
  ].join("\n");
}

/** Connections a probed row's stack is sized for: a probe puts no load on the backend. A timed row uses its case's. */
const SCENARIO_CONCURRENCY = 8;

/** `--scenarios`: every row against every peer, one fresh stack per pair, one line per row. */
async function scenarios(o: Options, cpus: number, commit: string, load: Load): Promise<number> {
  if (o.record !== null) throw new Fail("--scenarios records nothing; drop --record or --scenarios");
  const rows = SCENARIOS.filter((s) => o.row === null || s.id === o.row);
  if (rows.length === 0) throw new Fail(`--row ${o.row}: no such row; the rows are ${SCENARIOS.map((s) => s.id).join(", ")}`);
  for (const s of rows) {
    const byPeer = new Map<string, RowResult>();
    for (const peer of PEERS) {
      console.log(`  ${s.id}: ${peer.label} ...`);
      byPeer.set(peer.id, await scenarioPass(peer, s, { cpus, concurrency: SCENARIO_CONCURRENCY, tree: s.tree, case: s.case, commit, memory: s.memory }, load));
    }
    console.log(rowLine(s.id, byPeer));
    for (const peer of PEERS) {
      const evidence = Object.entries(byPeer.get(peer.id)!.evidence).map(([k, v]) => `${k} ${v}`);
      console.log(`    ${peer.label.padEnd(6)} ${evidence.join(", ")}`);
    }
  }
  return 0;
}

const mib = (bytes: number | null) => (bytes === null ? "    -" : fixed(bytes / 1048576, 1).padStart(6));

async function main(o: Options): Promise<number> {
  requireDocker();
  const commit = version("git", ["rev-parse", "--short", "HEAD"]);

  if (o.down) {
    for (const peer of PEERS) down(stackPath(peer), stackEnv(peer, { cpus: 1, concurrency: 1, tree: "cases", case: CASES[0]!.id, commit }));
    console.log("every peer's stack is down");
    return 0;
  }

  const cpus = o.backendCpus ?? (o.arm === "fair" ? 1 : 4);
  if (o.arm === "fair" && cpus !== 1) throw new Fail(`--arm fair is one cpu to each backend; --backend-cpus ${cpus} is the deployed arm and must say so`);
  const requests = o.requests ?? (o.smoke ? 300 : 20_000);
  const reps = o.reps ?? (o.smoke ? 1 : 3);
  if (requests < 1 || reps < 1) throw new Fail("--requests and --reps must each be at least 1");
  if (o.smoke && o.record !== null) throw new Fail("--smoke records nothing; drop --record or --smoke");

  ensureCert();
  if (o.scenarios) return await scenarios(o, cpus, commit, { requests, concurrency: SCENARIO_CONCURRENCY, reps });
  const ctxOf = (c: Case): PeerCtx => ({ cpus, concurrency: c.concurrency, tree: "cases", case: c.id, commit });

  // Agreement before any timing. `--smoke` checks it on the timed pass itself, since it records nothing.
  if (!o.smoke) {
    for (const c of CASES) {
      const answers = new Map<string, Answer[]>();
      for (const peer of PEERS) {
        console.log(`  ${c.id}: checking ${peer.label}'s answers ...`);
        answers.set(peer.id, (await onePass(peer, c, ctxOf(c), null)).answers);
      }
      const bad = agreement(c, answers);
      if (bad !== null) throw new Fail(`DIFF in ${c.id}: ${bad.peer} disagrees on input ${bad.input + 1}: ${bad.why}. No number is taken.`);
      console.log(`${c.id}: ${PEERS.length} peers agree`);
    }
  }

  const results = new Map<string, Map<string, PeerResult>>();
  for (const c of CASES) {
    const answers = new Map<string, Answer[]>();
    const byPeer = new Map<string, PeerResult>();
    for (const peer of PEERS) {
      console.log(`  ${c.id}: ${peer.label}, ${requests} requests over ${c.concurrency} connection(s), ${reps} rep(s) ...`);
      const pass = await onePass(peer, c, ctxOf(c), { requests, concurrency: c.concurrency, reps });
      answers.set(peer.id, pass.answers);
      byPeer.set(peer.id, pass.result!);
    }
    const bad = agreement(c, answers);
    if (bad !== null) throw new Fail(`DIFF in ${c.id}: ${bad.peer} disagrees on input ${bad.input + 1}: ${bad.why}. No number stands.`);
    results.set(c.id, byPeer);
    if (o.smoke) {
      console.log(`${c.id}: ${PEERS.length} peers agree`);
      continue;
    }
    const best = Math.max(...[...byPeer.values()].map((r) => r.requests_per_sec));
    console.log(`${c.id}, ${cpus} cpu per backend:`);
    for (const peer of PEERS) {
      const r = byPeer.get(peer.id)!;
      const p99 = r.p99_ms === null ? "      -" : fixed(r.p99_ms, 3).padStart(7);
      console.log(
        `  ${peer.label.padEnd(6)} ${grouped(r.requests_per_sec, 0).padStart(10)} requests/sec  ${fixed(r.requests_per_sec / best, 2)}x of best  ` +
          `p99 ${p99} ms  memory idle ${mib(r.memory_idle_bytes)} MiB, peak ${mib(r.memory_peak_bytes)} MiB  errors ${r.errors}`,
      );
    }
  }

  if (o.smoke) return 0;
  const machine = { ...host(), docker: version("docker", ["version", "--format", "{{.Server.Version}}"]) };
  const record = buildRecord({ date: stamp(), commit, arm: o.arm, backendCpus: cpus, requests, reps, machine, digests: digests() }, results);
  if (o.record === null) {
    console.log("  not recorded: pass --record PATH to write this run");
    return 0;
  }
  const full = resolve(o.record);
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, `${JSON.stringify(record, null, 2)}\n`, "utf8");
  console.log(`  recorded to ${o.record}`);
  return 0;
}

export async function run(args: string[]): Promise<number> {
  let o: Options | null;
  try {
    o = options(args);
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv bench-compare: error: ${e.message}`);
    return 2;
  }
  if (o === null) {
    console.log(help());
    return 0;
  }
  try {
    return await main(o);
  } catch (e) {
    if (!(e instanceof Fail)) throw e;
    console.error(e.message);
    return 1;
  }
}
