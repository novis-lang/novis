// `bun nv ci`: pushes `main` and watches GitHub's CI on it, for the session that drives CI and
// looper's `after_retire` hook.
//
//     bun nv ci --push                  # run the pre-push checks, then push main, which starts its CI
//                                       # and Website runs
//     bun nv ci --check                 # run the pre-push checks alone, and push nothing
//     bun nv ci --watch [--max-min <n>] [--past <job>]... [--next]
//                                       # wait for the first failed job, or the next green verdict
//     bun nv ci --status                # the newest run of each watched workflow, and each failed job
//     bun nv ci --rerun                 # run the failed jobs of each red newest run again
//     bun nv ci --dispatch full|dry-run # start a full ci.yml run, or a dry-run Release, on main
//     bun nv ci --watch --release       # watch the newest Release run, and cancel no run
//     bun nv ci --release-status        # the verdict of the newest completed dry-run Release
//
// **`--push`** pushes the local `main` to `origin`, and does nothing when `origin` has it already.
// Before it pushes, it runs `PRE_PUSH`: each check a CI job runs that judges files rather than running
// the suites, on the tree as it stands. Together they take about a minute once `cargo build` is warm,
// and they cover about half of the failures CI finds, which CI reports an hour or two after the push. A red
// check refuses the push: the command exits 1, prints each red check and its command, and writes them to
// `REFUSED`, which `--watch` and `--status` report as red until a push goes out. **`--check`** runs
// the same checks and pushes nothing, for a session before it claims a goal done. The push itself starts
// `ci.yml`'s push lane and the `Website` run, so the command starts no run. It then
// cancels every run of a workflow in `WORKFLOWS` still going on `main` for an older commit, so each
// workflow has one run going on `main` at a time (`rule:testing/one-run-on-main`). The push lane runs
// every job the diff since the last push reaches, and the nightly deep lane runs the rest
// (`rule:testing/ci-lanes`).
//
// **`--watch`** reads the newest run of each workflow in `WORKFLOWS` on `main`, passing over a cancelled
// one on an older commit than `origin/main`, which a newer push replaced. A cancelled run or job on
// `origin/main` itself is red: GitHub cancels a job that passes its `timeout-minutes`, and keeps no log
// of it, so the job's annotations are printed instead. It polls the jobs of each run in progress every
// `POLL_BUSY_MS`. Each poll cancels every run of
// a workflow still going beside a newer one, such as a push run the nightly deep lane started after. It exits 1 the moment a job
// fails, or at once when a newest run already ended red, and prints each failed job, its failed step,
// its URL and the end of its log. The whole log is kept under `.agent-tmp/ci/`. It exits 0 when every
// run it watched in progress ended green. With nothing in progress and nothing red it waits, polling
// every `POLL_IDLE_MS`, for the next run to start. It exits 3 after `--max-min` minutes with nothing to
// report, so a caller under a time limit runs it again. A `gh` that keeps failing for `GH_GIVE_UP` polls
// in a row exits 2. The exit codes are the verdict, and every line it prints is for the reader.
//
// **`--past <job>`**, given once per job, passes over a failure that somebody else's open work owns, so
// the watch goes on for every other job. It names a job, or the jobs under it: `--past proofs` covers
// `proofs / linux-x86_64`, and `--past "miri units"` covers each shard, such as `miri units (nvs-stdlib
// 1/12)`. A passed-over job is printed once and its log is not read. A run whose every failed job is
// passed over counts as ended, so an idle watch waits for the next run, and a watch that saw such a run
// end exits 4 with no other red. A session that drives CI passes the red a main loop's stage still
// owns this way, which `sleep` cannot.
//
// **`--next`** passes over every run that had already ended when the watch started, so the watch waits
// for the next push's runs. A session whose fixes for a red run are committed and not yet pushed
// watches this way, since that red run would otherwise end the watch at once.
//
// **`--status`** prints what `--watch` would read, once, and exits 1 when a newest run is red.
// **`--rerun`** reruns the failed jobs of each red newest run, for a failure that a fix cannot reach,
// such as a runner that went away.
//
// **`--dispatch full`** starts `ci.yml` on `main` with `workflow_dispatch`, which runs every job.
// **`--dispatch dry-run`** starts `release.yml` on `main` as a dry run, which writes nothing, tags
// nothing and pushes no image. Each argv is one constant in `DISPATCH`, and the release's carries
// `dry_run=true`, so no argument can start a real release: that is a person's (`docs/release.md`).
// Both refuse with 1 while local `main` has commits `origin/main` lacks that change a file outside
// `NOTES_ONLY`, since the run would test `origin`'s tip and not the tree. Commits to the loop's notes
// alone change nothing a run builds, and `--release-status` passes over them in the same way, so a
// dry run dispatched beside a handoff commit still lands on the commit a full run proved, and skips
// its `verify` job. Both then wait until GitHub lists a run newer than every one before
// the dispatch, and print its address, so a `--watch` started after reads the new run. Five minutes
// with no new run exits 2.
//
// **`--watch --release`** watches the newest `Release` run on `main` in place of `CI` and `Website`,
// with the same polling, report and exit codes. Neither it nor `--push` ever cancels a `Release` run: a
// cancelled `publish` is the torn state `release.yml`'s `concurrency` comment exists to prevent.
//
// **`--release-status`** judges the newest completed run of `release.yml` on `main` that its `run-name`
// titles `Release (dry run)`, prints each job, and exits 0 green, 1 red and 2 when `gh` or `git` cannot
// answer. `judgeDryRun` is the judgement: the run's commit is local `main`, or one that only
// `NOTES_ONLY` paths changed since; every job passed, except a `publish` a dry run skips, a `verify`
// skipped beside a green full `ci.yml` run of the commit (`greenFullRun`, the lookup `plan` makes),
// and a failed leg `build.yml` marks optional, which is a warning. It only reads: it never fetches,
// pushes, dispatches or cancels.
//
// What it spends: one `gh api` call per watched run every 15 seconds while a run is in progress, about
// 500 calls an hour of GitHub's 5,000, and up to `LOGS_KEPT` job logs on disk.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";
import { LOCKFILE_DIRS } from "./ci-changes.ts";
import { greenFullRun, type WorkflowRun } from "./release.ts";

export const summary =
  "pushes main and watches GitHub CI: nv ci --push | --check | --watch [--release] [--max-min <n>] [--past <job>] [--next] | --status | --rerun | --dispatch full|dry-run | --release-status";

const USAGE =
  "usage: bun nv ci --push\n       bun nv ci --check\n       bun nv ci --watch [--release] [--max-min <n>] [--past <job>]... [--next]\n       bun nv ci --status\n       bun nv ci --rerun\n       bun nv ci --dispatch full|dry-run\n       bun nv ci --release-status";

/** The workflows whose runs on `main` are watched, by their `name:`. */
export const WORKFLOWS = ["CI", "Website"];
/** `release.yml`'s `name:`, the one workflow `--watch --release` watches. No command cancels its runs. */
export const RELEASE = "Release";
/** The title `release.yml`'s `run-name` gives a dry run. */
export const DRY_RUN_TITLE = "Release (dry run)";
/**
 * Each watched workflow's `name:`, by its file. The runs API gives a run's `run-name` title as its
 * `name`, so a dry run's `name` is `Release (dry run)`, and `workflowOf` reads the workflow from `path`.
 */
const WORKFLOW_FILES: Record<string, string> = { "ci.yml": "CI", "pages.yml": "Website", "release.yml": RELEASE };
const BRANCH = "main";

/**
 * What `--dispatch` starts: each argv is a constant, so no argument can change it. A `workflow_dispatch`
 * of `ci.yml` runs every job, and the release's argv always carries `dry_run=true`, so no agent can start
 * a real release.
 */
export const DISPATCH = {
  full: { workflow: "CI", file: "ci.yml", argv: ["workflow", "run", "ci.yml", "--ref", BRANCH] },
  "dry-run": {
    workflow: RELEASE,
    file: "release.yml",
    argv: ["workflow", "run", "release.yml", "--ref", BRANCH, "-f", "bump=patch", "-f", "dry_run=true"],
  },
} as const;
type DispatchKind = keyof typeof DISPATCH;
/** How long `--dispatch` waits for GitHub to list the run it started. */
const DISPATCH_WAIT_MS = 5 * 60_000;

/** The job `release.yml`'s `publish` is, which a dry run skips, and the one its `test` is. */
export const PUBLISH_JOB = "tag, changelog and draft release";
export const VERIFY_JOB = "verify";
/** The paths a commit after a dry run may change and leave its verdict standing: the loop's own notes. */
export const NOTES_ONLY = ["docs/goals/", "docs/agent/playbook/", "docs/agent/playbook.md"];

/** How often a run in progress is polled, and how often an idle watch looks for a new run. */
export const POLL_BUSY_MS = 15_000;
export const POLL_IDLE_MS = 60_000;

/** How many failed polls in a row end the watch. */
const GH_GIVE_UP = 20;
/** The default `--max-min`: under a two-hour background limit. */
const MAX_MIN = 110;
/** How many lines of a failed job's log are printed, and how many logs `.agent-tmp/ci/` keeps. */
const TAIL_LINES = 40;
const LOGS_KEPT = 20;
/** How many earlier lines that name a failure are printed above the tail, and how one is told. */
const MARKED_LINES = 20;
const FAILURE_LINE = /^(FAIL\b|FAILED\b|error(\[E\d+\])?:|test .* \.\.\. FAILED|thread '.*' panicked|failures:|---- )|##\[error\]/;
/** A job conclusion that is a failure. A job that passes its `timeout-minutes` ends `cancelled`. */
const RED = new Set(["failure", "timed_out", "startup_failure", "cancelled"]);

const EXE = process.platform === "win32" ? "nvs.exe" : "nvs";
/** The binary the checks after `cargo build` read, which that build has just made current. */
const DEBUG_NVS = `target/debug/${EXE}`;

/** A check `--push` runs first: its name, its command, and the variables it adds. */
export interface PrePush {
  name: string;
  argv: string[];
  env?: Record<string, string>;
}

/** The checks `--push` runs before it pushes, cheapest first. Each is the step of a CI job (`ci.yml`'s
 * `lint`, `attribution`, `supply-chain`, `docs`, `reference` and `proofs` jobs) with the binary
 * named, so none reads a release binary built before the goal's own changes. */
export const PRE_PUSH: PrePush[] = [
  { name: "rustfmt", argv: ["cargo", "fmt", "--all", "--check"] },
  { name: "lint tables", argv: ["bun", "nv", "lints", "--check"] },
  { name: "rules", argv: ["bun", "nv", "rules", "--check"] },
  { name: "rulebook pages", argv: ["bun", "nv", "rules", "--render", "--check"] },
  { name: "rendered files", argv: ["bun", "nv", "render", "--check"] },
  { name: "third-party attribution", argv: ["bun", "nv", "gen-attribution", "--check"] },
  { name: "advisories", argv: ["cargo", "deny", "check", "advisories"] },
  ...LOCKFILE_DIRS.map((d): PrePush => ({
    name: `advisories (${d})`,
    argv: ["cargo", "deny", "--all-features", "--manifest-path", `${d}/Cargo.toml`, "--config", "deny.toml", "check", "advisories"],
  })),
  { name: "build", argv: ["cargo", "build"] },
  // `nv reference` runs each example in a folder of its own, so the binary's path is absolute.
  { name: "chapter examples", argv: ["bun", "nv", "reference"], env: { NVS_BIN: join(ROOT, DEBUG_NVS) } },
  { name: "feature proofs", argv: ["bun", "nv", "proofs", "--gate", "--limit", "0", "--nvs", DEBUG_NVS] },
  { name: "website render", argv: ["bun", "nv", "render", "--website", "--check"], env: { NVS_BIN: DEBUG_NVS } },
  { name: "website stale guard", argv: ["bun", "nv", "site", "--check", "structure", "snippets", "stale", "questions", "syntax", "guides", "tools", "in-depth", "roadmap", "prose", "--nvs", DEBUG_NVS] },
];

/** Where a refused push leaves its red checks, under the root. */
export const REFUSED = ".agent-tmp/ci/refused-push.txt";
/** How many lines of a red check's output are printed and kept, and how many of them from its start. */
const CHECK_LINES = 30;
const CHECK_HEAD = 5;

/** What a program printed, and how it exited. */
export interface Ran {
  code: number;
  stdout: string;
  stderr: string;
}

/** Everything the command reads, starts or waits on, replaced under test. */
export interface CiSeams {
  root: string;
  gh(args: string[]): Promise<Ran>;
  git(args: string[]): Promise<Ran>;
  /** Runs one `PRE_PUSH` check in the root. */
  check(c: PrePush): Promise<Ran>;
  sleep(ms: number): Promise<void>;
  now(): number;
  say(line: string): void;
}

const REAL: CiSeams = {
  root: ROOT,
  gh: async (args) => {
    const r = await proc(["gh", ...args], { timeoutMs: 120_000 });
    return { code: r.code, stdout: r.stdout, stderr: r.stderr };
  },
  git: async (args) => {
    const r = await proc(["git", ...args], { cwd: ROOT, timeoutMs: 300_000 });
    return { code: r.code, stdout: r.stdout, stderr: r.stderr };
  },
  // A cold `cargo build` is the slowest of them.
  check: async (c) => {
    const r = await proc(c.argv, { cwd: ROOT, timeoutMs: 1_800_000, env: c.env ?? {} });
    return { code: r.code, stdout: r.stdout, stderr: r.timedOut ? `${r.stderr}\ntimed out` : r.stderr };
  },
  sleep: (ms) => Bun.sleep(ms),
  now: () => Date.now(),
  say: (line) => console.log(line),
};

/** A workflow run on `main`, as the runs API gives it. */
export interface Run {
  id: number;
  name: string;
  event: string;
  status: string;
  conclusion: string | null;
  head_sha: string;
  html_url: string;
  /** The run's title: its `run-name`, such as `Release (dry run)`, or the workflow's name. */
  display_title: string;
  /** The workflow's file, such as `.github/workflows/release.yml`. */
  path?: string;
}

/** `r` with `name` set to its workflow's `name:`, which GitHub replaces with the run's `run-name` title. */
function workflowOf<R extends Run>(r: R): R {
  const named = WORKFLOW_FILES[(r.path ?? "").split("/").pop() ?? ""];
  return named === undefined ? r : { ...r, name: named };
}

/** A job of a run, with its steps. */
export interface Job {
  id: number;
  name: string;
  status: string;
  conclusion: string | null;
  html_url: string;
  steps: { name: string; conclusion: string | null }[];
}

class GhFailed extends Error {}

function firstLine(r: Ran): string {
  return `${r.stderr}\n${r.stdout}`.split("\n").map((l) => l.trim()).find((l) => l !== "") ?? `exit ${r.code}`;
}

async function ghJson<T>(s: CiSeams, args: string[]): Promise<T> {
  const r = await s.gh(args);
  if (r.code !== 0) throw new GhFailed(`\`gh ${args.slice(0, 2).join(" ")}\` exited ${r.code}: ${firstLine(r)}`);
  return JSON.parse(r.stdout) as T;
}

/** The newest runs on `main`, newest first, each named by its workflow. */
export async function listRuns(s: CiSeams): Promise<Run[]> {
  const runs = await ghJson<Run[]>(s, [
    "api",
    `repos/{owner}/{repo}/actions/runs?branch=${BRANCH}&per_page=40`,
    "--jq",
    "[.workflow_runs[] | {id, name, event, status, conclusion, head_sha, html_url, display_title, path}]",
  ]);
  return runs.map(workflowOf);
}

/** The jobs of run `id`, their latest attempt. */
export function listJobs(s: CiSeams, id: number): Promise<Job[]> {
  return ghJson<Job[]>(s, [
    "api",
    `repos/{owner}/{repo}/actions/runs/${id}/jobs?per_page=100&filter=latest`,
    "--jq",
    "[.jobs[] | {id, name, status, conclusion, html_url, steps: [(.steps // [])[] | {name, conclusion}]}]",
  ]);
}

/**
 * The newest run of each watched workflow, in `WORKFLOWS` order, passing over each cancelled run on a
 * commit other than `tip`, `origin/main`'s. A newer push cancels those, and its own runs may not be
 * listed yet. A cancelled run on `tip` stopped for another reason, such as a job's time limit.
 */
export function newest(runs: Run[], tip: string, workflows: readonly string[] = WORKFLOWS): Run[] {
  const out: Run[] = [];
  for (const w of workflows) {
    const kept = (x: Run) => x.conclusion !== "cancelled" || x.head_sha === tip;
    const r = runs.filter((x) => x.name === w && kept(x)).sort((a, b) => b.id - a.id)[0];
    if (r !== undefined) out.push(r);
  }
  return out;
}

/** `origin/main`'s commit as the last fetch or push left it, or "" when git cannot say. */
async function tipOf(s: CiSeams): Promise<string> {
  const r = await s.git(["rev-parse", `origin/${BRANCH}`]);
  return r.code === 0 ? r.stdout.trim() : "";
}

const done = (r: { status: string }) => r.status === "completed";
const red = (r: { conclusion: string | null }) => r.conclusion !== null && RED.has(r.conclusion);
const short = (sha: string) => sha.slice(0, 9);

/** One line naming a run. */
function runLine(r: Run): string {
  const state = done(r) ? r.conclusion : r.status;
  return `${r.name} (${r.event}) on ${short(r.head_sha)}: ${state}  ${r.html_url}`;
}

/**
 * The runs of each watched workflow still going that a newer run replaces: every one but the newest,
 * and with `head` given, every one on another commit too, since its run may not be listed yet. It never
 * names a `Release` run: a cancelled `publish` is the torn state `release.yml`'s `concurrency` exists to
 * prevent.
 */
export function superseded(runs: Run[], head?: string, workflows: readonly string[] = WORKFLOWS): Run[] {
  const out: Run[] = [];
  for (const w of workflows.filter((x) => x !== RELEASE)) {
    const going = runs.filter((x) => x.name === w && !done(x)).sort((a, b) => b.id - a.id);
    out.push(...going.filter((r, i) => i > 0 || (head !== undefined && r.head_sha !== head)));
  }
  return out;
}

/** Cancels each run `superseded` names that `asked` has not cancelled already. A failed cancel only warns. */
async function cancelSuperseded(s: CiSeams, runs: Run[], head: string | undefined, asked: Set<number>, workflows: readonly string[] = WORKFLOWS): Promise<void> {
  for (const r of superseded(runs, head, workflows).filter((x) => !asked.has(x.id))) {
    asked.add(r.id);
    const c = await s.gh(["run", "cancel", String(r.id)]);
    s.say(c.code === 0 ? `cancelled ${runLine(r)}: a newer run replaces it` : `could not cancel ${r.id}: ${firstLine(c)}`);
  }
}

/** Prints each failed job of `jobs`: its failed step, its URL and the end of its log. */
async function report(s: CiSeams, run: Run, jobs: Job[]): Promise<void> {
  for (const j of jobs.filter(red)) {
    const step = j.steps.find(red)?.name ?? "(no step named)";
    s.say(`FAILED ${run.name} / ${j.name} / step "${step}"  ${j.html_url}`);
    if (j.conclusion === "cancelled") {
      // A job a time limit stopped has no failed step and often no log. Its annotations say why it stopped.
      const notes = await s.gh(["api", `repos/{owner}/{repo}/check-runs/${j.id}/annotations`, "--jq", ".[].message"]);
      for (const n of notes.stdout.split(/\r?\n/).filter((l) => l.trim() !== "")) s.say(`  annotation: ${n}`);
    }
    // A log holds the colour codes the job printed, which `gh` passes on only when asked to.
    const log = await s.gh(["api", "--allow-escape-sequences", `repos/{owner}/{repo}/actions/jobs/${j.id}/logs`]);
    if (log.code !== 0) {
      s.say(`  the log could not be read: ${firstLine(log)}`);
      continue;
    }
    const lines = logLines(log.stdout);
    const path = keepLog(s.root, j.id, lines.join("\n"));
    s.say(`  the whole log: ${path}`);
    for (const l of tail(lines)) s.say(`  | ${l}`);
  }
}

/** A job log's lines without their timestamps and colour codes. */
export function logLines(text: string): string[] {
  return text
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((l) => l.replace(/^\d{4}-\d\d-\d\dT[\d:.]+Z ?/, "").replace(/\x1b\[[0-9;]*[A-Za-z]/g, ""));
}

/**
 * The lines worth printing of a failed job's log: up to `MARKED_LINES` lines before the tail that name a
 * failure, such as a test's `FAIL` line, then the last `TAIL_LINES` lines up to the log's last `##[error]`
 * line, or of the whole log when it has none. A suite prints its failure long before its summary.
 */
export function tail(lines: string[]): string[] {
  const end = lines.findLastIndex((l) => /##\[error\]/.test(l));
  const upTo = end === -1 ? lines.length : end + 1;
  const from = Math.max(0, upTo - TAIL_LINES);
  const marked = lines.slice(0, from).filter((l) => FAILURE_LINE.test(l)).slice(-MARKED_LINES);
  const last = lines.slice(from, upTo).filter((l) => l.trim() !== "");
  return marked.length === 0 ? last : [...marked, "...", ...last];
}

/** Writes a job's log under `.agent-tmp/ci/`, keeps the newest `LOGS_KEPT`, and returns its path. */
function keepLog(root: string, id: number, text: string): string {
  const dir = join(root, ".agent-tmp", "ci");
  mkdirSync(dir, { recursive: true });
  const path = join(dir, `job-${id}.log`);
  writeFileSync(path, text);
  const old = readdirSync(dir)
    .filter((f) => f.endsWith(".log"))
    .map((f) => ({ f, t: statSync(join(dir, f)).mtimeMs }))
    .sort((a, b) => b.t - a.t)
    .slice(LOGS_KEPT);
  for (const o of old) rmSync(join(dir, o.f), { force: true });
  return path.slice(root.length + 1).replaceAll("\\", "/");
}

// ---- --watch -----------------------------------------------------------------------------------------

/** Prints the red checks of a refused push, if one is waiting, and reports whether one was. */
function sayRefused(s: CiSeams): boolean {
  const path = join(s.root, REFUSED);
  if (!existsSync(path)) return false;
  s.say(readFileSync(path, "utf8").trimEnd());
  return true;
}

/**
 * Whether job `name` is `p` or a job under it: a reusable workflow's job, such as `proofs / linux-x86_64`
 * under `proofs`, or a matrix shard, such as `miri units (nvs-stdlib 1/12)` under `miri units`.
 */
const under = (name: string, p: string) => name === p || name.startsWith(`${p} / `) || name.startsWith(`${p} (`);

/**
 * Watches until a job fails (1), every run it saw in progress is green (0), or `maxMin` passes (3). A
 * failed job that `past` names is passed over, and a watched run that ended red in those jobs alone
 * ends the watch with 4. With `next`, every run that had ended at the first poll is passed over.
 * `workflows` are the workflows watched: `WORKFLOWS`, or `[RELEASE]` under `--release`, whose runs it
 * never cancels.
 */
export async function watch(s: CiSeams, maxMin: number, past: string[] = [], next = false, workflows: readonly string[] = WORKFLOWS): Promise<number> {
  if (sayRefused(s)) return 1;
  const until = s.now() + maxMin * 60_000;
  const watched = new Set<number>();
  const before = new Set<number>();
  const finished = new Map<number, number>();
  const passedOver = new Set<number>();
  const passed = (j: Job) => past.some((p) => under(j.name, p));
  /** Prints each failed job of `jobs` that `past` names, once, and reports whether any other failed. */
  const sort = (r: Run, jobs: Job[]): boolean => {
    for (const j of jobs.filter((x) => red(x) && passed(x) && !passedOver.has(x.id))) {
      passedOver.add(j.id);
      s.say(`  passing over FAILED ${r.name} / ${j.name}  ${j.html_url}`);
    }
    return jobs.some((j) => red(j) && !passed(j));
  };
  const cancelled = new Set<number>();
  let endedRed = false;
  let failures = 0;
  let first = true;
  while (true) {
    let busy = false;
    try {
      const all = await listRuns(s);
      await cancelSuperseded(s, all, undefined, cancelled, workflows);
      const runs = newest(all, await tipOf(s), workflows);
      failures = 0;
      for (const r of runs) {
        if (first && next && done(r)) {
          before.add(r.id);
          s.say(`passing over ${runLine(r)}`);
        }
        if (before.has(r.id)) continue;
        if (done(r) && red(r)) {
          const jobs = await listJobs(s, r.id);
          if (first || watched.has(r.id)) s.say(runLine(r));
          if (sort(r, jobs) || !jobs.some(red)) {
            await report(s, r, jobs.filter((j) => !passed(j)));
            return 1;
          }
          if (watched.has(r.id)) endedRed = true;
          continue;
        }
        if (first && done(r)) s.say(runLine(r));
        if (done(r)) continue;
        busy = true;
        const jobs = await listJobs(s, r.id);
        if (!watched.has(r.id)) {
          watched.add(r.id);
          s.say(`watching ${runLine(r)}`);
        }
        if (sort(r, jobs)) {
          await report(s, r, jobs.filter((j) => !passed(j)));
          return 1;
        }
        const n = jobs.filter(done).length;
        if (finished.get(r.id) !== n) {
          finished.set(r.id, n);
          s.say(`  ${r.name}: ${n} of ${jobs.length} jobs finished green`);
        }
      }
      first = false;
      if (!busy && watched.size > 0) {
        if (endedRed) {
          s.say("every watched run ended, and only the jobs `--past` names failed");
          return 4;
        }
        for (const r of runs) s.say(`GREEN ${runLine(r)}`);
        return 0;
      }
    } catch (e) {
      if (!(e instanceof GhFailed) && !(e instanceof SyntaxError)) throw e;
      failures++;
      s.say(`WATCH ERROR (${failures} of ${GH_GIVE_UP}): ${e.message}`);
      if (failures >= GH_GIVE_UP) return 2;
    }
    if (s.now() >= until) {
      s.say(`nothing failed in ${maxMin} minutes: run \`bun nv ci --watch\` again`);
      return 3;
    }
    await s.sleep(busy ? POLL_BUSY_MS : POLL_IDLE_MS);
  }
}

// ---- --push ------------------------------------------------------------------------------------------

async function gitOut(s: CiSeams, args: string[]): Promise<string> {
  const r = await s.git(args);
  if (r.code !== 0) throw new GhFailed(`\`git ${args.join(" ")}\` exited ${r.code}: ${firstLine(r)}`);
  return r.stdout.trim();
}

/** What a red check printed, without `bun`'s script lines: all of it up to `CHECK_LINES`, otherwise its
 * first `CHECK_HEAD` lines, where a verdict's summary is, and its end. */
export function checkLines(out: string): string[] {
  const lines = out.split(/\r?\n/).filter((l) => l.trim() !== "" && !/^\$ bun |^error: script "nv" exited/.test(l));
  if (lines.length <= CHECK_LINES) return lines;
  return [...lines.slice(0, CHECK_HEAD), "...", ...lines.slice(-(CHECK_LINES - CHECK_HEAD))];
}

/** Runs every `PRE_PUSH` check, and returns what each red one printed, empty when all are green. A
 * `cargo deny` that is not installed is passed over with a note, since CI's `supply-chain` job runs it. */
export async function prePush(s: CiSeams): Promise<string[]> {
  const reds: string[] = [];
  for (const c of PRE_PUSH) {
    const r = await s.check(c);
    const out = `${r.stdout}\n${r.stderr}`;
    if (r.code === 0) s.say(`pre-push: ${c.name}: ok`);
    else if (c.argv[1] === "deny" && /no such (sub)?command/.test(out)) s.say(`pre-push: ${c.name}: cargo-deny is not installed, so CI's supply-chain job is the only check`);
    else {
      const lines = checkLines(out);
      const env = Object.entries(c.env ?? {}).map(([k, v]) => `${k}=${v} `).join("");
      reds.push([`${c.name}: red (\`${env}${c.argv.join(" ")}\` exited ${r.code})`, ...lines.map((l) => `  ${l}`)].join("\n"));
      s.say(`pre-push: ${c.name}: red`);
    }
  }
  return reds;
}

/**
 * Pushes `main` when `origin` does not have it yet and every `PRE_PUSH` check is green, then cancels
 * each run still going on an older commit. The push starts its own runs.
 */
export async function push(s: CiSeams): Promise<number> {
  await gitOut(s, ["fetch", "-q", "origin", BRANCH]);
  const head = await gitOut(s, ["rev-parse", BRANCH]);
  const remote = await gitOut(s, ["rev-parse", `origin/${BRANCH}`]);
  const refused = join(s.root, REFUSED);
  if (head === remote) s.say(`origin/${BRANCH} is at ${short(head)} already: nothing to push`);
  else {
    const reds = await prePush(s);
    if (reds.length > 0) {
      const text = `the push of ${BRANCH} at ${short(head)} was refused: ${reds.length} pre-push check(s) are red. Fix each, commit, and run \`bun nv ci --push\` again.\n\n${reds.join("\n\n")}\n`;
      mkdirSync(join(s.root, ".agent-tmp", "ci"), { recursive: true });
      writeFileSync(refused, text);
      s.say(text.trimEnd());
      return 1;
    }
    const pushed = await s.git(["push", "origin", `${BRANCH}:${BRANCH}`]);
    if (pushed.code !== 0) {
      s.say(`the push failed: ${firstLine(pushed)}`);
      return 1;
    }
    s.say(`pushed ${BRANCH} at ${short(head)}: the push starts its CI and Website runs`);
  }
  rmSync(refused, { force: true });
  // The push succeeded whatever GitHub says next, so a failed listing only warns.
  try {
    await cancelSuperseded(s, await listRuns(s), head, new Set());
  } catch (e) {
    if (!(e instanceof GhFailed) && !(e instanceof SyntaxError)) throw e;
    s.say(`the older runs could not be listed: ${e.message}`);
  }
  return 0;
}

// ---- --status and --rerun ----------------------------------------------------------------------------

async function status(s: CiSeams): Promise<number> {
  let bad = sayRefused(s) ? 1 : 0;
  const runs = newest(await listRuns(s), await tipOf(s));
  for (const r of runs) {
    s.say(runLine(r));
    const jobs = await listJobs(s, r.id);
    for (const j of jobs.filter(red)) {
      bad++;
      s.say(`  FAILED ${j.name} / step "${j.steps.find(red)?.name ?? "(no step named)"}"  ${j.html_url}`);
    }
    if (done(r) && red(r) && !jobs.some(red)) bad++;
  }
  return bad > 0 ? 1 : 0;
}

async function rerun(s: CiSeams): Promise<number> {
  const reds = newest(await listRuns(s), await tipOf(s)).filter((r) => done(r) && red(r));
  if (reds.length === 0) s.say("no newest run is red: nothing to rerun");
  let code = 0;
  for (const r of reds) {
    const c = await s.gh(["run", "rerun", String(r.id), "--failed"]);
    s.say(c.code === 0 ? `rerunning the failed jobs of ${runLine(r)}` : `could not rerun ${r.id}: ${firstLine(c)}`);
    if (c.code !== 0) code = 1;
  }
  return code;
}

// ---- --dispatch --------------------------------------------------------------------------------------

/** The newest runs of one workflow file on `main`, newest first, with their branch and title. */
export async function workflowRuns(s: CiSeams, file: string, query = ""): Promise<(Run & WorkflowRun)[]> {
  const runs = await ghJson<(Run & WorkflowRun)[]>(s, [
    "api",
    `repos/{owner}/{repo}/actions/workflows/${file}/runs?branch=${BRANCH}&per_page=30${query}`,
    "--jq",
    "[.workflow_runs[] | {id, name, event, status, conclusion, head_sha, head_branch, html_url, display_title, path}]",
  ]);
  return runs.map(workflowOf);
}

/** The runs `--dispatch` may have started: a `workflow_dispatch` one, and of a release, a dry run. */
function dispatched(kind: DispatchKind, r: Run): boolean {
  return r.event === "workflow_dispatch" && (kind !== "dry-run" || r.display_title === DRY_RUN_TITLE);
}

/**
 * Starts the run `DISPATCH[kind]` names on `main`, then waits until GitHub lists a run of it newer than
 * every one before, and prints its address. It refuses while local `main` has commits `origin/main`
 * lacks that change a file outside `NOTES_ONLY`, since the run would test `origin`'s tip and not the
 * tree. Five minutes with no new run exits 2.
 */
export async function dispatch(s: CiSeams, kind: DispatchKind): Promise<number> {
  const d = DISPATCH[kind];
  await gitOut(s, ["fetch", "-q", "origin", BRANCH]);
  const ahead = Number(await gitOut(s, ["rev-list", "--count", `origin/${BRANCH}..${BRANCH}`]));
  if (ahead > 0) {
    const changed = (await gitOut(s, ["diff", "--name-only", `origin/${BRANCH}`, BRANCH])).split(/\r?\n/).filter((l) => l !== "");
    const code = changed.filter((p) => !NOTES_ONLY.some((n) => p.startsWith(n)));
    if (code.length > 0) {
      s.say(`local ${BRANCH} has ${ahead} commit(s) origin/${BRANCH} lacks, which change ${code.slice(0, 5).join(", ")}: run \`bun nv ci --push\` first, so the run tests the tree`);
      return 1;
    }
    s.say(`local ${BRANCH} has ${ahead} commit(s) origin/${BRANCH} lacks, which change only the loop's notes: the run tests origin/${BRANCH}'s tip`);
  }
  const before = Math.max(0, ...(await workflowRuns(s, d.file)).map((r) => r.id));
  const started = await s.gh([...d.argv]);
  if (started.code !== 0) {
    s.say(`could not dispatch ${d.file}: ${firstLine(started)}`);
    return 1;
  }
  s.say(`dispatched ${d.file} on ${BRANCH}${kind === "dry-run" ? " as a dry run" : ""}: waiting for GitHub to list the run`);
  const until = s.now() + DISPATCH_WAIT_MS;
  while (true) {
    await s.sleep(POLL_BUSY_MS);
    try {
      const fresh = (await workflowRuns(s, d.file)).filter((r) => r.id > before && dispatched(kind, r)).sort((a, b) => a.id - b.id)[0];
      if (fresh !== undefined) {
        s.say(`started ${runLine(fresh)}`);
        s.say(`watch it with \`bun nv ci --watch${kind === "dry-run" ? " --release" : ""}\``);
        return 0;
      }
    } catch (e) {
      if (!(e instanceof GhFailed) && !(e instanceof SyntaxError)) throw e;
      s.say(`the runs could not be listed: ${e.message}`);
    }
    if (s.now() >= until) {
      s.say(`GitHub listed no new ${d.workflow} run within ${DISPATCH_WAIT_MS / 60_000} minutes of the dispatch`);
      return 2;
    }
  }
}

// ---- --release-status --------------------------------------------------------------------------------

/** What `judgeDryRun` reads: the dry run, its jobs, the `ci.yml` runs of its commit, and what changed since. */
export interface DryRunFacts {
  run: Run;
  jobs: Job[];
  /** The `ci.yml` runs of the dry run's commit, for a `verify` job it skipped. */
  ciRuns: WorkflowRun[];
  /** The paths changed between the dry run's commit and local `main`, or null when `main` does not contain it. */
  changed: string[] | null;
  /** The job names of `build.yml`'s legs marked `optional: true`, as the release shows them. */
  optional: string[];
}

/** The verdict of `--release-status`: green, the lines it prints, and the warnings among them. */
export interface DryRunVerdict {
  green: boolean;
  lines: string[];
}

/**
 * Judges one completed dry run as `release.yml`'s own `if:` lines do. Its commit is local `main`, or one
 * whose changes since touch only `NOTES_ONLY`. Every job passed, except that `publish` is skipped, as
 * every dry run's is; `verify` may be skipped only beside a green full `ci.yml` run of that commit, the
 * lookup `plan` makes; and a failed optional build leg is a warning, since the release ships without it.
 */
export function judgeDryRun(f: DryRunFacts): DryRunVerdict {
  const lines = [`dry run ${runLine(f.run)}`];
  let green = true;
  const fail = (l: string) => {
    green = false;
    lines.push(l);
  };
  if (f.changed === null) fail(`RED its commit ${short(f.run.head_sha)} is not on local ${BRANCH}: dispatch a new dry run`);
  else {
    const code = f.changed.filter((p) => !NOTES_ONLY.some((n) => p.startsWith(n)));
    if (code.length > 0) fail(`RED ${code.length} file(s) outside the loop's notes changed since its commit, such as ${code.slice(0, 5).join(", ")}: dispatch a new dry run`);
  }
  if (f.jobs.length === 0) fail("RED the run lists no jobs");
  for (const j of f.jobs) {
    const step = j.steps.find(red)?.name;
    const where = `${j.name}${step === undefined ? "" : ` / step "${step}"`}  ${j.html_url}`;
    if (j.name.includes("${{")) fail(`RED ${j.name}: a job whose matrix did not expand  ${j.html_url}`);
    else if (j.name === PUBLISH_JOB) {
      if (j.conclusion === "skipped") lines.push(`  ok ${j.name}: skipped, as in every dry run`);
      else fail(`RED ${j.name} ended ${j.conclusion ?? j.status}, and a dry run skips it  ${j.html_url}`);
    } else if (j.name === VERIFY_JOB && j.conclusion === "skipped") {
      const full = greenFullRun(f.ciRuns, f.run.head_sha);
      if (full !== null) lines.push(`  ok ${j.name}: skipped beside the green full ci.yml run ${full.html_url}`);
      else fail(`RED ${j.name} was skipped, and no full ci.yml run of ${short(f.run.head_sha)} on ${BRANCH} is green`);
    } else if (j.conclusion === "success") lines.push(`  ok ${j.name}`);
    else if (f.optional.includes(j.name) && (j.conclusion === "failure" || j.conclusion === "timed_out")) {
      lines.push(`  WARNING ${where}: an optional leg, so the release ships without it`);
    } else fail(`FAILED ${where} (${j.conclusion ?? j.status})`);
  }
  lines.push(green ? `GREEN the dry run on ${short(f.run.head_sha)} passed` : "RED the dry run does not stand for main");
  return { green, lines };
}

/** The job names of `build.yml`'s optional legs, as a run of `release.yml` shows them. */
export function optionalLegs(root: string): string[] {
  type Leg = { name?: string; optional?: boolean };
  const wf = Bun.YAML.parse(readFileSync(join(root, ".github", "workflows", "build.yml"), "utf8")) as {
    jobs?: { build?: { strategy?: { matrix?: { include?: Leg[] } } } };
  };
  const legs = wf.jobs?.build?.strategy?.matrix?.include ?? [];
  return legs.filter((l) => l.optional === true && l.name !== undefined).map((l) => `build / build ${l.name}`);
}

/**
 * The verdict on the newest completed dry run of `release.yml` on `main`: 0 green, 1 red, 2 when `gh` or
 * `git` cannot answer. It reads only, and never fetches, pushes, dispatches or cancels.
 */
async function releaseStatus(s: CiSeams): Promise<number> {
  const dry = (await workflowRuns(s, "release.yml")).filter((r) => r.display_title === DRY_RUN_TITLE).sort((a, b) => b.id - a.id);
  for (const r of dry.filter((x) => !done(x))) s.say(`going, and not judged until it ends: ${runLine(r)}`);
  const run = dry.find(done);
  if (run === undefined) {
    s.say(`RED no completed run of release.yml on ${BRANCH} is titled \`${DRY_RUN_TITLE}\`: start one with \`bun nv ci --dispatch dry-run\``);
    return 1;
  }
  const main = await gitOut(s, ["rev-parse", BRANCH]);
  let changed: string[] | null = [];
  if (run.head_sha !== main) {
    const anc = await s.git(["merge-base", "--is-ancestor", run.head_sha, BRANCH]);
    if (anc.code !== 0) changed = null;
    else changed = (await gitOut(s, ["diff", "--name-only", run.head_sha, BRANCH])).split(/\r?\n/).filter((l) => l !== "");
  }
  const jobs = await listJobs(s, run.id);
  const skippedVerify = jobs.some((j) => j.name === VERIFY_JOB && j.conclusion === "skipped");
  const ciRuns = skippedVerify ? await workflowRuns(s, "ci.yml", `&head_sha=${run.head_sha}`) : [];
  const v = judgeDryRun({ run, jobs, ciRuns, changed, optional: optionalLegs(ROOT) });
  for (const l of v.lines) s.say(l);
  return v.green ? 0 : 1;
}

// ---- the command -------------------------------------------------------------------------------------

/** Parses and runs one mode; returns the exit code. */
export async function ci(args: string[], over: Partial<CiSeams> = {}): Promise<number> {
  const s: CiSeams = { ...REAL, ...over };
  const mode = args[0];
  let maxMin = MAX_MIN;
  const past: string[] = [];
  let next = false;
  let release = false;
  let kind: DispatchKind | undefined;
  let bad = ["--push", "--check", "--watch", "--status", "--rerun", "--dispatch", "--release-status"].includes(mode ?? "") ? "" : "name one mode";
  if (bad === "" && mode === "--dispatch") {
    const k = args[1];
    if (k === "full" || k === "dry-run") kind = k;
    else bad = "--dispatch takes `full` or `dry-run`";
  }
  for (let i = kind === undefined ? 1 : 2; i < args.length && bad === ""; i++) {
    const v = args[i + 1];
    if (mode === "--watch" && args[i] === "--release") release = true;
    else if (mode === "--watch" && args[i] === "--max-min" && v !== undefined && /^\d+$/.test(v)) {
      maxMin = Number(v);
      i++;
    } else if (mode === "--watch" && args[i] === "--past" && v !== undefined && v !== "") {
      past.push(v);
      i++;
    } else if (mode === "--watch" && args[i] === "--next") next = true;
    else bad = `unexpected argument ${args[i]}`;
  }
  if (bad !== "") {
    console.error(`${USAGE}\nnv ci: error: ${bad}`);
    return 2;
  }
  try {
    if (mode === "--push") return await push(s);
    if (mode === "--check") {
      const reds = await prePush(s);
      for (const r of reds) s.say(r);
      return reds.length > 0 ? 1 : 0;
    }
    if (mode === "--watch") return await watch(s, maxMin, past, next, release ? [RELEASE] : WORKFLOWS);
    if (mode === "--status") return await status(s);
    if (mode === "--dispatch" && kind !== undefined) return await dispatch(s, kind);
    if (mode === "--release-status") return await releaseStatus(s);
    return await rerun(s);
  } catch (e) {
    if (!(e instanceof GhFailed) && !(e instanceof SyntaxError)) throw e;
    s.say(`nv ci: ${e.message}`);
    return 2;
  }
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return ci(args);
}
