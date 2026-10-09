// `bun nv bench-compare`: Novis, PHP-FPM, Bun and Go, each behind the same nginx terminating TLS 1.3.
//
//     bun nv bench-compare --smoke                               # every peer and case, a few hundred requests
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

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { rel, ROOT } from "../lib/paths.ts";
import { ArgError, fixed, parseArgs, pyInt, pyRepr } from "../lib/py.ts";
import { Fail, grouped, host, stamp, version } from "./bench.ts";
import { type Arm, compose, down, type Load, logsOf, messageOf, ohaArm, requireDocker, up } from "./bench-docker.ts";
import { effectivePhp } from "./bench-proxied.ts";

export const summary = "Novis, PHP-FPM, Bun and Go behind one TLS nginx, in containers: nv bench-compare [--smoke] [--arm fair|deployed] [--record PATH]";

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
  case: string;
  commit: string;
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
    CASE: ctx.case,
    BACKEND_CPUS: String(ctx.cpus),
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

export const CASES: Case[] = [
  {
    id: "hello",
    concurrency: 64,
    timed: { method: "GET", path: "/" },
    inputs: [{ method: "GET", path: "/" }],
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

// ---------------------------------------------------------------------------------------------------
// The command.
// ---------------------------------------------------------------------------------------------------

const USAGE =
  "usage: nv bench-compare [-h] [--smoke] [--arm {fair,deployed}] [--backend-cpus N] [--requests REQUESTS]\n" +
  "                        [--reps REPS] [--record PATH] [--down]";

interface Options {
  smoke: boolean;
  arm: "fair" | "deployed";
  backendCpus: number | null;
  requests: number | null;
  reps: number | null;
  record: string | null;
  down: boolean;
}

function options(args: string[]): Options | null {
  const p = parseArgs(args, { flags: ["--smoke", "--down"], valued: ["--arm", "--backend-cpus", "--requests", "--reps", "--record"] });
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
    arm,
    backendCpus: int("--backend-cpus"),
    requests: int("--requests"),
    reps: int("--reps"),
    record: p.values.get("--record") ?? null,
    down: p.flags.has("--down"),
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
    "  --arm {fair,deployed}",
    "                        fair: one cpu to each backend. deployed: --backend-cpus to each backend",
    "  --backend-cpus N      the CPU budget of EACH backend container (default 1 for fair, 4 for deployed)",
    "  --requests REQUESTS   requests per timed pass (default 20000, 300 with --smoke)",
    "  --reps REPS           timed passes per arm, best wins (default 3, 1 with --smoke)",
    "  --record PATH         write this run to PATH as one JSON object",
    "  --down                tear every peer's stack down and stop",
    "",
    "benches/compare/README.md is the design; this tool is only how it is run. It needs bun and Docker.",
  ].join("\n");
}

const mib = (bytes: number | null) => (bytes === null ? "    -" : fixed(bytes / 1048576, 1).padStart(6));

async function main(o: Options): Promise<number> {
  requireDocker();
  const commit = version("git", ["rev-parse", "--short", "HEAD"]);

  if (o.down) {
    for (const peer of PEERS) down(stackPath(peer), stackEnv(peer, { cpus: 1, concurrency: 1, case: CASES[0]!.id, commit }));
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
  const ctxOf = (c: Case): PeerCtx => ({ cpus, concurrency: c.concurrency, case: c.id, commit });

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
