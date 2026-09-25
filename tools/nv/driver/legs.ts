// The acceptance sweep's two Linux legs, run after the native sweep is green: the WSL leg and the valgrind
// sweep. `linuxLegs` runs both and returns the ledger's line, or "" when both are green, skipped or
// answered by the memo.
//
// **The WSL leg** is Windows only. It builds the CLI inside the default WSL distro, into the goal's
// `env.wsl.targetDir`, and runs every fixture the sweep reached and every `nvs-suite` check against that
// Linux build. A JIT is where a calling-convention divergence hides, and this leg is what finds one. The
// build is `cargo build --quiet` over the whole workspace through `bash -lc`, so cargo comes from the
// distro's login `PATH`. The build and every fixture run from a copy of the working tree on the distro's
// own disk, which `tools/nv/driver/mirror.ts` brings up to date before each build. The build may start
// early with `startWslBuild`, beside the cargo tier, and `linuxLegs` then waits for it.
//
// That sync deletes every ignored file in the copy, so a file a `setup` check writes, such as a fixture's
// SQLite database, is gone after it. The leg therefore runs the goal's `setup` checks itself, with the
// Linux build in place of `{nvs}` and inside the copy, before any fixture and before the valgrind sweep. A
// `setup` check that names no `{nvs}` has nothing to run in the distro, and is skipped with a trace line.
// A red one is reported like a red fixture. Copying the Windows file over instead would carry the Windows
// run's rows into the Linux one.
//
// **The valgrind sweep** runs each fixture in the plan's file list under `valgrind --leak-check=full`
// over the Linux build: inside WSL on Windows, directly on Linux. It runs a fixture the sweep reached,
// minus `env.valgrind.skip`, with no arguments, as `nvs run <file>`. `examples/limits.nvs` runs with
// `tools/valgrind-limits.toml` layered over `nvs.toml`, because its cost under memcheck is the distance
// to a ceiling its leak verdict does not depend on. Naming any `--config` turns off the search for
// `./nvs.toml`, so the repository's own file is named first. Only exit 97 is a leak: a fixture's own exit
// status passes through valgrind, and a fixture that ends in a fatal error by design must not read as
// one. A fixture that timed out, or a distro with no valgrind, is red too, since nothing was checked.
// The sweep runs several fixtures at once, as wide as `tools/nv/lib/machine.ts` decides for the context
// (`NVS_VALGRIND_JOBS` overrides it), and submits the one that took longest last time first. It never
// stops early: every leaking fixture is named, earliest in the plan's order first.
//
// A platform with no WSL runs the valgrind sweep natively when `valgrind` is on `PATH`, and skips it
// with a trace line otherwise. Windows with no `wsl.exe`, or a goal that names no WSL target directory,
// skips both with a trace line. Neither skip is a failure.
//
// **The origin.** `examples/http.nvs` talks to `http://127.0.0.1:8099`. Inside WSL, with its default NAT
// networking, the listener `tools/nv/driver/origin.ts` holds on the Windows side is not reachable at that
// address, so the WSL leg holds an origin of its own inside the distro. The distro has no `bun`, and the
// tools add no new Python, so the origin is the Linux `nvs` the leg just built: `nvs serve` over a
// short program this module writes under `.agent-tmp/legs/` and deletes when the origin stops. The program answers `/ok` with `ok`
// and every other path with 404, each after the same 25ms hold `origin.ts` uses, so the example's last
// line is decided the same way on both legs. It runs under an empty configuration file, so nothing in the
// repository's `nvs.toml` (a queue, a schedule) starts beside it. When 127.0.0.1:8099 already answers
// inside the distro, which is the case under mirrored networking, that listener is left alone. The origin
// stops when its `bash` reads the end of its standard input. On Linux the sweep holds `origin.ts` itself.
//
// Each leg is remembered in the memo under its own id (`LEGS` in `tools/nv/keys/checks.ts`), and only
// when it is green on a sweep with the floor gate open: a leg over the goal's own fixtures alone is not
// the leg the memo names. The build is skipped only when the memo answers both legs, since either one
// still to run needs the binary.
//
// Every process this module starts goes through `LegsSeams`, so `tools/nv/test/legs.test.ts` drives it
// with recorded outcomes and no WSL.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import * as machine from "../lib/machine.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { type Check, type GreenMemo, type Outcome, PROGRAM_KINDS, allReds, firstErrLine, judgeCommand, judgeProgram, judgeTests, programFailLine } from "./accept.ts";
import { mirrorPath, q, syncMirror, wslPath } from "./mirror.ts";

export { wslPath };
import { type Origin, DELAY_MS, HOST, PORT, holdOrigin } from "./origin.ts";

export { q };

export type LegName = "wsl leg" | "valgrind sweep";

/** Where a leg's processes run: inside the default WSL distro, or on this machine. */
export type Where = "wsl" | "native";

export interface LegsOptions {
  /** The program checks the sweep reached, in plan order (kind in PROGRAM_KINDS). */
  programs: Check[];
  /** The `nvs-suite` checks the sweep reached. */
  suites: Check[];
  /** Every `setup` check of the goal, reached by the sweep or held back from it, which the WSL leg runs inside its copy. */
  setups: Check[];
  /** The plan's fixture list and valgrind skip list (goal record `files`, `env.valgrind.skip`). */
  files: string[];
  valgrindSkip: string[];
  /** `env.wsl.targetDir`, or null when the goal names none (then no WSL leg). */
  wslTarget: string | null;
  /** The floor gate is open: only then is a green leg remembered, since a leg over the goal's own fixtures alone is not the leg the memo names. */
  gateOpen: boolean;
  memo: GreenMemo;
  /** Each leg's key over the tree, or null (never remembered). */
  key: (leg: LegName) => string | null;
  /** Consult no memo. */
  full: boolean;
  label: (stage: number) => string;
  /** A process is starting / a step is reached, for the console. */
  onRun?: (what: string) => void;
  /** How many steps the legs will take, said once they know: the build, then each setup check, fixture, suite and valgrind run still to judge. */
  onPlan?: (steps: number) => void;
  /** One of the steps `onPlan` counted is finished. */
  onStep?: () => void;
  /** The processes, replaced under test. Each one left out is the real one. */
  seams?: Partial<LegsSeams>;
}

/** Everything the legs start or ask the machine, as one replaceable set. */
export interface LegsSeams {
  platform: NodeJS.Platform;
  /** Whether `wsl.exe` is on `PATH`. */
  hasWsl(): boolean;
  /** Whether `valgrind` is on this machine's `PATH`. */
  hasValgrind(): boolean;
  /** One `bash -lc` line where the leg runs. A process that cannot start comes back as exit -1. */
  shell(where: Where, line: string, timeoutMs: number): Promise<Outcome>;
  /** Brings the distro's copy of the tree at `mirror` to the working tree: "" when it did, else why not. */
  sync(mirror: string): Promise<string>;
  /** The origin on 127.0.0.1:8099 where the leg runs, up until `close`. `binary` is the leg's own CLI. */
  origin(where: Where, binary: string, repo: string): Promise<Origin>;
  /** The machine's facts for a context, probing through `runner` (the sample runs `sample` once). */
  profile(context: Where, runner: machine.Runner, sample: string): machine.Entry;
  /** How many fixtures to run at once in a context. */
  jobs(context: Where, ceiling: number): number;
  remember(context: Where, fields: Record<string, unknown>): void;
  /** Milliseconds, for the per-fixture cost the next sweep orders by. */
  now(): number;
}

/** A build or a fixture run: half an hour, the sweep's own ceiling. */
const TIMEOUT_MS = 1800 * 1000;

/** The exit status valgrind is told to use for an error it found, apart from any a fixture uses itself. */
export const VALGRIND_ERROR = 97;

/** A fixture that runs under a configuration layered over `nvs.toml` in the valgrind sweep, and which. */
export const VALGRIND_CONFIG: Record<string, string> = { "examples/limits.nvs": "tools/valgrind-limits.toml" };

/** The questions a check's `needs` may ask of a leg. The WSL leg is a Linux build, which has Unix-domain sockets. */
export const LEG_NEEDS: Record<string, (leg: { afUnix: boolean }) => boolean> = {
  "af-unix": (leg) => leg.afUnix,
};

/** The memo's stand-in for a whole leg: an id, remembered like a check. */
export function legSpec(leg: LegName): Check {
  return { id: leg, kind: "leg", stage: 0 };
}

/** The valgrind line for one fixture, run from the repository root where `repo` names it. */
export function valgrindLine(repo: string, binary: string, file: string): string {
  const over = VALGRIND_CONFIG[file];
  const config = over === undefined ? "" : `--config nvs.toml --config ${q(over)} `;
  return (
    `cd ${q(repo)} && valgrind --error-exitcode=${VALGRIND_ERROR} --leak-check=full ` +
    `--errors-for-leak-kinds=definite --suppressions=tools/valgrind.supp -q ` +
    `${q(binary)} ${config}run ${q(file)}`
  );
}

/** The build line for the WSL leg. */
export function wslBuildLine(repo: string, targetDir: string): string {
  return `cd ${q(repo)} && CARGO_TARGET_DIR=${q(targetDir)} cargo build --quiet`;
}

/** The valgrind sweep's reds as one line: the first whole, then the others by name. */
export function valgrindFailLine(fails: string[]): string {
  if (fails.length === 0) return "";
  if (fails.length === 1) return fails[0]!;
  return `${fails[0]}  (and ${fails.length - 1} more: ${fails.slice(1).map((x) => x.split(":")[0]).join(", ")})`;
}

// ---- the real processes ---------------------------------------------------------------------------

async function capture(argv: string[], timeoutMs: number): Promise<Outcome> {
  try {
    const r = await run(argv, { cwd: ROOT, timeoutMs });
    return { code: r.timedOut ? -1 : r.code, out: r.stdout, err: r.timedOut ? `timed out after ${timeoutMs / 1000}s` : r.stderr };
  } catch (e) {
    return { code: -1, out: "", err: String((e as Error).message ?? e) };
  }
}

/**
 * `bash -lc <line>` where the leg runs. `wsl.exe --exec` starts `bash` directly. `wsl.exe --` would first
 * hand the line to the distro's default shell, which expands every `$` and strips every quote in it
 * before `bash` sees it.
 */
function argvFor(where: Where, line: string): string[] {
  return where === "wsl" ? ["wsl.exe", "--exec", "bash", "-lc", line] : ["bash", "-lc", line];
}

/** The origin program `nvs serve` runs inside the distro, and the empty configuration it runs under. */
const ORIGIN_DIR = ".agent-tmp/legs";
const ORIGIN_PROGRAM = `<?nvs
Core\\Time::sleep(${DELAY_MS}ms);
if (Core\\Request::path() == '/ok') {
    echo 'ok';
} else {
    Core\\Response::setStatus(404);
    echo 'not found';
}
`;

/** `nvs serve` over the origin program inside the distro, up until its standard input ends. */
async function wslOrigin(binary: string, repo: string): Promise<Origin> {
  const taken = await capture(argvFor("wsl", `(exec 3<>/dev/tcp/${HOST}/${PORT}) 2>/dev/null`), 30_000);
  if (taken.code === 0) return { line: `wsl origin: http://${HOST}:${PORT} already has a listener -- leaving it alone`, close() {} };
  try {
    mkdirSync(join(ROOT, ORIGIN_DIR), { recursive: true });
    writeFileSync(join(ROOT, ORIGIN_DIR, "origin.nvs"), ORIGIN_PROGRAM);
    writeFileSync(join(ROOT, ORIGIN_DIR, "origin.toml"), "");
  } catch (e) {
    return { line: `wsl origin: cannot write ${ORIGIN_DIR} -- ${(e as Error).message}`, close() {} };
  }
  const serve =
    `${q(binary)} serve --no-init --config ${ORIGIN_DIR}/origin.toml --listen ${HOST}:${PORT} ${ORIGIN_DIR}/origin.nvs`;
  const line = `cd ${q(repo)} && { ${serve} & p=$!; read -r _; kill $p 2>/dev/null; wait $p; }`;
  let child: ReturnType<typeof Bun.spawn<"pipe", "pipe", "pipe">>;
  try {
    child = Bun.spawn(argvFor("wsl", line), { cwd: ROOT, stdin: "pipe", stdout: "pipe", stderr: "pipe" });
  } catch (e) {
    return { line: `wsl origin: cannot start -- ${(e as Error).message}`, close() {} };
  }
  const close = () => {
    try {
      child.stdin.end();
    } catch {}
    const timer = setTimeout(() => child.kill(), 15_000);
    // The program and its configuration are this origin's scratch, so they go with it.
    void child.exited.then(() => {
      clearTimeout(timer);
      rmSync(join(ROOT, ORIGIN_DIR), { recursive: true, force: true });
    });
  };
  // Readiness is the server's own `listening on` line. The reader keeps draining for the process's life,
  // so a full pipe can never stall the server.
  const ready = await new Promise<string | null>((resolve) => {
    const timer = setTimeout(() => resolve(null), 120_000);
    void (async () => {
      const decoder = new TextDecoder();
      let text = "";
      let said = false;
      for await (const chunk of child.stdout) {
        if (said) continue;
        text += decoder.decode(chunk, { stream: true });
        const hit = text.split("\n").find((l) => l.startsWith("listening on"));
        if (hit !== undefined) {
          said = true;
          clearTimeout(timer);
          resolve(hit.trim());
        }
      }
      clearTimeout(timer);
      resolve(null);
    })();
  });
  if (ready !== null) return { line: `wsl origin: ${ready}`, close };
  const err = (await Promise.race([new Response(child.stderr).text(), Bun.sleep(2000).then(() => "")])).trim();
  close();
  return { line: `the wsl leg has no origin on ${PORT} -- ${err.split("\n").pop() || "it never said it was listening"}`, close() {} };
}

function syncRunner(where: Where): machine.Runner {
  if (where === "native") return machine.bash;
  return (line) => {
    const [exe, ...args] = argvFor("wsl", line);
    const p = spawnSync(exe!, args, { encoding: "utf8", timeout: 300_000 });
    if (p.error) return { code: 1, text: "" };
    return { code: p.status ?? 1, text: (p.stdout ?? "") + (p.stderr ?? "") };
  };
}

const REAL: LegsSeams = {
  platform: process.platform,
  hasWsl: () => Bun.which("wsl.exe") !== null,
  hasValgrind: () => Bun.which("valgrind") !== null,
  shell: (where, line, timeoutMs) => capture(argvFor(where, line), timeoutMs),
  sync: (mirror) => syncMirror(mirror),
  origin: (where, binary, repo) => (where === "wsl" ? wslOrigin(binary, repo) : holdOrigin()),
  profile: (context, runner, sample) => machine.profile(context, { probe: () => machine.posixProbe(runner, sample) }),
  jobs: (context, ceiling) => machine.jobs(context, { ceiling, envs: ["NVS_VALGRIND_JOBS"] }),
  remember: (context, fields) => machine.remember(context, fields),
  now: () => performance.now(),
};

function seamsOf(o: { seams?: Partial<LegsSeams> }): LegsSeams {
  return { ...REAL, ...o.seams };
}

// ---- the legs -------------------------------------------------------------------------------------

/** One leg's build, started early and awaited by `linuxLegs`, by target directory. */
const inflight = new Map<string, Promise<string>>();

function answered(o: Pick<LegsOptions, "memo" | "key" | "full">, leg: LegName): boolean {
  return !o.full && o.memo.answers(legSpec(leg), o.key(leg));
}

/** The copy of this checkout the WSL leg builds and runs from, for a target directory. */
function wslRepo(targetDir: string): string {
  return mirrorPath(targetDir, wslPath(ROOT));
}

async function buildWsl(s: LegsSeams, targetDir: string): Promise<string> {
  const synced = await s.sync(wslRepo(targetDir));
  if (synced !== "") return `the wsl copy of the tree could not be synced -- ${synced}`;
  const r = await s.shell("wsl", wslBuildLine(wslRepo(targetDir), targetDir), TIMEOUT_MS);
  return r.code === 0 ? "" : `the wsl build failed -- ${firstErrLine(r)}`;
}

/**
 * Starts the WSL build in the background, beside the cargo tier, when `linuxLegs` will need it: the floor
 * gate is open, the sweep reaches a fixture, WSL is here and the memo does not answer both legs. The
 * build and nothing after it: the fixtures and the sweep reach the same database servers the cargo tier's
 * tests do, so they wait for `linuxLegs`.
 */
export function startWslBuild(o: Omit<LegsOptions, "suites" | "setups" | "files" | "valgrindSkip" | "label">): boolean {
  const s = seamsOf(o);
  if (!o.gateOpen || o.programs.length === 0 || o.wslTarget === null || s.platform !== "win32" || !s.hasWsl()) return false;
  if (answered(o, "wsl leg") && answered(o, "valgrind sweep")) return false;
  if (inflight.has(o.wslTarget)) return true;
  o.onRun?.("wsl build started in the background");
  inflight.set(o.wslTarget, buildWsl(s, o.wslTarget));
  return true;
}

/** Runs the WSL leg (Windows with WSL) and the valgrind sweep. Returns "" when both are green, skipped or remembered, else the ledger line: the WSL leg's fixture reds as one `programFailLine`, then valgrind's `valgrind <file>: exit 97 -- <first error line>  (and N more: ...)`. */
export async function linuxLegs(o: LegsOptions): Promise<string> {
  const s = seamsOf(o);
  const say = (what: string) => o.onRun?.(what);
  if (o.programs.length === 0) {
    say("wsl leg and valgrind sweep skipped -- the sweep reached no fixture");
    return "";
  }
  const reached = new Set(o.programs.map((c) => c.file ?? ""));
  const targets = o.files.filter((f) => reached.has(f) && !o.valgrindSkip.includes(f));

  if (s.platform !== "win32") {
    if (answered(o, "valgrind sweep")) {
      say("valgrind sweep green on these inputs -- not run");
      return "";
    }
    if (!s.hasValgrind()) {
      say("valgrind sweep skipped -- no valgrind on this platform");
      return "";
    }
    if (targets.length === 0) return "";
    o.onPlan?.(1 + targets.length);
    say("cargo build");
    const built = await s.shell("native", `cd ${q(ROOT)} && cargo build --quiet`, TIMEOUT_MS);
    if (built.code !== 0) return `the native build failed -- ${firstErrLine(built)}`;
    o.onStep?.();
    const origin = await s.origin("native", join(ROOT, "target", "debug", "nvs"), ROOT);
    say(`native ${origin.line}`);
    try {
      return await valgrindSweep(o, s, "native", ROOT, join(ROOT, "target", "debug", "nvs"), targets);
    } finally {
      origin.close();
    }
  }

  if (o.wslTarget === null) {
    say("wsl leg and valgrind sweep skipped -- the goal names no env.wsl.targetDir");
    return "";
  }
  const legGreen = answered(o, "wsl leg");
  const sweepGreen = answered(o, "valgrind sweep");
  if (legGreen && sweepGreen) {
    inflight.delete(o.wslTarget);
    say("wsl leg and valgrind sweep both green on these inputs -- neither is rebuilt");
    return "";
  }
  if (!s.hasWsl()) {
    say("wsl leg and valgrind sweep skipped -- no wsl.exe on this machine");
    return "";
  }

  const repo = wslRepo(o.wslTarget);
  const binary = `${o.wslTarget}/debug/nvs`;
  o.onPlan?.(1 + o.setups.length + (legGreen ? 0 : o.programs.length + o.suites.length) + (sweepGreen ? 0 : targets.length));
  say("wsl build");
  const pending = inflight.get(o.wslTarget);
  inflight.delete(o.wslTarget);
  const buildFail = await (pending ?? buildWsl(s, o.wslTarget));
  if (buildFail !== "") return buildFail;
  o.onStep?.();

  // The origin's program is written into this checkout, so it runs from the mount, not from the copy.
  const origin = await s.origin("wsl", binary, wslPath(ROOT));
  say(origin.line);
  try {
    const reds: string[] = [];
    const setupFails = await wslSetup(o, s, repo, binary);
    if (legGreen) {
      say("wsl fixtures green on these inputs -- not run");
      if (setupFails.length > 0) reds.push(programFailLine(setupFails, o.label));
    } else {
      const line = await wslFixtures(o, s, repo, binary, setupFails);
      if (line !== "") reds.push(line);
      else if (o.gateOpen) o.memo.remember(legSpec("wsl leg"), o.key("wsl leg"));
    }
    if (sweepGreen) say("valgrind sweep green on these inputs -- not run");
    else if (targets.length > 0) {
      const line = await valgrindSweep(o, s, "wsl", repo, binary, targets);
      if (line !== "") reds.push(line);
    }
    return reds.length === 0 ? "" : allReds(reds);
  } finally {
    origin.close();
  }
}

/** The goal's `setup` checks, run inside the copy with the Linux build: each red one, judged as the sweep judges it. */
async function wslSetup(o: LegsOptions, s: LegsSeams, repo: string, binary: string): Promise<{ c: Check; fail: string }[]> {
  const fails: { c: Check; fail: string }[] = [];
  for (const c of o.setups) {
    const label = `wsl ${c.name ?? c.id} [${o.label(c.stage)}]`;
    const argv = c.argv ?? [];
    if (!argv.includes("{nvs}")) {
      o.onRun?.(`${label} -- skipped: it does not run nvs`);
      o.onStep?.();
      continue;
    }
    const line = argv.map((a) => (a === "{nvs}" ? q(binary) : q(a))).join(" ");
    o.onRun?.(`wsl setup: ${argv.join(" ")}`);
    const r = await s.shell("wsl", `cd ${q(`${repo}/${c.cwd ?? "."}`)} && ${line}`, TIMEOUT_MS);
    const fail = judgeCommand(c, r, label);
    if (fail !== "") fails.push({ c, fail });
    o.onStep?.();
  }
  return fails;
}

/**
 * Every fixture and suite the sweep reached, against the Linux build: every red, as one line, after any
 * red `setup` check handed in. Checks that name one command line share one run, as they do in the sweep,
 * and each judges its own `want` or cases against it.
 */
async function wslFixtures(o: LegsOptions, s: LegsSeams, repo: string, binary: string, setupFails: { c: Check; fail: string }[]): Promise<string> {
  const leg = { afUnix: true };
  const fails: { c: Check; fail: string }[] = [...setupFails];
  const runs = new Map<string, Outcome>();
  const shared = async (line: string, what: string): Promise<Outcome> => {
    const seen = runs.get(line);
    if (seen !== undefined) return seen;
    o.onRun?.(what);
    const r = await s.shell("wsl", line, TIMEOUT_MS);
    runs.set(line, r);
    return r;
  };
  for (const c of o.programs) {
    const fail = await wslProgram(o, c, leg, shared, repo, binary);
    if (fail !== "") fails.push({ c, fail });
    o.onStep?.();
  }
  for (const c of o.suites) {
    const label = `wsl ${c.name ?? c.id} [${o.label(c.stage)}]`;
    const args = c.args ?? [];
    const r = await shared(`cd ${q(repo)} && ${q(binary)} ${args.map(q).join(" ")}`, `wsl nvs ${args.join(" ")}`);
    const v = judgeTests(c, r, label, (rel) => existsSync(join(ROOT, rel)));
    if (v.fail !== "") fails.push({ c, fail: v.fail });
    o.onStep?.();
  }
  if (fails.length === 0) return "";
  const ordered = fails.map((f, i) => ({ f, i })).sort((a, b) => a.f.c.stage - b.f.c.stage || a.i - b.i).map((x) => x.f);
  return programFailLine(ordered, o.label);
}

/** One fixture on the WSL leg: its red line, or "" when it is green or the leg has not what it `needs`. */
async function wslProgram(
  o: LegsOptions,
  c: Check,
  leg: { afUnix: boolean },
  shared: (line: string, what: string) => Promise<Outcome>,
  repo: string,
  binary: string,
): Promise<string> {
  const label = `wsl ${c.file} [${o.label(c.stage)}]`;
  if (c.needs !== undefined) {
    const ask = LEG_NEEDS[c.needs];
    if (ask === undefined) return `${label}: \`needs\` is ${JSON.stringify(c.needs)}, which no leg is asked -- one of: ${Object.keys(LEG_NEEDS).join(", ")}`;
    if (!ask(leg)) {
      o.onRun?.(`${label} -- skipped: this leg has no ${c.needs}`);
      return "";
    }
  }
  if (!PROGRAM_KINDS.has(c.kind)) return "";
  const args = [...(c.args ?? []), c.file ?? ""];
  const r = await shared(`cd ${q(repo)} && ${q(binary)} run ${args.map(q).join(" ")}`, `wsl ${c.file}`);
  return judgeProgram(c, r, label);
}

/** Every target under valgrind, `jobs` at a time and longest first: every leaking fixture, as one line. */
async function valgrindSweep(o: LegsOptions, s: LegsSeams, where: Where, repo: string, binary: string, targets: string[]): Promise<string> {
  const say = (what: string) => o.onRun?.(what);
  const prof = s.profile(where, syncRunner(where), valgrindLine(repo, binary, targets[0]!));
  const width = s.jobs(where, targets.length);
  say(`valgrind sweep: ${targets.length} fixtures, ${width} at a time (${prof.cores ?? "?"} cores on the ${where} leg)`);

  // A fixture with no recorded cost goes first, because it may be the long one.
  const last = (prof.fixture_s ?? {}) as Record<string, number>;
  const order = [...targets].sort((a, b) => (last[b] ?? Infinity) - (last[a] ?? Infinity));
  const done = new Map<string, { r: Outcome; seconds: number }>();
  const began = s.now();
  let next = 0;
  const worker = async () => {
    while (next < order.length) {
      const f = order[next++]!;
      say(`valgrind ${f}`);
      const started = s.now();
      const r = await s.shell(where, valgrindLine(repo, binary, f), TIMEOUT_MS);
      done.set(f, { r, seconds: (s.now() - started) / 1000 });
      o.onStep?.();
    }
  };
  await Promise.all(Array.from({ length: Math.max(1, Math.min(width, order.length)) }, worker));
  const spent = (s.now() - began) / 1000;

  const fails: string[] = [];
  for (const f of targets) {
    const { r } = done.get(f)!;
    if (r.code === VALGRIND_ERROR) fails.push(`valgrind ${f}: exit ${r.code} -- ${firstErrLine(r)}`);
    else if (r.code === 127 && /valgrind: (command )?not found/.test(r.err)) fails.push(`valgrind ${f}: valgrind is not on the ${where} leg's PATH`);
    else if (r.code === -1) fails.push(`valgrind ${f}: did not finish -- ${firstErrLine(r)}`);
  }
  if (typeof prof.sample_s === "number" && spent > 0) {
    s.remember(where, { sweep_s: round1(spent), sweep_speedup: Math.round(((prof.sample_s * targets.length) / spent) * 100) / 100 });
  }
  s.remember(where, { fixture_s: Object.fromEntries([...done].map(([f, d]) => [f, round1(d.seconds)])) });
  if (fails.length > 0) return valgrindFailLine(fails);
  if (o.gateOpen) o.memo.remember(legSpec("valgrind sweep"), o.key("valgrind sweep"));
  return "";
}

function round1(n: number): number {
  return Math.round(n * 10) / 10;
}
