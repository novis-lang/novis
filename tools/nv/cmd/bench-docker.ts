// The Docker half of the containerised benchmark legs: the daemon check, `docker compose` and `oha`.
// Every leg that runs in containers imports it. It is not a command: `main.ts` names no entry for it.
//
// Each leg owns its compose files, its arms and its record. This file owns only how a stack is brought
// up and torn down and how one `oha` arm is taken from inside a stack's network, so the legs measure the
// same way. `benches/proxied/README.md` is why a stack is shaped the way it is.

import { basename } from "node:path";
import { Fail, median, round } from "./bench.ts";

export const messageOf = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Fails before anything is built if there is no daemon, naming what is missing. */
export function requireDocker(): void {
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
export function compose(stack: string, args: string[], env: Record<string, string>, capture: boolean): Done {
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
 * in a leg's compose files carries a healthcheck: without it the first requests of a run land on a server
 * that is still initialising, and they land only in the first rep -- which reads as a warm-up effect
 * rather than as the harness's own bug.
 *
 * **Services are named rather than defaulted, and that is load-bearing.** The origin arms are measured
 * with the proxy not yet started; § *Why the origin is measured before its proxy exists* in
 * `bench-proxied`'s `main` is the reason, and it is a correctness one rather than a tidiness one.
 *
 * **`--build` on every call**, because a locally built image is a tag compose reuses whenever it exists:
 * without the flag a run measures whatever binary the image was first built from and records this
 * checkout's commit beside it. Docker's layer cache makes the rebuild of an unchanged tree cheap, and the
 * services that only name an image have nothing to build.
 */
export function up(stack: string, env: Record<string, string>, ...services: string[]): void {
  const done = compose(stack, ["up", "-d", "--wait", "--build", ...services], env, false);
  if (done.code !== 0) throw new Error(`\`docker compose -f ${basename(stack)} up ${services.join(" ")}\` failed with ${done.code}`);
}

/** Tears a stack down, including its network. Never throws: this runs in a `finally`. */
export function down(stack: string): void {
  try {
    compose(stack, ["down", "-v", "--remove-orphans"], {}, true);
  } catch {
    // The stack may already be gone, or docker with it; neither is this run's failure.
  }
}

/** What a service printed, for the failure message. A generator only ever sees a reset socket. */
export function logsOf(stack: string, service: string): string {
  const done = compose(stack, ["logs", "--no-color", "--tail", "40", service], {}, true);
  return done.stdout + done.stderr;
}

// ---------------------------------------------------------------------------------------------------
// One `oha` arm.
// ---------------------------------------------------------------------------------------------------

export type Arm = Record<string, any>;

/** How hard an arm is driven: the leg's `--requests`, `--concurrency` and `--reps`. */
export interface Load {
  requests: number;
  concurrency: number;
  reps: number;
}

/**
 * One `oha` pass from inside the stack's own network, as parsed JSON. In-network rather than against the
 * published port: on Windows a published port goes through Docker Desktop's userland proxy, which is a
 * cost no peer pays in production and which is large enough at these rates to be the thing measured.
 * The stack names its generator service `oha`.
 */
export function ohaOnce(stack: string, url: string, requests: number, concurrency: number, env: Record<string, string>): Arm {
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
 * Warms the peer, then takes `reps` passes and keeps the best, as `bun nv bench --serve-vs-fpm` does. The
 * warm pass is discarded for the reason `bun nv bench`'s `measureServer` discards its own: the page cache,
 * opcache's first compile and the JITs' first traces are start-up costs, and a proxied origin that has
 * been up for a while is what these legs are about.
 */
export function ohaArm(stack: string, url: string, o: Load, env: Record<string, string>, kind: string, label: string, detail: string): Arm {
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
