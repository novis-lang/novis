// `bun nv bench-proxied`: nginx in front of `nvs serve` and in front of PHP-FPM, one stack at a time.
//
//     bun nv bench-proxied --record benches/serve-proxied.json     # the fair arm, recorded
//     bun nv bench-proxied --arm deployed --backend-cpus 8         # both peers on eight cpus
//     bun nv bench-proxied --nvs-bin /var/tmp/nvs-target-wsl/release/nvs   # skip the image build
//     bun nv bench-proxied --down                                  # tear both stacks down
//
// `benches/proxied/README.md` is this leg's design and the only home for **why** it is shaped this way --
// why nginx fronts both peers, why the two stacks are separate compose files, why there are two arms,
// and why its artifact is not `benches/serve.json`. Nothing here restates any of it. This file owns only
// how the run is carried out.
//
// The other leg is `bun nv bench --serve-vs-fpm`: no containers, no proxy, Windows-native, and driven by
// a generator written into that command. It stays, it is goal `server`'s acceptance check, and it is not
// comparable with this one. Where the two would otherwise hold the same code -- the FastCGI client and
// the closed-loop generator behind arm 4 -- this one imports it from `tools/nv/cmd/bench.ts`.
//
// # What a run does
//
// Bring up one stack, measure its arms, tear it down, then the other. Never both at once: they would
// compete for the same cores and neither number would be worth writing down. The teardown is in a
// `finally`, so an interrupted run does not leave a stack holding a CPU budget.
//
// A record's `host` names the machine the way `bun nv bench` does, and arm 4's `generator` is
// `nv bench`, the client that drives it.

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { rel, ROOT } from "../lib/paths.ts";
import { ArgError, fixed, parseArgs, pyInt, pyRepr, splitlines } from "../lib/py.ts";
import { bytesRepr, Fail, FcgiConn, grouped, hammer, host, median, round, stamp, version, writeServeRecord } from "./bench.ts";

export const summary = "nginx in front of nvs serve and of PHP-FPM, in containers: nv bench-proxied [--arm fair|deployed] [--record PATH] [--down]";

const BENCH_DIR = join(ROOT, "benches", "proxied");
const CASE = "hello";

// The published ports, all on the loopback and none of them standard. `tests/db/compose.yaml`'s header
// owns that rule for every compose file here.
const PHP_NGINX_PORT = 18080;
const PHP_FCGI_PORT = 19000;
const NVS_NGINX_PORT = 18081;
const NVS_DIRECT_PORT = 18082;

// The path `/app` is mounted at inside both backends, written POSIX whatever the host is: PHP-FPM given
// `\app\hello.php` answers 404 to every request in the run.
const CONTAINER_PHP = "/app/hello.php";

const PHP_STACK = join(BENCH_DIR, "compose.php.yaml");
const NVS_STACK = join(BENCH_DIR, "compose.nvs.yaml");

// Every file whose contents could move a number. Digested into each record so a figure can always be
// tied to the configuration that produced it -- and so a run taken after an untracked edit is visibly a
// different run rather than a mysterious one.
const TUNABLES = [
  join(BENCH_DIR, "nginx", "php.conf"),
  join(BENCH_DIR, "nginx", "nvs.conf"),
  join(BENCH_DIR, "php", "opcache.ini"),
  join(BENCH_DIR, "php", "pool.conf.in"),
];

const USAGE =
  "usage: nv bench-proxied [-h] [--arm {fair,deployed}] [--backend-cpus N] [--requests REQUESTS]\n" +
  "                        [--concurrency CONCURRENCY] [--reps REPS] [--record PATH] [--nvs-bin PATH]\n" +
  "                        [--allow-debug] [--keep-up] [--down]";

interface Options {
  arm: "fair" | "deployed";
  backendCpus: number | null;
  requests: number;
  concurrency: number;
  reps: number;
  record: string | null;
  nvsBin: string | null;
  allowDebug: boolean;
  keepUp: boolean;
  down: boolean;
}

/** The options argparse would have read, or null when `--help` asked for the help instead. */
function options(args: string[]): Options | null {
  const p = parseArgs(args, {
    flags: ["--allow-debug", "--keep-up", "--down"],
    valued: ["--arm", "--backend-cpus", "--requests", "--concurrency", "--reps", "--record", "--nvs-bin"],
  });
  if (p.flags.has("--help")) return null;
  const int = (o: string): number | null => {
    const word = p.values.get(o);
    if (word === undefined) return null;
    const n = pyInt(word);
    if (n === null) throw new ArgError(`argument ${o}: invalid int value: ${pyRepr(word)}`);
    return n;
  };
  const arm = p.values.get("--arm") ?? "fair";
  if (arm !== "fair" && arm !== "deployed") {
    throw new ArgError(`argument --arm: invalid choice: ${pyRepr(arm)} (choose from 'fair', 'deployed')`);
  }
  return {
    arm,
    backendCpus: int("--backend-cpus"),
    requests: int("--requests") ?? 20_000,
    concurrency: int("--concurrency") ?? 8,
    reps: int("--reps") ?? 3,
    record: p.values.get("--record") ?? null,
    nvsBin: p.values.get("--nvs-bin") ?? null,
    allowDebug: p.flags.has("--allow-debug"),
    keepUp: p.flags.has("--keep-up"),
    down: p.flags.has("--down"),
  };
}

function help(): string {
  return [
    USAGE,
    "",
    "nginx in front of both peers: nvs serve against PHP-FPM, in containers.",
    "",
    "options:",
    "  -h, --help            show this help message and exit",
    "  --arm {fair,deployed}",
    "                        fair: one cpu to each backend (the runtime-against-runtime number).",
    "                        deployed: --backend-cpus to each backend, which both peers use whole",
    "  --backend-cpus N      the CPU budget given to EACH backend container (default 1 for fair, 4 for",
    "                        deployed). The PHP worker count is sized from --concurrency instead;",
    "                        php/pool.conf.in says why",
    "  --requests REQUESTS   requests per timed pass (default 20000)",
    "  --concurrency CONCURRENCY",
    "                        concurrent connections (default 8)",
    "  --reps REPS           timed passes per arm, best wins (default 3)",
    "  --record PATH         append this run to a JSON array",
    "  --nvs-bin PATH        a prebuilt Linux release nvs to measure",
    "  --allow-debug         permit a debug --nvs-bin",
    "  --keep-up             leave the Novis stack running after the run, to poke at by hand. The PHP",
    "                        stack is always torn down: it comes first, and the Novis arms must not",
    "                        share a box with it",
    "  --down                tear both stacks down and stop",
    "",
    "benches/proxied/README.md is the design; this tool is only how it is run.",
  ].join("\n");
}

const messageOf = (e: unknown) => (e instanceof Error ? e.message : String(e));

// ---------------------------------------------------------------------------------------------------
// Rendering, staging and the two preconditions.
// ---------------------------------------------------------------------------------------------------

/**
 * Writes `generated/pool.conf` from the committed template. php-fpm interpolates no environment variable
 * in a pool file, so the one number that varies between arms is substituted here; the template's own
 * header carries why.
 */
function renderPool(children: number): void {
  const out = join(BENCH_DIR, "generated");
  mkdirSync(out, { recursive: true });
  const template = readFileSync(join(BENCH_DIR, "php", "pool.conf.in"), "utf8");
  writeFileSync(join(out, "pool.conf"), template.replaceAll("@PHP_CHILDREN@", String(children)), "utf8");
}

/**
 * Copies a prebuilt Linux `nvs` to `.bin/nvs`, refusing anything that is not one. Two checks and no more.
 * **ELF magic**, because the obvious mistake on this machine is handing it `target/release/nvs.exe` -- a
 * PE binary the image would accept, place on the PATH and fail to exec, with a message about a missing
 * file that is plainly there. And a **`debug` path component**, because a debug build measured as a
 * release one does not fail at all: it produces a number several times too low and nothing in the output
 * says why. `bun nv bench` refuses the same thing for the same reason.
 */
function stageBinary(path: string, allowDebug: boolean): void {
  if (!existsSync(path) || !statSync(path).isFile()) throw new Fail(`--nvs-bin ${path} is not a file`);
  const bytes = readFileSync(path);
  const magic = bytes.subarray(0, 4);
  if (!magic.equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]))) {
    throw new Fail(
      `--nvs-bin ${path} is not a Linux ELF binary (magic ${bytesRepr(magic)}); this image runs Linux, ` +
        "so a Windows `nvs.exe` cannot be the thing measured",
    );
  }
  if (!allowDebug && path.split(/[\\/]/).includes("debug")) {
    throw new Fail(
      `--nvs-bin ${path} looks like a debug build; a debug binary benchmarks several times ` +
        "slower and the result would be wrong rather than failed. Pass --allow-debug to insist",
    );
  }
  const staged = join(BENCH_DIR, ".bin");
  mkdirSync(staged, { recursive: true });
  writeFileSync(join(staged, "nvs"), bytes);
}

/** Fails before anything is built if there is no daemon, naming what is missing. */
function requireDocker(): void {
  let out: ReturnType<typeof Bun.spawnSync>;
  try {
    out = Bun.spawnSync(["docker", "version", "--format", "{{.Server.Version}}"], { stdout: "pipe", stderr: "pipe", timeout: 60_000 });
  } catch (e) {
    throw new Fail(`docker is not runnable (${messageOf(e)}); this leg is containers and has no fallback`);
  }
  if (out.exitCode !== 0) {
    const said = (out.stderr?.toString() ?? "").trim() || (out.stdout?.toString() ?? "").trim();
    throw new Fail(`the docker daemon is not answering; this leg is containers and has no fallback. \`docker version\` said: ${said}`);
  }
}

// ---------------------------------------------------------------------------------------------------
// Driving compose.
// ---------------------------------------------------------------------------------------------------

interface Done {
  code: number;
  stdout: string;
  stderr: string;
}

/** One `docker compose -f <stack> ...`, with the environment the compose file reads. */
function compose(stack: string, args: string[], env: Record<string, string>, capture: boolean): Done {
  const out = Bun.spawnSync(["docker", "compose", "-f", stack, ...args], {
    env: { ...process.env, ...env },
    stdout: capture ? "pipe" : "inherit",
    stderr: capture ? "pipe" : "inherit",
    timeout: 1_800_000,
  });
  return { code: out.exitCode ?? 1, stdout: out.stdout?.toString() ?? "", stderr: out.stderr?.toString() ?? "" };
}

/**
 * Brings named services up and waits for **healthy**, not merely started. `--wait` is why every service
 * in both compose files carries a healthcheck: without it the first requests of a run land on a server
 * that is still initialising, and they land only in the first rep -- which reads as a warm-up effect
 * rather than as the harness's own bug.
 *
 * **Services are named rather than defaulted, and that is load-bearing.** The origin arms are measured
 * with the proxy not yet started; § *Why the origin is measured before its proxy exists* in `main` is the
 * reason, and it is a correctness one rather than a tidiness one.
 *
 * **`--build` on every call**, because `nvs-bench-serve:local` is a tag compose reuses whenever it
 * exists: without the flag a run measures whatever binary the image was first built from and records
 * this checkout's commit beside it. Docker's layer cache makes the rebuild of an unchanged tree cheap,
 * and the services that only name an image have nothing to build.
 */
function up(stack: string, env: Record<string, string>, ...services: string[]): void {
  const done = compose(stack, ["up", "-d", "--wait", "--build", ...services], env, false);
  if (done.code !== 0) throw new Error(`\`docker compose -f ${basename(stack)} up ${services.join(" ")}\` failed with ${done.code}`);
}

/** Tears a stack down, including its network. Never throws: this runs in a `finally`. */
function down(stack: string): void {
  try {
    compose(stack, ["down", "-v", "--remove-orphans"], {}, true);
  } catch {
    // The stack may already be gone, or docker with it; neither is this run's failure.
  }
}

/** What a service printed, for the failure message. A generator only ever sees a reset socket. */
function logsOf(stack: string, service: string): string {
  const done = compose(stack, ["logs", "--no-color", "--tail", "40", service], {}, true);
  return done.stdout + done.stderr;
}

// ---------------------------------------------------------------------------------------------------
// The arms.
// ---------------------------------------------------------------------------------------------------

type Arm = Record<string, any>;

/**
 * One `oha` pass from inside the stack's own network, as parsed JSON. In-network rather than against the
 * published port: on Windows a published port goes through Docker Desktop's userland proxy, which is a
 * cost neither peer pays in production and which is large enough at these rates to be the thing measured.
 */
function ohaOnce(stack: string, url: string, requests: number, concurrency: number, env: Record<string, string>): Arm {
  const done = compose(
    stack,
    ["run", "--rm", "--no-deps", "oha", "-n", String(requests), "-c", String(concurrency), "--no-tui", "--output-format", "json", url],
    env,
    true,
  );
  if (done.code !== 0) throw new Error(`oha failed against ${url} (${done.code}): ${done.stderr.trim()}`);
  try {
    return JSON.parse(done.stdout);
  } catch (e) {
    throw new Error(`oha's output against ${url} is not JSON (${messageOf(e)}): ${done.stdout.slice(0, 400)}`);
  }
}

/**
 * Warms the peer, then takes `--reps` passes and keeps the best, as the other leg does. The warm pass is
 * discarded for the reason `bun nv bench`'s `measureServer` discards its own: the page cache, opcache's
 * first compile and both JITs' first traces are start-up costs, and a proxied origin that has been up for
 * a while is what this leg is about.
 */
function ohaArm(stack: string, url: string, o: Options, env: Record<string, string>, kind: string, label: string, detail: string): Arm {
  ohaOnce(stack, url, Math.max(50, Math.min(o.requests, 500)), o.concurrency, env);
  const passes: Arm[] = [];
  for (let i = 0; i < o.reps; i++) passes.push(ohaOnce(stack, url, o.requests, o.concurrency, env));
  const rates = passes.map((p) => p.summary.requestsPerSec as number);
  const top = Math.max(...rates);
  const best = passes[rates.indexOf(top)]!;
  const statuses: Record<string, number> = {};
  for (const p of passes) {
    for (const [code, count] of Object.entries((p.statusCodeDistribution ?? {}) as Record<string, number>)) {
      statuses[code] = (statuses[code] ?? 0) + count;
    }
  }
  return {
    kind,
    label,
    detail,
    generator: "oha",
    requests_per_sec: round(top, 1),
    median_requests_per_sec: round(median(rates), 1),
    ms_per_request: round((1000 * o.concurrency) / top, 4),
    latency_ms: best.metrics.latency_ms,
    success_rate: best.summary.successRate,
    status_codes: statuses,
    errors: best.errorDistribution ?? {},
  };
}

/**
 * Arm 4: PHP-FPM over FastCGI with no proxy, driven by `bun nv bench`'s own client. The one arm `oha`
 * cannot take, and not because of a gap in the harness: **FPM speaks FastCGI and nothing else**, so "PHP
 * with no proxy in front" is not an HTTP endpoint that exists. The generator is therefore the other
 * leg's, which makes arm 4 against arm 3 a comparison across two generators -- said in the record's
 * caveats rather than left for a reader to notice.
 */
async function fcgiArm(o: Options): Promise<Arm> {
  const open = () => FcgiConn.open(PHP_FCGI_PORT, CONTAINER_PHP);
  await hammer(open, Math.max(50, Math.min(o.requests, 500)), o.concurrency);
  const rates: number[] = [];
  let reopened = 0;
  for (let i = 0; i < o.reps; i++) {
    const pass = await hammer(open, o.requests, o.concurrency);
    rates.push(o.requests / pass.elapsed);
    reopened += pass.reopened;
  }
  const best = Math.max(...rates);
  return {
    kind: "php-fpm-fcgi-direct",
    label: "php-fpm (fcgi, no proxy)",
    detail: "the FastCGI SAPI, driven as nginx drives it but with nginx removed",
    generator: "nv bench",
    requests_per_sec: round(best, 1),
    median_requests_per_sec: round(median(rates), 1),
    ms_per_request: round((1000 * o.concurrency) / best, 4),
    latency_ms: null,
    success_rate: 1.0,
    status_codes: {},
    errors: {},
    reconnects: reopened,
  };
}

/**
 * One GET from the host, for the agreement check. Never timed. `fetch` rather than `HttpConn`, because
 * nginx in front of PHP-FPM answers chunked and `HttpConn` measures fixed bodies only.
 */
async function httpBody(port: number): Promise<Buffer> {
  const answer = await fetch(`http://127.0.0.1:${port}/`, { headers: { "Accept-Encoding": "identity" }, signal: AbortSignal.timeout(30_000) });
  return Buffer.from(await answer.arrayBuffer());
}

/** Arm 4's answer, through the same client that times it. */
async function fcgiBody(): Promise<Buffer> {
  const conn = await FcgiConn.open(PHP_FCGI_PORT, CONTAINER_PHP);
  try {
    return await conn.request();
  } finally {
    conn.close();
  }
}

// ---------------------------------------------------------------------------------------------------
// The record.
// ---------------------------------------------------------------------------------------------------

/**
 * A sha256 per tunable file, so a number can be tied to the configuration that produced it. Digests and
 * the effective values below them, rather than the files' full text: the artifact keeps a hundred runs,
 * the files are mostly comments, and a hash plus the commit recovers the exact bytes from git whenever a
 * row is actually questioned.
 */
function digests(): Record<string, string> {
  const out: Record<string, string> = {};
  for (const path of TUNABLES) {
    if (existsSync(path) && statSync(path).isFile()) {
      out[rel(path).replaceAll("\\", "/")] = createHash("sha256").update(readFileSync(path)).digest("hex").slice(0, 16);
    }
  }
  return out;
}

/** The opcache keys that would move a number, parsed out of the committed ini. */
function effectivePhp(): Record<string, string> {
  const wanted = new Set(["opcache.enable", "opcache.jit", "opcache.jit_buffer_size", "opcache.validate_timestamps", "opcache.memory_consumption"]);
  const out: Record<string, string> = {};
  for (const raw of splitlines(readFileSync(join(BENCH_DIR, "php", "opcache.ini"), "utf8"))) {
    const line = raw.trim();
    if (line.startsWith(";") || !line.includes("=")) continue;
    const at = line.indexOf("=");
    const key = line.slice(0, at).trim();
    if (wanted.has(key)) out[key] = line.slice(at + 1).trim();
  }
  return out;
}

// ---------------------------------------------------------------------------------------------------

async function main(o: Options): Promise<number> {
  requireDocker();

  if (o.down) {
    for (const stack of [PHP_STACK, NVS_STACK]) down(stack);
    console.log("both stacks are down");
    return 0;
  }

  const backendCpus = o.backendCpus ?? (o.arm === "fair" ? 1 : 4);
  if (o.arm === "fair" && backendCpus !== 1) {
    throw new Fail(`--arm fair is one cpu against one cpu; --backend-cpus ${backendCpus} is the deployed arm and must say so`);
  }
  if (o.requests < 1 || o.concurrency < 1 || o.reps < 1) throw new Fail("--requests, --concurrency and --reps must each be at least 1");

  const commit = version("git", ["rev-parse", "--short", "HEAD"]);

  // The pool is sized by **concurrency**, never by the CPU budget. `php/pool.conf.in` owns why in full;
  // the short of it is that a FastCGI worker bound to a keep-alive connection accepts nothing else, so one
  // worker per core does not run slowly at concurrency 8 -- it hangs. The floor of 96 clears the 64 idle
  // upstream connections nginx can hold.
  const phpChildren = Math.max(2 * o.concurrency, 96);
  renderPool(phpChildren);

  // The same budget PHP's container gets, in both arms. `nvs serve` starts one worker per CPU the
  // container can see, as FPM starts its whole pool, so the `cpus:` quota and not either worker count
  // is what bounds the two peers equally.
  const nvsEnv: Record<string, string> = { NVS_CPUS: String(backendCpus), NVS_BUILD_COMMIT: commit };
  const phpEnv: Record<string, string> = { BACKEND_CPUS: String(backendCpus) };
  if (o.nvsBin !== null) {
    stageBinary(o.nvsBin, o.allowDebug);
    nvsEnv.NVS_STAGE = "prebuilt";
  }

  console.log(`case ${CASE}, arm ${o.arm}: ${o.requests} requests over ${o.concurrency} connection(s), ${o.reps} rep(s), best of reps`);
  console.log(`  backend budget: php ${backendCpus} cpu / ${phpChildren} worker(s), nvs serve ${backendCpus} cpu / one worker per visible cpu; nginx 4 cpu on both sides`);
  console.log();

  const arms: Record<string, Arm> = {};
  const bodies = new Map<string, Buffer>();

  // ---- the PHP stack, alone on the box -------------------------------------------------------------
  //
  // § Why the origin is measured before its proxy exists.
  //
  // The origin arm runs with only the backend up, and the proxy is started afterwards for the proxied
  // arm. On the PHP side this is not a preference -- it is the difference between a number and a hang.
  // In the fair arm FPM has **one** worker, and nginx holds its upstream FastCGI connection open
  // (`fastcgi_keep_conn on`, without which its `keepalive` pool is inert). One worker bound to one
  // held-open connection is a worker that never accepts a second one, so a direct FastCGI client is
  // accepted by the kernel's backlog and then waits for a responder that will never come. It presents as
  // a 30-second socket timeout while the container's own log shows healthy 200s going past, which is
  // about as misleading as a failure gets.
  //
  // The Novis stack is phased the same way and does not need to be: `nvs serve` accepts every connection
  // it is offered. It is done anyway because arm 3 against arm 4 is only a comparison if the two were
  // taken under the same conditions, and "the proxy was running for one of them" is a difference.
  try {
    console.log("  bringing up php-fpm alone, for the origin arm ...");
    up(PHP_STACK, phpEnv, "php");
    bodies.set("php-fpm (fcgi, no proxy)", await fcgiBody());
    arms["php-fpm-fcgi-direct"] = await fcgiArm(o);

    console.log("  bringing up nginx in front of it ...");
    up(PHP_STACK, phpEnv, "nginx");
    bodies.set("nginx -> php-fpm", await httpBody(PHP_NGINX_PORT));
    arms["nginx-php-fpm"] = ohaArm(PHP_STACK, "http://nginx/", o, phpEnv, "nginx-php-fpm", "nginx -> php-fpm", "PHP 8.5.9 FPM with opcache and tracing JIT");
  } catch (e) {
    throw new Fail(`${messageOf(e)}\nphp said:\n${logsOf(PHP_STACK, "php")}`);
  } finally {
    // Unconditional, and `--keep-up` deliberately does not reach it: the next stack is about to take the
    // same cores, and leaving this one running would make every number after it a measurement of two
    // stacks sharing a box. `--keep-up` keeps the *last* stack, which is the one there is any reason to
    // poke at afterwards.
    down(PHP_STACK);
  }

  // ---- the Novis stack, alone on the box -----------------------------------------------------------
  try {
    console.log("  bringing up nvs serve alone (the first run builds the image) ...");
    up(NVS_STACK, nvsEnv, "nvs");
    bodies.set("nvs serve (no proxy)", await httpBody(NVS_DIRECT_PORT));
    arms["nvs-serve-direct"] = ohaArm(NVS_STACK, "http://nvs:8080/", o, nvsEnv, "nvs-serve-direct", "nvs serve (no proxy)", "the same origin with nginx removed");

    console.log("  bringing up nginx in front of it ...");
    up(NVS_STACK, nvsEnv, "nginx");
    bodies.set("nginx -> nvs serve", await httpBody(NVS_NGINX_PORT));
    arms["nginx-nvs-serve"] = ohaArm(
      NVS_STACK,
      "http://nginx/",
      o,
      nvsEnv,
      "nginx-nvs-serve",
      "nginx -> nvs serve",
      `\`rule:http-server/two-deployments-and-nothing-a-proxy-owns\`'s proxied origin, ${backendCpus} cpu`,
    );
  } catch (e) {
    throw new Fail(`${messageOf(e)}\nnvs said:\n${logsOf(NVS_STACK, "nvs")}`);
  } finally {
    if (!o.keepUp) down(NVS_STACK);
  }

  // ---- agreement, before any number is reported ----------------------------------------------------
  // The other leg's rule, and it is not a formality: a peer answering 404 fast is the easiest way for a
  // benchmark to report a large win. Every arm has to have sent the same bytes.
  if (new Set([...bodies.values()].map((b) => b.toString("latin1"))).size > 1) {
    console.log();
    console.log("DIFF: the arms did not answer with the same bytes, so no number here stands");
    for (const [label, body] of bodies) console.log(`  ${label} sent ${bytesRepr(body)}`);
    return 1;
  }

  // ---- report --------------------------------------------------------------------------------------
  const order = ["nginx-php-fpm", "nginx-nvs-serve", "nvs-serve-direct", "php-fpm-fcgi-direct"];
  const width = Math.max(...order.map((k) => arms[k]!.label.length));
  console.log();
  for (const key of order) {
    const a = arms[key]!;
    const tail = a.latency_ms ? `  p99 ${fixed(a.latency_ms.p99, 3).padStart(7)} ms` : "";
    console.log(
      `  ${a.label.padEnd(width)}  ${grouped(a.requests_per_sec, 0).padStart(10)} requests/sec  ${fixed(a.ms_per_request, 3).padStart(8)} ms/request${tail}`,
    );
  }

  const shipped = arms["nginx-nvs-serve"]!.requests_per_sec / arms["nginx-php-fpm"]!.requests_per_sec;
  const origin = arms["nvs-serve-direct"]!.requests_per_sec / arms["php-fpm-fcgi-direct"]!.requests_per_sec;
  console.log();
  console.log(`  as shipped, behind nginx: nvs serve is ${fixed(shipped, 2)}x php-fpm`);
  console.log(`  origin against origin:    nvs serve is ${fixed(origin, 2)}x php-fpm  (two generators)`);

  const caveats = [
    "nginx fronts both peers, which is the only deployment either has -- FPM speaks FastCGI " +
      "and nvs serve is `rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s proxied origin; the proxy's cost is in both numbers",
    "arm 4 is driven by nv bench's FastCGI client and not by oha, because FPM has no " +
      "HTTP origin to point a load generator at; origin-against-origin therefore crosses two " +
      "generators and is the weaker of the two ratios",
    "this row shares no inputs with benches/serve.json -- different OS, generator, topology " +
      "and CPU budget -- and no arithmetic between the two files is meaningful",
  ];
  if (o.arm === "deployed") {
    caveats.push(
      `deployed arm: both containers are given ${backendCpus} cpus and both use all of them -- ` +
        "php-fpm through its process manager, nvs serve through one worker per CPU the container " +
        "can see. Each peer starts more workers than the quota has cpus, so the quota is the bound",
    );
  } else {
    caveats.push(
      "fair arm: one cpu to each backend container, so the number is runtime against " +
        `runtime. The PHP pool is ${phpChildren} workers, sized from the concurrency rather ` +
        "than from the budget, because a FastCGI worker is bound to the keep-alive connection " +
        "it holds and a pool smaller than the open connections stops rather than slows",
    );
  }
  console.log();
  for (const caveat of caveats) console.log(`  note: ${caveat}`);

  const record = {
    date: stamp(),
    commit,
    case: CASE,
    arm: o.arm,
    requests: o.requests,
    concurrency: o.concurrency,
    reps: o.reps,
    backend_cpus: backendCpus,
    php_children: phpChildren,
    nginx_cpus: 4,
    ratio_shipped: round(shipped, 4),
    ratio_origin: round(origin, 4),
    caveats,
    arms,
    images: { php: "php:8.5.9-fpm", nginx: "nginx:1.29.3-alpine", oha: "ghcr.io/hatoo/oha@sha256:3ec3dbf5" },
    php_ini: effectivePhp(),
    config_digests: digests(),
    nvs_source: o.nvsBin !== null ? "prebuilt" : "built from this checkout",
    host: host(),
  };
  console.log();
  if (o.record) writeServeRecord(o.record, record);
  else console.log("  not recorded: pass --record PATH to append this run to a JSON array");
  return 0;
}

export async function run(args: string[]): Promise<number> {
  let o: Options | null;
  try {
    o = options(args);
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv bench-proxied: error: ${e.message}`);
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
