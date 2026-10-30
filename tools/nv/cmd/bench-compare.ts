// `bun nv bench-compare`: Novis, PHP-FPM, Bun and Go, each behind the same nginx terminating TLS 1.3.
//
//     bun nv bench-compare --smoke                               # every peer and case, a few hundred requests
//     bun nv bench-compare --scenarios --smoke                   # every feature row against every peer
//     bun nv bench-compare --scenarios --smoke --row config      # one row
//     bun nv bench-compare --scenarios --row loop --expect nvs=on-by-default   # exits 1 on another result
//     bun nv bench-compare --scenarios --smoke --row flood       # the nightly deep lane's run
//     bun nv bench-compare --record benches/compare.json         # the fair arm, recorded
//     bun nv bench-compare --scenarios --record benches/compare.json   # the feature rows, into that record
//     bun nv bench-compare --arm deployed --record PATH          # four cpus to each backend
//     bun nv bench-compare --down                                # tear every stack down
//     bun nv bench-compare --profile json-api                    # perf and strace on Novis serving one case
//
// `benches/compare/README.md` is why the comparison is built the way it is, and this file owns only how a
// run is carried out. The Docker half is `tools/nv/cmd/bench-docker.ts`'s, shared with `bench-proxied`.
//
// # What a run does
//
// The peers are data: `PEERS` names each one's compose file, nginx config, published port, image and the
// settings written into its rows, and a fifth peer is a new entry and a compose file. One stack is up at a
// time, and every (case, rep, peer) gets a fresh one, so `memory.peak` is that pass's alone. The reps
// interleave the peers, and a peer's figures are the median rep's (`medianOf`). A full run first brings
// every pair up only to check that the peers answer alike, and times nothing until they do.
// `--smoke` is one pass at a few hundred requests that checks the answers and records nothing.
//
// `--scenarios` runs the feature rows in `SCENARIOS` instead of the cases. A row brings each peer's stack
// up fresh over the row's own programs, runs its probe, and maps what the peer did to one of the four
// results in `Verdict`, with the evidence beside it.
//
// `--profile CASE` measures no peer. It builds the Novis image, copies `oha` out of its pinned image, and
// runs `benches/compare/profile.sh` in a privileged container of the Novis image, with no nginx: `nvs
// serve` on one pinned core, `oha` on others, the top self symbols of `perf record -F 1999` over 15 s, the
// Novis functions with their callees from a call-graph record over 10 s, and the system calls per request
// from `strace -c`. The `wait` case first prints the server cgroup's
// `memory.peak` minus its idle `memory.current`, per request in flight, over two loads of 1000. When
// `perf` cannot record in a container on this machine it exits 1, and `profile.sh` is the recipe by hand.

import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { rel, ROOT } from "../lib/paths.ts";
import { ArgError, fixed, parseArgs, pyInt, pyRepr } from "../lib/py.ts";
import { Fail, grouped, host, stamp, version } from "./bench.ts";
import { type Arm, compose, down, type Load, logsOf, messageOf, ohaArm, requireDocker, up } from "./bench-docker.ts";
import { effectivePhp, PHP_INI } from "./bench-proxied.ts";

export const summary = "Novis, PHP-FPM, Bun and Go behind one TLS nginx, in containers: nv bench-compare [--smoke] [--scenarios] [--arm fair|deployed] [--record PATH] [--profile CASE]";

export const COMPARE_DIR = join(ROOT, "benches", "compare");
const GENERATED = join(COMPARE_DIR, "generated");
const TLS_DIR = join(GENERATED, "tls");
/** What `--profile` mounts at `/profile`: `oha` and the timed request's body. */
const PROFILE_DIR = join(GENERATED, "profile");
export const PROFILE_SCRIPT = join(COMPARE_DIR, "profile.sh");
/** The image `compose.nvs.yaml` builds, which `--profile` runs with `perf` and `strace` added. */
export const NVS_IMAGE = "nvs-bench-serve:local";

/** The ini `compose.php.yaml` mounts after the proxied leg's, which turns PHP's file check back on at its defaults. */
export const FILE_CHECK_INI = join(COMPARE_DIR, "php", "file-check.ini");

export const NGINX_IMAGE = "nginx:1.31.8-alpine";
export const OHA_IMAGE = "ghcr.io/hatoo/oha@sha256:3ec3dbf549ea197793482d47a6324797411406bbf438c2fe8b91f244ec641a2f";
const CERT_IMAGE = "alpine/openssl:3.5.9";
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
  /** The folder in the container the backend serves its program from. Unset is `/app`. */
  appDir?: string | undefined;
  /**
   * The backend serves the `site` volume, mounted read-write, which `deploy/deploy.sh` writes from inside
   * the container. Unset serves `case`'s folder, mounted read-only.
   */
  site?: boolean | undefined;
  /** The part count of a deploy row, which the PHP, Bun and Go programs read from `DEPLOY_PARTS`. */
  parts?: number | undefined;
  /** An nginx config under `nginx/` in place of the peer's own, for a row that serves differently. */
  nginx?: string | undefined;
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
  /** The program is compiled into the image from the committed tree, so a row's copy is never served. */
  compiled?: boolean;
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
    image: "php:8.5.12-fpm",
    env: () => ({}),
    settings: (ctx) => ({ server: "php-fpm", pm: "static", "pm.max_children": phpChildren(ctx.concurrency), ...effectivePhp([PHP_INI, FILE_CHECK_INI]) }),
    prepare: (ctx) => renderPool(phpChildren(ctx.concurrency)),
  },
  {
    id: "bun",
    label: "Bun",
    stack: "compose.bun.yaml",
    nginx: "http.conf",
    port: 18443,
    image: "oven/bun:1.4.4-slim",
    env: () => ({}),
    settings: (ctx) => ({ server: "Bun.serve", processes: ctx.cpus, reusePort: true }),
  },
  {
    id: "go",
    label: "Go",
    stack: "compose.go.yaml",
    nginx: "http.conf",
    port: 18444,
    image: "golang:1.27.3-alpine, run on alpine:3.24.2",
    env: () => ({}),
    settings: (ctx) => ({ server: "net/http", GOMAXPROCS: ctx.cpus }),
    compiled: true,
  },
];

/** Every variable a peer's compose file reads, so `up`, `exec`, `logs` and `down` all see the same ones. */
export function stackEnv(peer: Peer, ctx: PeerCtx): Record<string, string> {
  return {
    TREE: ctx.tree,
    CASE: ctx.case,
    BACKEND_CPUS: String(ctx.cpus),
    BACKEND_MEMORY: ctx.memory ?? "0",
    APP_SOURCE: ctx.site === true ? "site" : `./${ctx.tree}/${ctx.case}`,
    APP_DIR: ctx.appDir ?? "/app",
    APP_MOUNT_MODE: ctx.site === true ? "rw" : "ro",
    NGINX_CONF: ctx.nginx ?? peer.nginx,
    NGINX_PORT: String(peer.port),
    ...(ctx.parts === undefined ? {} : { DEPLOY_PARTS: String(ctx.parts) }),
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

/**
 * What a row says of one peer: the feature is there with no setting, there through a setting the peer
 * starts with, one setting away, code the developer writes, or absent.
 */
export type Verdict = "built-in" | "on-by-default" | "one-setting" | "your-code" | "no";

/** Every `Verdict`, in the page's order. */
export const VERDICTS: readonly Verdict[] = ["built-in", "on-by-default", "one-setting", "your-code", "no"];

/** One peer's result in one row, with the measured evidence beside it. */
export interface RowResult {
  result: Verdict;
  /** The setting an `on-by-default` or `one-setting` result names, or the command a `one-setting` one needed. */
  setting?: string;
  evidence: Record<string, string | number | boolean | null>;
}

/** A setting a row's feature depends on, and whether the peer starts with it on. */
export interface Setting {
  name: string;
  default: boolean;
}

/** A feature that worked: `built-in` with no setting, else `on-by-default` or `one-setting` naming it. */
export function passed(setting: Setting | undefined, evidence: RowResult["evidence"]): RowResult {
  if (setting === undefined) return { result: "built-in", evidence };
  return { result: setting.default ? "on-by-default" : "one-setting", setting: setting.name, evidence };
}

/** A setting a row writes before its probe, which the peer does not start with. */
const written = (edit: TableEdit | undefined): Setting | undefined => (edit === undefined ? undefined : { name: edit.name, default: false });

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
  /** Whether the backend container runs, and how often its restart policy has started it again. */
  backend(): Backend;
  /** `nvs check` over a file of the row's folder, in a one-off app container: the exit code and what it printed. */
  check(file: string): { code: number; output: string };
  /** A command run in the backend container, as an operator would run it there. */
  exec(command: string[]): { code: number; output: string };
  /** Restarts the backend container, as a deploy that needs a new process does. */
  restart(): { code: number; output: string };
  /**
   * A deploy step, a command in the backend container or its restart, run without blocking. The deploy
   * rows' own requests come from this process, so a blocking step would pause them over the very window
   * it is meant to show.
   */
  step(run: string[] | "restart"): Promise<{ code: number; output: string }>;
  /** The backend's last log lines. */
  logs(): string;
  /**
   * `oha` against `path` from inside the stack for `seconds`, while the row sends its own requests, at
   * `connections` or else the row's connections.
   */
  steady(path: string, seconds: number, connections?: number): Promise<Steady>;
  /** The peer's `id` in `PEERS`. */
  peer: string;
  /** The peer's program is compiled into its image, so `dir` is not what it serves. */
  compiled: boolean;
  /** The host folder the stack serves, which a row that edits the program writes to. */
  dir: string;
}

/**
 * The backend container after a row: whether it runs, and how often its restart policy started it again.
 * Every stack restarts a backend that exits, as a deployment's supervisor does, so a crash costs the
 * requests in flight and the restart, and the row counts it.
 */
export interface Backend {
  running: boolean;
  restarts: number;
}

/** A backend that runs and was never started again. */
export const survived = (b: Backend) => b.running && b.restarts === 0;

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
  /** The row's case at a backend budget of `cpus`: `--reps` passes, each on a fresh stack, as `medianOf` merges them. */
  pass(cpus: number): Promise<PeerResult>;
}

export interface Scenario {
  id: string;
  /** The folder under `benches/compare/` and the program folder in it the stack serves. */
  tree: string;
  case: string;
  /** `false` for a row that only checks a program, so no stack is brought up. */
  serves?: boolean;
  /** Serve a fresh copy of the folder under `generated/`, for a row that edits the program from the host. */
  copy?: boolean;
  /** The backend container's memory cap, for a row whose program allocates without end. */
  memory?: string;
  /** The backend's cpus, for a row that needs more than the run's budget. */
  cpus?: number;
  /** The request a timed row sends to the row's own program, where it does not time a case in `cases/`. */
  timedCase?: Case;
  /** The folder in the container the backend serves, and the deploy part count, for a deploy row. */
  appDir?: string;
  parts?: number;
  /**
   * A deploy row's layout. The backend serves the `site` volume, and `deploy/deploy.sh init` writes
   * version 1 into it in a one-off container before the stack comes up. A compiled peer runs no init.
   */
  site?: DeployLayout;
  /** An nginx config per peer id in place of the peer's own. */
  nginx?: Record<string, string>;
  /**
   * Fresh stacks the row runs against each peer, one probe each, merged by `merge`. The runs interleave
   * the peers (run 1 of every peer, then run 2), so a slow stretch of the shared host lands on all of them.
   * `--smoke` runs one.
   */
  runs?: number;
  merge?(peer: string, results: RowResult[]): RowResult;
  /** The result of a peer that has nothing to run for this row, with the documentation that says so. */
  fixed?: Record<string, RowResult>;
  /** A row about requests: sent to one stack the driver brings up. */
  probe?(p: Probe): Promise<RowResult>;
  /** A row about a figure: times `timedCase`, or else the case in `cases/` named by `case`, each pass on a fresh stack. */
  timed?(t: Timer): Promise<RowResult>;
}

/** The factor from one cpu to four below which a peer is not using the extra cores. */
export const CORES_SCALING = 2;

/**
 * Turns of the parallel program's loop in one request of the cores row: about a millisecond of one core
 * on every peer, so four cores answer a few thousand requests a second and the TLS front, which answers
 * tens of thousands, is never what the row measures.
 */
export const CORES_TURNS = 300_000;

/** The cores row's request: one part of the parallel program's loop, which every peer runs in the request itself. */
export const CORES_CASE: Case = {
  id: "cores",
  concurrency: 64,
  timed: { method: "GET", path: `/?parts=1&turns=${CORES_TURNS}` },
  inputs: [{ method: "GET", path: `/?parts=1&turns=${CORES_TURNS}` }],
  html: false,
};

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
export const WAITING_SETTING: Record<string, Setting> = { php: { name: "pm.max_children", default: false } };

/**
 * Every request answered `200` passes, naming `WAITING_SETTING` where the peer needs it. A failed request
 * is `no`. The memory each request in flight costs is the evidence beside it: `memory.peak` less the idle
 * `memory.current`, over the requests that can be in flight at once.
 */
export function waitingResult(r: PeerResult, inFlight: number, setting: Setting | undefined): RowResult {
  const failed = r.errors + Object.entries(r.status_codes).reduce((n, [code, count]) => (code === "200" ? n : n + count), 0);
  const kib = r.memory_peak_bytes === null || r.memory_idle_bytes === null ? null : Number(((r.memory_peak_bytes - r.memory_idle_bytes) / inFlight / 1024).toFixed(1));
  const failures = [...Object.entries(r.status_codes).filter(([code]) => code !== "200"), ...Object.entries(r.error_kinds)];
  const evidence = {
    in_flight: inFlight,
    kib_per_request: kib,
    failed_requests: failed,
    failures: failures.map(([what, count]) => `${what} x${count}`).join("; ") || null,
  };
  if (failed > 0) return { result: "no", evidence };
  return passed(setting, evidence);
}

/** How long the loop and memory rows' steady load runs, and how long after the load starts the runaway request is sent. */
export const RUNAWAY_LOAD_SECONDS = 15;
/** How long the unmeasured load before the baseline runs, so the baseline and the runaway load both start warm. */
export const RUNAWAY_WARMUP_SECONDS = 5;
/** Fresh stacks each runaway row runs against each peer, each with its own baseline and runaway load. */
export const RUNAWAY_RUNS = 3;
export const RUNAWAY_START_MS = 3_000;

/** How long a runaway request may take to be stopped: under nginx's 60-second read timeout, so a reply is the backend's. */
export const RUNAWAY_WAIT_MS = 55_000;

/** The backend container's memory cap in the loop, memory and flood rows, so an allocation without end stops there and not at the host. */
export const RUNAWAY_MEMORY = "512m";

/** The statuses nginx gives when the backend did not answer: a reply with one of them is not the backend stopping the request. */
const NGINX_STATUSES = new Set([502, 504]);

/** A runaway request's reply: its status and the seconds it took, or `null` for both when it was never answered. */
export interface Stopped {
  status: number | null;
  seconds: number | null;
}

/**
 * Passes when the steady load saw no failed request, the runaway request was stopped by the backend, and
 * the backend ran throughout, naming the setting that stopped it where the peer has one. Anything else is
 * `no`, or `your-code` naming `inCode` where the peer's own code can stop the request although the
 * program as its documentation writes it does not. `baseline` is the same load on the same stack with no
 * runaway request, so the page can show the share of its own throughput each peer kept rather than how
 * fast it is anyway.
 */
export function runawayResult(baseline: Steady, load: Steady, runaway: Stopped, backend: Backend, setting: Setting | undefined, inCode?: string): RowResult {
  const stopped = runaway.status !== null && !NGINX_STATUSES.has(runaway.status);
  const evidence = {
    baseline_requests: baseline.requests,
    requests: load.requests,
    failed_requests: load.failed,
    failures: load.failures,
    p99_ms: load.p99_ms,
    runaway_status: runaway.status,
    runaway_seconds: runaway.seconds,
    backend_running: backend.running,
    backend_restarts: backend.restarts,
    runs: 1,
    code: inCode ?? null,
  };
  if (load.failed > 0 || !survived(backend) || !stopped) return { result: inCode === undefined ? "no" : "your-code", evidence };
  return passed(setting, evidence);
}

/** `502 x12; timeout x3` as counts by kind, the inverse of `failureText`. */
export function failureCounts(text: unknown): Map<string, number> {
  const counts = new Map<string, number>();
  if (typeof text !== "string") return counts;
  for (const part of text.split("; ")) {
    const m = /^(.*) x(\d+)$/.exec(part);
    if (m !== null) counts.set(m[1]!, (counts.get(m[1]!) ?? 0) + Number(m[2]));
  }
  return counts;
}

/** Counts by kind as `502 x12; timeout x3`, or `null` for none. */
export const failureText = (counts: Map<string, number>): string | null => [...counts].map(([what, n]) => `${what} x${n}`).join("; ") || null;

/** The `failures` texts of several runs as one, each kind's counts summed. */
function sumFailures(texts: unknown[]): string | null {
  const all = new Map<string, number>();
  for (const text of texts) for (const [what, n] of failureCounts(text)) all.set(what, (all.get(what) ?? 0) + n);
  return failureText(all);
}

/** The middle of `values` once sorted, the lower of the two middle ones for an even count, so it is always a measured value. */
export function medianIndex(values: number[]): number {
  const order = values.map((_, i) => i).sort((a, b) => values[a]! - values[b]!);
  return order[Math.floor((order.length - 1) / 2)]!;
}

/**
 * The runaway rows' runs as one: the load and the runaway request are the median run's, by the share of
 * its own baseline the load kept, and the failures and restarts are summed over every run. One run that
 * is `no` makes the row `no`.
 */
export function mergeRunaways(_peer: string, results: RowResult[]): RowResult {
  const e = results.map((r) => r.evidence);
  const mid = e[medianIndex(e.map((x) => Number(x.requests) / Math.max(1, Number(x.baseline_requests))))]!;
  const evidence = {
    baseline_requests: mid.baseline_requests ?? null,
    requests: mid.requests ?? null,
    failed_requests: e.reduce((n, x) => n + Number(x.failed_requests), 0),
    failures: sumFailures(e.map((x) => x.failures)),
    p99_ms: mid.p99_ms ?? null,
    runaway_status: mid.runaway_status ?? null,
    runaway_seconds: mid.runaway_seconds ?? null,
    backend_running: e.every((x) => x.backend_running === true),
    backend_restarts: e.reduce((n, x) => n + Number(x.backend_restarts), 0),
    runs: e.reduce((n, x) => n + Number(x.runs ?? 1), 0),
    code: mid.code ?? null,
  };
  if (results.some((r) => r.result === "no")) return { result: "no", evidence };
  if (results.some((r) => r.result === "your-code")) return { result: "your-code", evidence };
  const first = results[0]!;
  return first.setting === undefined ? { result: first.result, evidence } : { result: first.result, setting: first.setting, evidence };
}

/**
 * The limit each peer starts with that stops the loop row's and the memory row's runaway request. Both
 * rows run every peer at its defaults. Novis's `[limits]` and PHP's ini stop a request at 128 MB or 30
 * seconds of CPU time unless a setting turns the limit off. Bun and Go have no such limit.
 */
export const LOOP_DEFAULT: Record<string, Setting> = {
  nvs: { name: "[limits] cpu_time", default: true },
  php: { name: "max_execution_time", default: true },
};
export const MEMORY_DEFAULT: Record<string, Setting> = {
  nvs: { name: "[limits] memory", default: true },
  php: { name: "memory_limit", default: true },
};

/**
 * The code a peer's program can carry to stop the loop row's runaway request, where the peer has a way.
 * Bun's `Worker.terminate()` stops a worker that never yields within milliseconds, so a program that
 * runs each request in a worker under a time limit stops it. Bun ignores a worker's `resourceLimits`, so
 * nothing of the kind exists for the memory row, and a goroutine cannot be stopped from outside at all.
 */
export const LOOP_IN_CODE: Record<string, string> = { bun: "each request in a Worker, and terminate() after a time limit" };

/** How long the flood row's load runs. */
export const FLOOD_SECONDS = 10;
/**
 * The flood row's connections, each sending `/hold` without pause. Every hold keeps 8 MiB for 300 ms, so
 * 200 of them want 1.6 GB, three times the backend's `RUNAWAY_MEMORY`.
 */
export const FLOOD_CONNECTIONS = 200;
/** How often the flood row sends `/` while the load runs. */
export const FLOOD_PROBE_EVERY_MS = 250;
/** How long one of those probes may take before it counts as a probe that hung. */
export const FLOOD_PROBE_WAIT_MS = 10_000;
/** The holds sent together once `/` answers again: every one must answer `200`. 64 MiB, far under the backend's cap. */
export const FLOOD_AFTER = 8;

/** One probe the flood row sent: its status and whether it carried `Retry-After`, or `null` when it hung. */
export interface FloodProbe {
  status: number | null;
  ms: number;
  retryAfter: boolean;
}

/** A probe that tells the client what happened: `200`, or the backend's `503` with `Retry-After`. */
export const probeHeld = (p: FloodProbe) => p.status === 200 || (p.status === 503 && p.retryAfter);

/**
 * Passes when every probe sent during the flood was answered in time with `200` or a `503` that carries
 * `Retry-After`, `/` answered `200` again after the load, every hold sent after it was answered, and the
 * backend ran throughout. A peer that bounds its load through a setting names it. Anything else is `no`.
 */
export function floodResult(flood: Steady, probes: FloodProbe[], recovery: number | null, after: number, backend: Backend, setting: Setting | undefined): RowResult {
  const ms = probes.filter((p) => p.status !== null).map((p) => p.ms).sort((a, b) => a - b);
  const evidence = {
    flood_requests: flood.requests,
    refused: failureCounts(flood.failures).get("503") ?? 0,
    flood_failures: flood.failures,
    probes: probes.length,
    probes_failed: probes.filter((p) => !probeHeld(p)).length,
    probe_median_ms: ms.length === 0 ? null : Math.round(ms[medianIndex(ms)]!),
    probe_max_ms: ms.length === 0 ? null : Math.round(ms[ms.length - 1]!),
    recovery_seconds: recovery,
    after_ok: after,
    backend_running: backend.running,
    backend_restarts: backend.restarts,
  };
  if (evidence.probes_failed > 0 || recovery === null || after < FLOOD_AFTER || !survived(backend)) return { result: "no", evidence };
  return passed(setting, evidence);
}

/** The setting that bounds the flood on a peer whose bound is one. Novis's admission has none. */
export const FLOOD_SETTING: Record<string, Setting> = { php: { name: "pm.max_children", default: false } };

/**
 * Keys a row sets in one table of the Novis data folder's `nvs.toml`. The shipped file has each key
 * commented out inside its table, and the same key may be commented out in another table too, so the
 * edit uncomments it only between the table's header and the next one.
 */
export interface TableEdit {
  table: string;
  set: Record<string, string>;
  /** The setting's name in the row. */
  name: string;
}

/** The command that makes `edit` inside the Novis container. */
export function tableEditCommand(edit: TableEdit): string[] {
  const header = edit.table.replaceAll(".", "\\.");
  const subs = Object.entries(edit.set).map(([key, value]) => `s|^#${key} = .*|${key} = ${value}|`);
  return ["sed", "-i", `/^\\[${header}\\]$/,/^\\[/{${subs.join(";")}}`, "/usr/local/bin/.nvsdata/nvs.toml"];
}

/** Makes `edit` in the running Novis container and waits until `nvs serve` has reloaded it. */
async function applyTableEdit(p: Probe, edit: TableEdit): Promise<void> {
  const command = tableEditCommand(edit);
  const wrote = p.exec(command);
  if (wrote.code !== 0) throw new Fail(`${command.join(" ")}: ${wrote.output}`);
  if ((await logUntil(p, "configuration reloaded")) === null) throw new Fail(`${edit.name} was written, and the configuration was not reloaded`);
}

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

/**
 * Puts steady load on `/` for `RUNAWAY_WARMUP_SECONDS` and discards it, then once more with nothing else
 * running, then again with the runaway request at `path` sent into it, and judges what the last saw. A
 * peer with a limit at its defaults names it from `defaults`. Without the warm-up the baseline is the
 * first load a fresh server gets, so it pays for opcache, the JITs and new FPM workers, and a peer could
 * answer more requests with the runaway request than without it.
 */
async function runawayProbe(p: Probe, path: string, defaults: Record<string, Setting>, inCode: Record<string, string> = {}): Promise<RowResult> {
  ok(await p.get("/"), "the first request");
  await p.steady("/", RUNAWAY_WARMUP_SECONDS);
  const baseline = await p.steady("/", RUNAWAY_LOAD_SECONDS);
  const load = p.steady("/", RUNAWAY_LOAD_SECONDS);
  await Bun.sleep(RUNAWAY_START_MS);
  const runaway = stoppedAt(p, path);
  const steady = await load;
  return runawayResult(baseline, steady, await runaway, p.backend(), defaults[p.peer], inCode[p.peer]);
}

/** One `GET /` of the flood row, timed, given up after `FLOOD_PROBE_WAIT_MS`. */
async function floodProbeOnce(p: Probe): Promise<FloodProbe> {
  const start = performance.now();
  try {
    const reply = await p.get("/", FLOOD_PROBE_WAIT_MS);
    return { status: reply.status, ms: performance.now() - start, retryAfter: reply.headers["retry-after"] !== undefined };
  } catch {
    return { status: null, ms: performance.now() - start, retryAfter: false };
  }
}

/**
 * Sends `/hold` from `FLOOD_CONNECTIONS` connections for `FLOOD_SECONDS`, and `/` every
 * `FLOOD_PROBE_EVERY_MS` beside it. Once the load ends, waits up to `CHANGE_WAIT_MS` for `/` to answer `200`, then sends
 * `FLOOD_AFTER` holds together, and judges all of it.
 */
async function floodProbe(p: Probe): Promise<RowResult> {
  ok(await p.get("/"), "the first request");
  ok(await p.get("/hold"), "the first hold");
  let flooding = true;
  const load = p.steady("/hold", FLOOD_SECONDS, FLOOD_CONNECTIONS).finally(() => {
    flooding = false;
  });
  const probes: FloodProbe[] = [];
  while (flooding) {
    probes.push(await floodProbeOnce(p));
    await Bun.sleep(FLOOD_PROBE_EVERY_MS);
  }
  const flood = await load;
  const recovery = await secondsUntil(p, (r) => r.status === 200);
  let after = 0;
  if (recovery !== null) {
    const replies = await Promise.all(Array.from({ length: FLOOD_AFTER }, () => p.get("/hold").catch(() => null)));
    after = replies.filter((r) => r?.status === 200).length;
  }
  return floodResult(flood, probes, recovery, after, p.backend(), FLOOD_SETTING[p.peer]);
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

/** The request after a crash must be answered by a backend that never stopped, not by one its restart policy started again. */
export function crashResult(crashStatus: number, nextStatus: number, backend: Backend): RowResult {
  return {
    result: nextStatus === 200 && survived(backend) ? "built-in" : "no",
    evidence: { crash_status: crashStatus, next_status: nextStatus, backend_running: backend.running, backend_restarts: backend.restarts },
  };
}

/** The setting that sends each peer's security headers, where it has one. Novis sends its set with nothing written. */
export const HEADERS_SETTING: Record<string, Setting> = { nvs: { name: "[http.headers]", default: true } };

/** All three headers pass, naming the peer's `HEADERS_SETTING`. One missing is the developer's to write. */
export function headersResult(headers: Record<string, string>, setting: Setting | undefined): RowResult {
  const nosniff = (headers["x-content-type-options"] ?? "").trim().toLowerCase() === "nosniff";
  const framePolicy = headers["x-frame-options"] !== undefined || /frame-ancestors/i.test(headers["content-security-policy"] ?? "");
  const referrerPolicy = headers["referrer-policy"] !== undefined;
  const evidence = { nosniff, frame_policy: framePolicy, referrer_policy: referrerPolicy };
  return nosniff && framePolicy && referrerPolicy ? passed(setting, evidence) : { result: "your-code", evidence };
}

/** The term the escape row prints. A body that carries it raw would run it in a browser. */
export const ESCAPE_TERM = "<script>alert(1)</script>";

/**
 * `template` is the body the documented template wrote, `direct` the one the plainest write to the
 * response wrote. Built in only when both arrive escaped, so no way to print the term skips the
 * escape. Otherwise the developer has to choose the safe way or call an escaper, which is `your-code`.
 */
export function escapeResult(template: string, direct: string): RowResult {
  const rawTemplate = template.includes("<script>");
  const rawDirect = direct.includes("<script>");
  return {
    result: rawTemplate || rawDirect ? "your-code" : "built-in",
    evidence: { raw_script_from_template: rawTemplate, raw_script_from_direct_write: rawDirect },
  };
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

/** How long the session row's `/slow` runs before its `/fast` is sent, and the wait under which `/fast` was not held. */
export const SESSION_HEAD_START_MS = 200;
export const SESSION_WAIT_LIMIT_MS = 1000;

/** The code a peer's program needs so a session it only reads does not wait for one another request writes. */
const SESSION_IN_CODE: Record<string, string> = { php: "session_start(['read_and_close' => true]) or session_write_close()" };

/** A peer with no session in its standard library, so the row has nothing to run. */
const noSession = (docs: string): RowResult => ({ result: "your-code", evidence: { session: "none in the standard library", docs } });

/**
 * Every `set-cookie` of `reply` as one `Cookie` header, the attributes dropped. A reply's headers join
 * several `set-cookie`s with `, `, so a comma followed by `name=` starts the next one.
 */
export function cookieOf(reply: Reply): string {
  const raw = reply.headers["set-cookie"];
  if (raw === undefined || raw === "") throw new Fail(`the first request set no cookie: status ${reply.status}`);
  return raw
    .split(/,(?=\s*[^;,=\s]+=)/)
    .map((c) => c.split(";")[0]!.trim())
    .join("; ");
}

/**
 * `/slow` writes the session and then works, so `/fast`, sent `SESSION_HEAD_START_MS` later with the same
 * cookie, waits for nothing it needs. Under `SESSION_WAIT_LIMIT_MS` is `built-in`. Longer is the session
 * lock holding `/fast` until `/slow` ends, which the peer's own code can avoid, so it is `your-code`.
 */
export function sessionResult(slow: Reply, fast: Reply, fastMs: number, inCode: string | undefined): RowResult {
  if (slow.status !== 200 || !slow.body.includes("slow done")) throw new Fail(`/slow: status ${slow.status}, body ${pyRepr(slow.body.slice(0, 200))}`);
  if (fast.status !== 200 || !fast.body.includes("count:")) throw new Fail(`/fast: status ${fast.status}, body ${pyRepr(fast.body.slice(0, 200))}`);
  const evidence = { fast_wait_ms: Math.round(fastMs), head_start_ms: SESSION_HEAD_START_MS, code: inCode ?? null };
  return { result: fastMs < SESSION_WAIT_LIMIT_MS ? "built-in" : "your-code", evidence };
}

async function sessionProbe(p: Probe): Promise<RowResult> {
  const cookie = cookieOf(ok(await p.get("/"), "the session start"));
  const slow = p.send("GET", "/slow", { Cookie: cookie });
  await Bun.sleep(SESSION_HEAD_START_MS);
  const start = performance.now();
  const fast = await p.send("GET", "/fast", { Cookie: cookie });
  const fastMs = performance.now() - start;
  return sessionResult(await slow, fast, fastMs, SESSION_IN_CODE[p.peer]);
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
 * An edit that shows passes, naming the peer's setting when the peer starts with it on. One that never
 * shows is `one-setting` where the peer has a setting it starts without, and `no` otherwise. The seconds
 * stay `null` in the evidence either way.
 */
export function liveEditResult(seconds: number | null, setting: Setting | undefined): RowResult {
  const evidence = { seconds_to_new_answer: seconds, restarts: 0 };
  if (seconds !== null) return passed(onByDefault(setting), evidence);
  return setting === undefined || setting.default ? { result: "no", evidence } : { result: "one-setting", setting: setting.name, evidence };
}

/** The setting each peer's documentation names for showing an edit without a restart, and whether the peer starts with it on. */
export const LIVE_EDIT_SETTING: Record<string, Setting> = {
  php: { name: "opcache.validate_timestamps", default: true },
  bun: { name: "bun --hot", default: false },
};

/** `setting` where the peer starts with it on: the setting a change of files that shows without a step names. */
export const onByDefault = (setting: Setting | undefined): Setting | undefined => (setting?.default === true ? setting : undefined);

/** The line the live-edit program prints, and the line the driver edits it to. */
export const LIVE_EDIT_FROM = "version 1";
export const LIVE_EDIT_TO = "version 2";

/** The part files of the `deploy` and `deploy-switch` trees, of the `deploy-big` tree, and the pause between two of them while the copy writes them. */
export const DEPLOY_PARTS = 200;
export const DEPLOY_BIG_PARTS = 2_000;
export const DEPLOY_PAUSE_MS = 10;

/** How long after the checking load starts the copy begins, and how many requests the load keeps in flight. */
export const DEPLOY_START_MS = 2_000;
export const DEPLOY_CLIENTS = 8;

/** How long the checking load runs on after the first reply of the new version, and how long one request may take. */
export const DEPLOY_AFTER_MS = 1_000;
export const DEPLOY_REQUEST_MS = 5_000;

/** Fresh stacks each deploy row runs against each peer. Where a mixed reply comes from depends on timing, so one copy proves little. */
export const DEPLOY_RUNS = 3;

/** The pause between the last file of a release and the link switch to it, in the `deploy-switch` row. */
export const DEPLOY_SETTLE_MS = 2_000;

/**
 * How a deploy row lays out its program: `flat` serves `/app` and replaces its parts in place, `release`
 * serves the link `/app/current` to `/app/releases/N/` and switches it in one rename.
 */
export type DeployLayout = "flat" | "release";

/**
 * Where the compose files mount `benches/compare/deploy/deploy.sh` in the backend container. The script
 * writes the deploy rows' files there, on the `site` volume, and its header says why.
 */
export const DEPLOY_SCRIPT = "/srv/deploy.sh";

/** `deploy.sh`'s arguments for one peer: the command, its part extension, count and layout, then the command's own. */
export function deployCommand(command: "init" | "copy" | "switch", ext: string, parts: number, layout: DeployLayout, ...rest: string[]): string[] {
  return [DEPLOY_SCRIPT, command, ext, String(parts), layout, ...rest];
}

/**
 * The `docker compose` arguments that write version 1 before the stack comes up: a one-off app container
 * with the row's committed folder at `/src`. `sh` is the entrypoint because Novis's image starts `nvs`.
 */
export function deployInitRun(source: string, ext: string, parts: number, layout: DeployLayout): string[] {
  return ["run", "--rm", "--no-deps", "--entrypoint", "sh", "-v", `${source}:/src:ro`, "app", ...deployCommand("init", ext, parts, layout)];
}

/** The copy step of version 2, run in the backend container, with `DEPLOY_PAUSE_MS` between two files. */
export const deployCopyStep = (ext: string, parts: number, layout: DeployLayout) =>
  ["sh", ...deployCommand("copy", ext, parts, layout, "2", String(DEPLOY_PAUSE_MS / 1000))];

/** The `deploy-switch` row's switch of `/app/current` to release 2, in one rename in the backend container. */
export const deploySwitchStep = (ext: string, parts: number) => ["sh", ...deployCommand("switch", ext, parts, "release", "2")];

/**
 * How a peer deploys a new version at its defaults once the files are in place: a command run in the
 * container, or a restart of it. A peer with no entry runs no command: PHP checks its files' timestamps
 * by default, as Novis does.
 */
export interface DeployWay {
  name: string;
  run: string[] | "restart";
}

export const DEPLOY_WAYS: Record<string, DeployWay> = {
  bun: { name: "a restart", run: "restart" },
  go: { name: "a new image and a restart", run: "restart" },
};

/** The extension of the parts each peer's program reads. Go's parts are compiled into its image. */
export const DEPLOY_EXT: Record<string, string> = { nvs: "nvs", php: "php", bun: "ts" };

/**
 * What the deploy rows' checking load saw: every reply, each failed (an exception or a status but
 * `200`), mixed or complete, and the seconds from the deploy step to the first reply of the new version.
 */
export interface DeployLoad {
  requests: number;
  failed: number;
  /** Each kind of failure and how often it came: a status, or the error a request threw. */
  failures: Record<string, number>;
  mixed: number;
  complete: number;
  seconds: number | null;
}

/** When the copy and the peer's step finished, which the load times the new version from, and whether the row gave up. */
interface Deploying {
  appliedAt: number | null;
  stopped?: boolean;
}

/** A body of part versions holds more than one version: more than one distinct character once trimmed. */
export const mixedVersions = (body: string) => new Set(body.trim()).size > 1;

/** The evidence a deploy row records, one run's or the sum of several. */
export interface DeployEvidence {
  requests: number;
  failed_requests: number;
  failures: string | null;
  mixed_replies: number;
  complete_replies: number;
  seconds_to_copy: number;
  seconds_to_new_answer: number | null;
  command: string | null;
  restarts: number;
  version_checked: boolean;
  runs: number;
  [key: string]: string | number | boolean | null;
}

/** The step a peer's deploy needs at its defaults, and the setting that checks its files where it starts with one. */
export const deployStep = (peer: string) => ({ way: DEPLOY_WAYS[peer], setting: onByDefault(LIVE_EDIT_SETTING[peer]) });

/**
 * A deploy with no failed request and no reply that mixes versions passes when it needed no command,
 * naming the `setting` that checks the files where the peer has one, and is `one-setting` naming the
 * command or restart it needed. A failed request, a mixed reply or a new version that never showed is `no`.
 */
function deployVerdict(evidence: DeployEvidence, way: DeployWay | undefined, setting: Setting | undefined): RowResult {
  if (evidence.failed_requests > 0 || evidence.mixed_replies > 0 || evidence.seconds_to_new_answer === null) return { result: "no", evidence };
  return way === undefined ? passed(setting, evidence) : { result: "one-setting", setting: way.name, evidence };
}

/**
 * One deploy run's result, by `deployVerdict`. A `compiled` peer's reply cannot show the copy, so its
 * new version is its first reply to a request sent after the restart.
 */
export function deployResult(load: DeployLoad, copySeconds: number, way: DeployWay | undefined, compiled: boolean, setting?: Setting): RowResult {
  const evidence: DeployEvidence = {
    requests: load.requests,
    failed_requests: load.failed,
    failures: failureText(new Map(Object.entries(load.failures))),
    mixed_replies: load.mixed,
    complete_replies: load.complete,
    seconds_to_copy: copySeconds,
    seconds_to_new_answer: load.seconds,
    command: way?.name ?? null,
    restarts: way?.run === "restart" ? 1 : 0,
    version_checked: !compiled,
    runs: 1,
  };
  return deployVerdict(evidence, way, setting);
}

/**
 * Several deploy runs of one peer as one: every count summed, the seconds the slowest run's (`null` when
 * any run never showed the new version), and the verdict `deployVerdict` gives the sums.
 */
export function mergeDeploys(peer: string, results: RowResult[]): RowResult {
  const e = results.map((r) => r.evidence as DeployEvidence);
  const sum = (key: keyof DeployEvidence) => e.reduce((n, x) => n + Number(x[key] ?? 0), 0);
  const slowest = (key: keyof DeployEvidence) => (e.some((x) => x[key] === null) ? null : Math.max(...e.map((x) => Number(x[key]))));
  const { way, setting } = deployStep(peer);
  const evidence: DeployEvidence = {
    requests: sum("requests"),
    failed_requests: sum("failed_requests"),
    failures: sumFailures(e.map((x) => x.failures)),
    mixed_replies: sum("mixed_replies"),
    complete_replies: sum("complete_replies"),
    seconds_to_copy: slowest("seconds_to_copy") ?? 0,
    seconds_to_new_answer: slowest("seconds_to_new_answer"),
    command: e[0]?.command ?? null,
    restarts: sum("restarts"),
    version_checked: e[0]?.version_checked ?? true,
    runs: sum("runs"),
  };
  return deployVerdict(evidence, way, setting);
}

/** What a request that threw failed with: its error code where it has one, else its name. */
function failureKind(e: unknown): string {
  if (!(e instanceof Error)) return String(e);
  const code = (e as { code?: unknown }).code;
  return typeof code === "string" && code !== "" ? code : e.name;
}

/**
 * The deploy rows' load: `DEPLOY_CLIENTS` loops of `GET /` from this process, each reply checked. It
 * stops `DEPLOY_AFTER_MS` after the first reply of the new version to a request sent after the deploy
 * was applied, or `CHANGE_WAIT_MS` after it was applied with none, or when the row gives up.
 */
async function checkedLoad(p: Probe, deploying: Deploying, isNew: (body: string) => boolean): Promise<DeployLoad> {
  const load: DeployLoad = { requests: 0, failed: 0, failures: {}, mixed: 0, complete: 0, seconds: null };
  let newAt: number | null = null;
  const fail = (what: string) => {
    load.failed++;
    load.failures[what] = (load.failures[what] ?? 0) + 1;
  };
  const done = () => {
    if (deploying.stopped === true) return true;
    const applied = deploying.appliedAt;
    if (applied === null) return false;
    return newAt !== null ? performance.now() - newAt >= DEPLOY_AFTER_MS : performance.now() - applied >= CHANGE_WAIT_MS;
  };
  const client = async () => {
    while (!done()) {
      const sent = performance.now();
      load.requests++;
      try {
        const r = await p.get("/", DEPLOY_REQUEST_MS);
        if (r.status !== 200) fail(String(r.status));
        else if (mixedVersions(r.body)) load.mixed++;
        else {
          load.complete++;
          const applied = deploying.appliedAt;
          if (newAt === null && applied !== null && sent >= applied && isNew(r.body)) {
            newAt = performance.now();
            load.seconds = Number(((newAt - applied) / 1000).toFixed(1));
          }
        }
      } catch (e) {
        fail(failureKind(e));
      }
    }
  };
  await Promise.all(Array.from({ length: DEPLOY_CLIENTS }, client));
  return load;
}

/**
 * One deploy run. The checking load starts, and `DEPLOY_START_MS` later `deploy.sh` writes version 2 in
 * the backend container: the parts over the served ones, or in the `release` layout the whole of
 * `releases/2/` and then the link switch. Then the peer's own step runs. Go compiles its parts into the
 * image, so it writes nothing and only restarts.
 */
async function deployProbe(p: Probe, parts: number, layout: DeployLayout): Promise<RowResult> {
  const { way, setting } = deployStep(p.peer);
  const first = ok(await p.get("/"), "the first version").body.trim();
  if (first !== "1".repeat(parts)) throw new Fail(`the first version: body ${pyRepr(first.slice(0, 200))}`);
  const fresh = "2".repeat(parts);
  const deploying: Deploying = { appliedAt: null };
  const load = checkedLoad(p, deploying, (body) => p.compiled || body.trim() === fresh);
  let copySeconds = 0;
  try {
    await Bun.sleep(DEPLOY_START_MS);
    const copyStart = performance.now();
    const ext = DEPLOY_EXT[p.peer]!;
    if (!p.compiled) {
      const copied = await p.step(deployCopyStep(ext, parts, layout));
      if (copied.code !== 0) throw new Fail(`the copy: ${copied.output}`);
    }
    copySeconds = Number(((performance.now() - copyStart) / 1000).toFixed(1));
    if (layout === "release" && !p.compiled) {
      // A deploy tool switches the link once the upload is complete.
      await Bun.sleep(DEPLOY_SETTLE_MS);
      const switched = await p.step(deploySwitchStep(ext, parts));
      if (switched.code !== 0) throw new Fail(`the link switch: ${switched.output}`);
    }
    if (way !== undefined) {
      const done = await p.step(way.run);
      if (done.code !== 0) throw new Fail(`${way.name}: ${done.output}`);
    }
    deploying.appliedAt = performance.now();
  } catch (e) {
    deploying.stopped = true;
    await load;
    throw e;
  }
  return deployResult(await load, copySeconds, way, p.compiled, setting);
}

/** One of the three deploy rows: `parts` part files, served from a `current` link to a release in the `release` layout. */
function deployRow(id: string, parts: number, layout: DeployLayout): Scenario {
  return {
    id,
    tree: "scenarios",
    case: "deploy",
    site: layout,
    parts,
    runs: DEPLOY_RUNS,
    merge: mergeDeploys,
    ...(layout === "release" ? { appDir: "/app/current", nginx: { php: "fastcgi-switch.conf" } } : {}),
    probe: (p) => deployProbe(p, parts, layout),
  };
}

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

/** The parts one request of the parallel row splits its work into, and the backend cpus it has for them. */
export const PARALLEL_PARTS = 4;

/** The loop turns in one part: enough that a part takes far longer than starting a thread or a child. */
export const PARALLEL_TURNS = 200_000_000;

/** Requests the parallel row sends at each part count. The fastest one counts. */
export const PARALLEL_TRIES = 3;

/** The speedup over doing the parts one after another below which a peer is not using the cores. */
export const PARALLEL_SPEEDUP = 2;

/** How each peer's program runs its parts at once, where it can. */
export const PARALLEL_WAY: Record<string, string> = { nvs: 'spawn script … on: "worker"', bun: "Worker", go: "goroutines" };

/** The setting a peer needs before its program may run a part elsewhere. Novis denies `spawn script` by default. */
export const PARALLEL_GRANT: Record<string, TableEdit> = {
  nvs: { table: "capabilities.script", set: { spawn: "true" }, name: "[capabilities.script] spawn" },
};

/**
 * `four` is the seconds for `PARALLEL_PARTS` parts, `one` for one part. A speedup of `PARALLEL_SPEEDUP` or
 * more over running them one after another passes, naming the setting where the peer needs one. Less than
 * that is `no`.
 */
export function parallelResult(one: number, four: number, way: string | undefined, setting: Setting | undefined): RowResult {
  const speedup = Number(((PARALLEL_PARTS * one) / four).toFixed(2));
  const evidence = { seconds_1_part: one, seconds_4_parts: four, speedup, way: way ?? null };
  if (!(speedup >= PARALLEL_SPEEDUP)) return { result: "no", evidence };
  return passed(setting, evidence);
}

/** The fewest seconds `PARALLEL_TRIES` requests for `parts` parts took, each checked to answer `200`. */
async function fastest(p: Probe, parts: number): Promise<number> {
  let best = Number.POSITIVE_INFINITY;
  for (let i = 0; i < PARALLEL_TRIES; i++) {
    const start = performance.now();
    ok(await p.get(`/?parts=${parts}&turns=${PARALLEL_TURNS}`), `${parts} parts`);
    best = Math.min(best, (performance.now() - start) / 1000);
  }
  return Number(best.toFixed(3));
}

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
    id: "escape",
    tree: "scenarios",
    case: "escape",
    probe: async (p) => {
      const q = `?q=${encodeURIComponent(ESCAPE_TERM)}`;
      return escapeResult(ok(await p.get(`/${q}`), "the template").body, ok(await p.get(`/direct${q}`), "the direct write").body);
    },
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
  deployRow("deploy", DEPLOY_PARTS, "flat"),
  deployRow("deploy-big", DEPLOY_BIG_PARTS, "flat"),
  deployRow("deploy-switch", DEPLOY_PARTS, "release"),
  {
    id: "cores",
    tree: "scenarios",
    case: "parallel",
    timedCase: CORES_CASE,
    timed: async (t) => coresResult(await t.pass(1), await t.pass(4), CORES_IN_CODE[t.peer]),
  },
  {
    id: "crash",
    tree: "scenarios",
    case: "crash",
    probe: async (p) => {
      const crash = await p.get("/crash");
      const next = await p.get("/");
      return crashResult(crash.status, next.status, p.backend());
    },
  },
  {
    id: "loop",
    tree: "scenarios",
    case: "runaway",
    memory: RUNAWAY_MEMORY,
    runs: RUNAWAY_RUNS,
    merge: mergeRunaways,
    probe: (p) => runawayProbe(p, "/loop", LOOP_DEFAULT, LOOP_IN_CODE),
  },
  {
    id: "memory",
    tree: "scenarios",
    case: "runaway",
    memory: RUNAWAY_MEMORY,
    runs: RUNAWAY_RUNS,
    merge: mergeRunaways,
    probe: (p) => runawayProbe(p, "/alloc", MEMORY_DEFAULT),
  },
  {
    id: "flood",
    tree: "scenarios",
    case: "flood",
    memory: RUNAWAY_MEMORY,
    probe: floodProbe,
  },
  {
    id: "csrf",
    tree: "scenarios",
    case: "csrf",
    probe: async (p) => csrfResult(await p.send("POST", "/change", {}), await p.send("POST", "/change", { Origin: OTHER_ORIGIN })),
  },
  {
    id: "session",
    tree: "scenarios",
    case: "session",
    fixed: {
      bun: noSession("https://bun.sh/docs/api/cookie"),
      go: noSession("https://pkg.go.dev/net/http#Cookie"),
    },
    probe: sessionProbe,
  },
  {
    id: "headers",
    tree: "cases",
    case: "hello",
    probe: async (p) => headersResult(ok(await p.get("/"), "hello").headers, HEADERS_SETTING[p.peer]),
  },
  {
    id: "waiting",
    tree: "cases",
    case: "wait",
    timed: async (t) => waitingResult(await t.pass(1), Math.min(t.requests, t.concurrency), WAITING_SETTING[t.peer]),
  },
  {
    id: "parallel",
    tree: "scenarios",
    case: "parallel",
    cpus: PARALLEL_PARTS,
    probe: async (p) => {
      const grant = PARALLEL_GRANT[p.peer];
      if (grant !== undefined) await applyTableEdit(p, grant);
      const one = await fastest(p, 1);
      return parallelResult(one, await fastest(p, PARALLEL_PARTS), PARALLEL_WAY[p.peer], written(grant));
    },
  },
];

/** `built-in`, or `one-setting (limits.memory)` and `on-by-default ([http.headers])` for a result that names its setting. */
export const verdictText = (r: RowResult) => (r.setting === undefined ? r.result : `${r.result} (${r.setting})`);

/** The line `--scenarios` prints for a row: `<id>: <peer> <result>, ...` in `PEERS` order. */
export function rowLine(id: string, byPeer: Map<string, RowResult>): string {
  const parts = PEERS.filter((p) => byPeer.has(p.id)).map((p) => `${p.label} ${verdictText(byPeer.get(p.id)!)}`);
  return `${id}: ${parts.join(", ")}`;
}

// ---------------------------------------------------------------------------------------------------
// The record.
// ---------------------------------------------------------------------------------------------------

/**
 * One peer's figures in one case. A recorded one is `medianOf` the reps: throughput and latency from the
 * median rep, memory the median of the reps', errors and statuses summed. Memory is the backend
 * container's cgroup, in bytes.
 */
export interface PeerResult {
  requests_per_sec: number;
  /** The same figure as `requests_per_sec`, kept for the readers of older records. */
  median_requests_per_sec: number;
  p50_ms: number | null;
  p99_ms: number | null;
  errors: number;
  /** Each connection error `oha` saw, by its message. */
  error_kinds: Record<string, number>;
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
  host: Record<string, unknown>;
  digests: Record<string, string>;
}

/** The figures `oha` gave for one arm, in the record's units. */
export function resultOf(arm: Arm, memory: { idle: number | null; peak: number | null }, settings: PeerResult["settings"]): PeerResult {
  const kinds = (arm.errors ?? {}) as Record<string, number>;
  const errors = Object.values(kinds).reduce((a, b) => a + b, 0);
  return {
    requests_per_sec: arm.requests_per_sec,
    median_requests_per_sec: arm.median_requests_per_sec,
    p50_ms: arm.latency_ms?.p50 ?? null,
    p99_ms: arm.latency_ms?.p99 ?? null,
    errors,
    error_kinds: kinds,
    status_codes: arm.status_codes ?? {},
    memory_idle_bytes: memory.idle,
    memory_peak_bytes: memory.peak,
    settings,
  };
}

/** The median of the values that are not `null`, by `medianIndex`, or `null` when there are none. */
function medianOrNull(values: (number | null)[]): number | null {
  const known = values.filter((v): v is number => v !== null);
  return known.length === 0 ? null : known[medianIndex(known)]!;
}

/**
 * One peer's passes of one case as one result. Throughput and latency are the median pass's, by
 * requests per second, and each memory figure is the median of the passes'. Errors and statuses are
 * summed over every pass, so a failure in any pass shows.
 */
export function medianOf(passes: PeerResult[]): PeerResult {
  const mid = passes[medianIndex(passes.map((p) => p.requests_per_sec))]!;
  const errorKinds: Record<string, number> = {};
  const statusCodes: Record<string, number> = {};
  for (const p of passes) {
    for (const [what, n] of Object.entries(p.error_kinds)) errorKinds[what] = (errorKinds[what] ?? 0) + n;
    for (const [code, n] of Object.entries(p.status_codes)) statusCodes[code] = (statusCodes[code] ?? 0) + n;
  }
  return {
    ...mid,
    median_requests_per_sec: mid.requests_per_sec,
    errors: passes.reduce((n, p) => n + p.errors, 0),
    error_kinds: errorKinds,
    status_codes: statusCodes,
    memory_idle_bytes: medianOrNull(passes.map((p) => p.memory_idle_bytes)),
    memory_peak_bytes: medianOrNull(passes.map((p) => p.memory_peak_bytes)),
  };
}

/**
 * The whole record `--record` writes: the machine under `host`, as `bench-proxied`'s record names it,
 * every peer's image, and per case every peer's figures, the median of `reps` interleaved passes,
 * in `PEERS` order. The scenario rows sit under `scenarios`, from their own run; nothing joins the
 * record to `benches/serve-proxied.json`.
 */
export function buildRecord(run: RunInfo, results: Map<string, Map<string, PeerResult>>, scenarios: RecordedScenarios | null = null) {
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
    host: run.host,
    peers: Object.fromEntries(PEERS.map((p) => [p.id, { label: p.label, image: p.image }])),
    images: { nginx: NGINX_IMAGE, oha: OHA_IMAGE },
    cases,
    scenarios,
    config_digests: run.digests,
  };
}

/** One row as the record keeps it: the repo-relative folder its four programs are in, and each peer's result in `PEERS` order. */
export interface RecordedRow {
  folder: string;
  peers: Record<string, RowResult>;
}

/** One `--scenarios` run: when, at which commit, and every row in `SCENARIOS` order. */
export interface RecordedScenarios {
  date: string;
  commit: string;
  rows: Record<string, RecordedRow>;
}

/** The folder a row's programs are in, which the page links each cell to. */
export const scenarioFolder = (s: Scenario) => `benches/compare/${s.tree}/${s.case}`;

export function buildScenarios(date: string, commit: string, results: Map<string, Map<string, RowResult>>): RecordedScenarios {
  const rows: Record<string, RecordedRow> = {};
  for (const s of SCENARIOS) {
    const byPeer = results.get(s.id);
    if (byPeer === undefined) continue;
    const peers: Record<string, RowResult> = {};
    for (const p of PEERS) {
      const r = byPeer.get(p.id);
      if (r !== undefined) peers[p.id] = r;
    }
    rows[s.id] = { folder: scenarioFolder(s), peers };
  }
  return { date, commit, rows };
}

/** `prior`, a record the cases wrote, with its `scenarios` replaced and its keys in `buildRecord`'s order. */
export function withScenarios(prior: Record<string, unknown>, scenarios: RecordedScenarios): Record<string, unknown> {
  if (typeof prior.cases !== "object" || prior.cases === null) throw new Fail("the record has no cases; record them first with bun nv bench-compare --record PATH");
  const { scenarios: _, config_digests, ...rest } = prior;
  return { ...rest, scenarios, config_digests };
}

/** The record at `path`, or `null` when there is none yet. */
function readRecord(path: string): Record<string, unknown> | null {
  return existsSync(path) ? (JSON.parse(readFileSync(path, "utf8")) as Record<string, unknown>) : null;
}

function writeRecord(path: string, record: unknown) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(record, null, 2)}\n`, "utf8");
}

/** Every file whose contents could move a number, digested into the record. */
const TUNABLES = [
  ...PEERS.map((p) => join(COMPARE_DIR, p.stack)),
  join(COMPARE_DIR, "compose.common.yaml"),
  join(COMPARE_DIR, "nginx", "http.conf"),
  join(COMPARE_DIR, "nginx", "fastcgi.conf"),
  join(COMPARE_DIR, "nginx", "fastcgi-switch.conf"),
  join(COMPARE_DIR, "bun", "serve.ts"),
  join(COMPARE_DIR, "deploy", "deploy.sh"),
  join(COMPARE_DIR, "go", "Dockerfile"),
  PHP_INI,
  FILE_CHECK_INI,
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

/** The backend container's state from `docker inspect`. A container that is gone does not run. */
function backendOf(stack: string, env: Record<string, string>): Backend {
  const id = compose(stack, ["ps", "-a", "-q", "app"], env, true).stdout.trim();
  if (id === "") return { running: false, restarts: 0 };
  const out = Bun.spawnSync(["docker", "inspect", "-f", "{{.State.Running}} {{.RestartCount}}", id], { stdout: "pipe", stderr: "pipe" });
  const [running, restarts] = out.stdout.toString().trim().split(" ");
  return { running: running === "true", restarts: pyInt(restarts ?? "") ?? 0 };
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
async function steadyOf(stack: string, env: Record<string, string>, path: string, seconds: number, connections = SCENARIO_CONCURRENCY): Promise<Steady> {
  const args = ["run", "--rm", "--no-deps", "oha", "-z", `${seconds}s`, "-c", String(connections), "-t", "5s"];
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

/**
 * The figures `oha`'s JSON gives for a steady run. Every status but `200` and every error but `DEADLINE`
 * failed. A latency `oha` did not measure is `null` or absent, and both give a `null` p99.
 */
export function steadyResult(json: { statusCodeDistribution?: Record<string, number>; errorDistribution?: Record<string, number>; metrics?: { latency_ms?: { p99?: number | null } } }): Steady {
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
    p99_ms: p99 == null ? null : Number(p99.toFixed(1)),
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
    const c = s.timedCase ?? CASES.find((k) => k.id === s.case)!;
    const loadOf = { ...load, concurrency: c.concurrency, reps: 1 };
    // A timed row asks for its passes one budget at a time, inside one peer's turn, so its reps run one
    // peer after another and do not interleave as the cases' do. Each is still a fresh stack, and the
    // median of them is what the row judges.
    return await s.timed({
      peer: peer.id,
      requests: load.requests,
      concurrency: c.concurrency,
      pass: async (cpus) => {
        const passes: PeerResult[] = [];
        for (let i = 0; i < load.reps; i++) passes.push((await onePass(peer, c, { ...ctx, cpus, concurrency: c.concurrency }, loadOf)).result!);
        return medianOf(passes);
      },
    });
  }
  if (s.copy === true && peer.compiled !== true) {
    rmSync(join(GENERATED, s.case), { recursive: true, force: true });
    cpSync(join(COMPARE_DIR, s.tree, s.case), join(GENERATED, s.case), { recursive: true });
    ctx = { ...ctx, tree: "generated" };
  }
  const env = stackEnv(peer, ctx);
  const stack = stackPath(peer);
  peer.prepare?.(ctx);
  try {
    if (s.site !== undefined && peer.compiled !== true) {
      const init = deployInitRun(join(COMPARE_DIR, s.tree, s.case), DEPLOY_EXT[peer.id]!, s.parts ?? DEPLOY_PARTS, s.site);
      const done = compose(stack, init, env, true);
      if (done.code !== 0) throw new Error(`deploy.sh init: ${done.stdout}${done.stderr}`);
    }
    if (s.serves !== false) up(stack, env, "app", "nginx");
    return await s.probe!({
      peer: peer.id,
      compiled: peer.compiled === true,
      dir: join(COMPARE_DIR, ctx.tree, ctx.case),
      exec: (command) => {
        const done = compose(stack, ["exec", "-T", "app", ...command], env, true);
        return { code: done.code, output: done.stdout + done.stderr };
      },
      restart: () => {
        const done = compose(stack, ["restart", "app"], env, true);
        return { code: done.code, output: done.stdout + done.stderr };
      },
      step: (run) => composeAsync(stack, run === "restart" ? ["restart", "app"] : ["exec", "-T", "app", ...run], env),
      get: (path, timeoutMs) => replyOf(peer, path, "GET", {}, timeoutMs),
      logs: () => logsOf(stack, "app", env),
      steady: (path, seconds, connections) => steadyOf(stack, env, path, seconds, connections),
      send: (method, path, headers) => replyOf(peer, path, method, headers),
      backend: () => backendOf(stack, env),
      check: (file) => {
        const done = compose(stack, ["run", "--rm", "--no-deps", "--build", "app", "check", `/app/${file}`], env, true);
        return { code: done.code, output: done.stdout + done.stderr };
      },
    });
  } catch (e) {
    throw new Fail(`${peer.label}, row ${s.id}: ${messageOf(e)}\n${peer.id} said:\n${logsOf(stack, "app", env)}`);
  } finally {
    // `down` removes the `site` volume a deploy row wrote, too.
    down(stack, env);
  }
}

/** `docker compose` without blocking this process, for a step taken while the row's own requests run. */
async function composeAsync(stack: string, args: string[], env: Record<string, string>): Promise<{ code: number; output: string }> {
  const proc = Bun.spawn(["docker", "compose", "-f", stack, ...args], { env: { ...process.env, ...env }, stdout: "pipe", stderr: "pipe" });
  const [out, err, code] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
  return { code, output: out + err };
}

// ---------------------------------------------------------------------------------------------------
// The command.
// ---------------------------------------------------------------------------------------------------

const USAGE =
  "usage: nv bench-compare [-h] [--smoke] [--scenarios] [--arm {fair,deployed}] [--backend-cpus N]\n" +
  "                        [--row ID] [--expect PEER=RESULT,...] [--requests REQUESTS] [--reps REPS]\n" +
  "                        [--record PATH] [--down] [--profile CASE]";

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
  expect: Map<string, Verdict> | null;
  profile: Case | null;
}

/** `--expect nvs=on-by-default,php=no`: the result each named peer must get in every row run. */
export function parseExpect(word: string): Map<string, Verdict> {
  const expect = new Map<string, Verdict>();
  for (const pair of word.split(",")) {
    const [peer, result, ...rest] = pair.split("=");
    if (peer === undefined || result === undefined || rest.length > 0 || peer === "") throw new ArgError(`argument --expect: ${pyRepr(pair)} is not PEER=RESULT`);
    if (!PEERS.some((p) => p.id === peer)) throw new ArgError(`argument --expect: no peer ${pyRepr(peer)} (choose from ${PEERS.map((p) => `'${p.id}'`).join(", ")})`);
    if (!(VERDICTS as readonly string[]).includes(result)) throw new ArgError(`argument --expect: invalid result: ${pyRepr(result)} (choose from ${VERDICTS.map((v) => `'${v}'`).join(", ")})`);
    if (expect.has(peer)) throw new ArgError(`argument --expect: peer ${pyRepr(peer)} is named twice`);
    expect.set(peer, result as Verdict);
  }
  return expect;
}

/** One line for each row whose result for an expected peer is not the expected one, in row order. */
export function unmetExpectations(expect: Map<string, Verdict>, results: Map<string, Map<string, RowResult>>): string[] {
  const lines: string[] = [];
  for (const [row, byPeer] of results) {
    for (const [peer, want] of expect) {
      const got = byPeer.get(peer)?.result;
      if (got !== want) lines.push(`${row}: ${peer} is ${got ?? "missing"}, want ${want}`);
    }
  }
  return lines;
}

function options(args: string[]): Options | null {
  const p = parseArgs(args, { flags: ["--smoke", "--scenarios", "--down"], valued: ["--arm", "--backend-cpus", "--requests", "--reps", "--record", "--row", "--expect", "--profile"] });
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
  const expect = p.values.get("--expect");
  if (expect !== undefined && !p.flags.has("--scenarios")) throw new ArgError("argument --expect: only with --scenarios");
  const profile = p.values.get("--profile");
  const profiled = profile === undefined ? null : CASES.find((c) => c.id === profile);
  if (profiled === undefined) throw new ArgError(`argument --profile: invalid choice: ${pyRepr(profile!)} (choose from ${CASES.map((c) => `'${c.id}'`).join(", ")})`);
  if (profiled !== null && (p.flags.has("--smoke") || p.flags.has("--scenarios") || p.flags.has("--down") || p.values.has("--record"))) {
    throw new ArgError("argument --profile: not with --smoke, --scenarios, --down or --record");
  }
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
    expect: expect === undefined ? null : parseExpect(expect),
    profile: profiled,
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
    "                        one line per row with each peer's result. With --record, the rows replace",
    "                        the scenarios of the record the cases already wrote to PATH",
    "  --arm {fair,deployed}",
    "                        fair: one cpu to each backend. deployed: --backend-cpus to each backend",
    "  --backend-cpus N      the CPU budget of EACH backend container (default 1 for fair, 4 for deployed)",
    "  --requests REQUESTS   requests per timed pass (default 20000, 300 with --smoke)",
    "  --reps REPS           timed passes per arm, each on a fresh stack, the peers taking turns; the",
    "                        median pass is recorded (default 5, 1 with --smoke)",
    "  --record PATH         write this run to PATH as one JSON object. A case run keeps the scenarios",
    "                        already recorded there",
    "  --row ID              with --scenarios, run only the row ID",
    "  --expect PEER=RESULT,...",
    "                        with --scenarios, exit 1 when a row's result for PEER is not RESULT",
    "                        (built-in, on-by-default, one-setting, your-code or no)",
    "  --down                tear every peer's stack down and stop",
    "  --profile CASE        profile Novis alone serving CASE: the top symbols under perf, the system",
    "                        calls per request under strace, and for wait the memory per request in",
    "                        flight. Needs a privileged container",
    "",
    "benches/compare/README.md is the design; this tool is only how it is run. It needs bun and Docker.",
  ].join("\n");
}

/** Connections a probed row's stack is sized for: a probe puts no load on the backend. A timed row uses its case's. */
const SCENARIO_CONCURRENCY = 8;

/** What one peer's stack is started with for row `s`, at a backend budget of `cpus` unless the row names its own. */
export function scenarioCtx(s: Scenario, peer: Peer, cpus: number, commit: string): PeerCtx {
  return {
    cpus: s.cpus ?? cpus,
    concurrency: SCENARIO_CONCURRENCY,
    tree: s.tree,
    case: s.case,
    commit,
    memory: s.memory,
    appDir: s.appDir,
    site: s.site !== undefined,
    parts: s.parts,
    nginx: s.nginx?.[peer.id],
  };
}

/** `--scenarios`: every row against every peer, one fresh stack per pair, one line per row, then the record. */
async function scenarios(o: Options, cpus: number, commit: string, load: Load): Promise<number> {
  if (o.record !== null && o.row !== null) throw new Fail("--record keeps every row of one run; drop --row or --record");
  const prior = o.record === null ? null : readRecord(resolve(o.record));
  if (o.record !== null && prior === null) throw new Fail(`${o.record}: no record yet; record the cases first with bun nv bench-compare --record ${o.record}`);
  const rows = SCENARIOS.filter((s) => o.row === null || s.id === o.row);
  if (rows.length === 0) throw new Fail(`--row ${o.row}: no such row; the rows are ${SCENARIOS.map((s) => s.id).join(", ")}`);
  const results = new Map<string, Map<string, RowResult>>();
  for (const s of rows) {
    const runs = o.smoke ? 1 : (s.runs ?? 1);
    if (runs > 1 && s.merge === undefined) throw new Fail(`row ${s.id} runs ${runs} times and has no merge`);
    const passes = new Map<string, RowResult[]>(PEERS.map((p) => [p.id, []]));
    // Run 1 of every peer, then run 2, so a slow stretch of the shared host lands on every peer.
    for (let run = 1; run <= runs; run++) {
      for (const peer of PEERS) {
        if (run > 1 && s.fixed?.[peer.id] !== undefined) continue;
        console.log(`  ${s.id}: ${peer.label}${runs > 1 ? `, run ${run} of ${runs}` : ""} ...`);
        passes.get(peer.id)!.push(await scenarioPass(peer, s, scenarioCtx(s, peer, cpus, commit), load));
      }
    }
    const byPeer = new Map<string, RowResult>();
    for (const [peer, rs] of passes) byPeer.set(peer, rs.length === 1 ? rs[0]! : s.merge!(peer, rs));
    console.log(rowLine(s.id, byPeer));
    results.set(s.id, byPeer);
    for (const peer of PEERS) {
      const evidence = Object.entries(byPeer.get(peer.id)!.evidence).map(([k, v]) => `${k} ${v}`);
      console.log(`    ${peer.label.padEnd(6)} ${evidence.join(", ")}`);
    }
  }
  if (o.record !== null && prior !== null) {
    writeRecord(resolve(o.record), withScenarios(prior, buildScenarios(stamp(), commit, results)));
    console.log(`  recorded the rows to ${o.record}`);
  }
  if (o.expect === null) return 0;
  const unmet = unmetExpectations(o.expect, results);
  for (const line of unmet) console.error(`nv bench-compare: unexpected result: ${line}`);
  return unmet.length === 0 ? 0 : 1;
}

const mib = (bytes: number | null) => (bytes === null ? "    -" : fixed(bytes / 1048576, 1).padStart(6));

/** The body file `--profile` writes for a case whose timed request has one, under `PROFILE_DIR`. */
const PROFILE_BODY = "body";

/** `docker run`'s arguments for `--profile`: the bench image, privileged, with `profile.sh` as its program. */
export function profileArgs(c: Case, dir: string): string[] {
  const mount = (src: string, dst: string, readonly: boolean) => ["--mount", `type=bind,src=${src},dst=${dst}${readonly ? ",readonly" : ""}`];
  return [
    "run", "--rm", "--privileged", "--entrypoint", "bash",
    ...mount(join(COMPARE_DIR, "cases"), "/cases", true),
    ...mount(dir, "/profile", true),
    ...mount(PROFILE_SCRIPT, "/profile.sh", true),
    NVS_IMAGE,
    "/profile.sh", c.id, c.timed.method, c.timed.path, ...(c.timed.body === undefined ? [] : [PROFILE_BODY]),
  ];
}

/** `--profile`: builds the image, stages `oha` and the body, and runs `profile.sh` with its output shown. */
function profile(c: Case, commit: string): number {
  const nvs = PEERS.find((p) => p.id === "nvs")!;
  const env = stackEnv(nvs, { cpus: 1, concurrency: c.concurrency, tree: "cases", case: c.id, commit });
  if (compose(stackPath(nvs), ["build", "app"], env, false).code !== 0) throw new Fail(`building ${NVS_IMAGE} failed`);
  mkdirSync(PROFILE_DIR, { recursive: true });
  const docker = (args: string[]) => {
    const out = Bun.spawnSync(["docker", ...args], { stdout: "pipe", stderr: "pipe" });
    if (out.exitCode !== 0) throw new Fail(`docker ${args[0]} failed: ${out.stderr.toString().trim()}`);
    return out.stdout.toString().trim();
  };
  const made = docker(["create", OHA_IMAGE]);
  try {
    docker(["cp", `${made}:/bin/oha`, join(PROFILE_DIR, "oha")]);
  } finally {
    docker(["rm", made]);
  }
  if (c.timed.body !== undefined) writeFileSync(join(PROFILE_DIR, PROFILE_BODY), c.timed.body);
  const done = Bun.spawnSync(["docker", ...profileArgs(c, PROFILE_DIR)], { stdout: "inherit", stderr: "inherit" });
  if (done.exitCode === 3) {
    console.error(`perf cannot record in a container on this machine. ${rel(PROFILE_SCRIPT)} is the profile, step by step: run its steps by hand on a Linux host.`);
    return 1;
  }
  return done.exitCode === 0 ? 0 : 1;
}

async function main(o: Options): Promise<number> {
  requireDocker();
  const commit = version("git", ["rev-parse", "--short", "HEAD"]);
  if (o.profile !== null) return profile(o.profile, commit);

  if (o.down) {
    for (const peer of PEERS) down(stackPath(peer), stackEnv(peer, { cpus: 1, concurrency: 1, tree: "cases", case: CASES[0]!.id, commit }));
    console.log("every peer's stack is down");
    return 0;
  }

  const cpus = o.backendCpus ?? (o.arm === "fair" ? 1 : 4);
  if (o.arm === "fair" && cpus !== 1) throw new Fail(`--arm fair is one cpu to each backend; --backend-cpus ${cpus} is the deployed arm and must say so`);
  const requests = o.requests ?? (o.smoke ? 300 : 20_000);
  const reps = o.reps ?? (o.smoke ? 1 : 5);
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
    const passes = new Map<string, PeerResult[]>(PEERS.map((p) => [p.id, []]));
    // Rep 1 of every peer, then rep 2, each on a fresh stack, so a slow stretch of the shared host lands
    // on every peer and not on one peer's block of reps.
    for (let rep = 1; rep <= reps; rep++) {
      const answers = new Map<string, Answer[]>();
      for (const peer of PEERS) {
        console.log(`  ${c.id}: ${peer.label}, rep ${rep} of ${reps}, ${requests} requests over ${c.concurrency} connection(s) ...`);
        const pass = await onePass(peer, c, ctxOf(c), { requests, concurrency: c.concurrency, reps: 1 });
        answers.set(peer.id, pass.answers);
        passes.get(peer.id)!.push(pass.result!);
      }
      const bad = agreement(c, answers);
      if (bad !== null) throw new Fail(`DIFF in ${c.id}: ${bad.peer} disagrees on input ${bad.input + 1}: ${bad.why}. No number stands.`);
    }
    const byPeer = new Map<string, PeerResult>();
    for (const [peer, rs] of passes) byPeer.set(peer, medianOf(rs));
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
  if (o.record === null) {
    console.log("  not recorded: pass --record PATH to write this run");
    return 0;
  }
  const full = resolve(o.record);
  const kept = (readRecord(full)?.scenarios ?? null) as RecordedScenarios | null;
  writeRecord(full, buildRecord({ date: stamp(), commit, arm: o.arm, backendCpus: cpus, requests, reps, host: machine, digests: digests() }, results, kept));
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
