// `bun nv ci`: pushes `main` and watches GitHub's CI on it, for the session that drives CI and
// looper's `after_retire` hook.
//
//     bun nv ci --push                  # run the pre-push checks, then push main, which starts its CI,
//                                       # Website and Dev build runs
//     bun nv ci --check                 # run the pre-push checks alone, and push nothing
//     bun nv ci --watch [--max-min <n>] [--past <job>]... [--next]
//                                       # wait for the first failed job of any workflow, or the next
//                                       # green verdict
//     bun nv ci --status                # the newest run of each workflow and trigger, and each failed job
//     bun nv ci --rerun                 # run the failed jobs of each red newest run again
//     bun nv ci --dispatch full|dry-run # start a full ci.yml run, or a dry-run Release, on main
//     bun nv ci --dispatch release      # start the real Release, after a green dry run of main's tip
//     bun nv ci --watch --release       # watch the newest Release run, and cancel no run
//     bun nv ci --release-status        # the verdict of the newest completed Release, dry or real
//     bun nv ci --approve-release       # approve the waiting real run's `release` environment
//     bun nv ci --publish-release       # publish the draft a green real run made
//     bun nv ci --release-published     # the verdict of the newest release, its images and its tag
//
// **`--push`** pushes the local `main` to `origin`, and does nothing when `origin` has it already.
// Before it pushes, it runs `PRE_PUSH`: each check a CI job runs that judges files rather than running
// the suites, on the tree as it stands. Together they take about two minutes once `cargo build` is warm,
// and they cover about half of the failures CI finds, which CI reports an hour or two after the push. A red
// check refuses the push: the command exits 1, prints each red check and its command, and writes them to
// `REFUSED`, which `--watch` and `--status` report as red until a push goes out. **`--check`** runs
// the same checks and pushes nothing, for a session before it claims a goal done. The push itself starts
// `ci.yml`'s push lane, the `Website` run and the `Dev build` run, so the command starts no run. It then
// cancels every run of a workflow in `ONE_RUN_ON_MAIN` still going on `main` for an older commit, so each
// workflow has one run going on `main` at a time (`rule:testing/one-run-on-main`); `ci.yml`'s own
// concurrency group cancels an older push's `CI` run too. The push lane runs every job the diff since
// the newest commit whose `CI` run went green reaches, and the nightly deep lane runs the rest
// (`rule:testing/ci-lanes`).
//
// **Every workflow that starts runs of its own is watched**, so no red run goes unseen: `CI`, `Website`,
// `Dev build`, `Release` and `Promote release images`. `build.yml` and `image.yml` start no run, and
// their jobs are judged inside the run that calls them. `listRuns` reads the newest runs on `main`,
// and lists the two workflows with no push trigger apart (`LISTED_APART`), with the runs a published
// release started on its tag. A workflow's runs are judged per trigger
// (`lane`): the nightly `CI` run is not judged by the push run after it, which runs fewer jobs, and the
// `Dev build` run CI's end starts is not judged by the push's own `Dev build` run. A newer green
// dispatched run of a workflow answers each of its lanes, since it runs every job.
//
// **`--watch`** reads the newest run of each lane, passing over a cancelled
// one on an older commit than `origin/main`, which a newer push replaced. A cancelled run or job on
// `origin/main` itself is red: GitHub cancels a job that passes its `timeout-minutes`, and keeps no log
// of it, so the job's annotations are printed instead. It polls the jobs of each run in progress every
// `POLL_BUSY_MS`. A run waiting on an environment's approval is printed once and polled every
// `POLL_IDLE_MS`, and its verdict comes once it goes on. Each poll cancels every run of
// a workflow in `ONE_RUN_ON_MAIN` still going beside a newer one, such as a push run the nightly deep lane started after. It exits 1 the moment a job
// fails, or at once when a newest run already ended red, and prints each failed job, its failed step,
// its URL and the end of its log. The whole log is kept under `.agent-tmp/ci/`. It exits 0 when every
// run it watched in progress ended green. With nothing in progress and nothing red it waits, polling
// every `POLL_IDLE_MS`, for the next run to start. It exits 3 after `--max-min` minutes with nothing to
// report, so a caller under a time limit runs it again. A `gh` that keeps failing for `GH_GIVE_UP` polls
// in a row exits 2. The exit codes are the verdict, and every line it prints is for the reader.
//
// **A watch never waits for a push that only its caller can make.** When its first poll finds no run
// going, a newest run that ended red and was passed over (by `--past` or `--next`), and local `main`
// has commits `origin/main` lacks that change a file outside `NOTES_ONLY`, it exits 5 and lists them:
// the next run starts with a push, and a session whose fix is among them would otherwise wait until
// `--max-min` for a run that never starts. Pushing them is the answer whoever wrote them, since `--push`
// carries every commit on `main` anyway. An exit 4 lists the same commits, since a `--past` job they
// fix is red until they are pushed. `--watch --release` does neither: a push starts no `Release` run.
//
// **`--past <job>`**, given once per job, passes over a failure that somebody else's open work owns, so
// the watch goes on for every other job. It names a job, or the jobs under it: `--past proofs` covers
// `proofs / linux-x86_64`, and `--past "miri units"` covers each shard, such as `miri units (nvs-stdlib
// 1/12)`. A passed-over job is printed once and its log is not read. A run whose every failed job is
// passed over counts as ended, so an idle watch waits for the next run, and a watch that saw such a run
// end exits 4 with no other red. A session that drives CI passes the red a main loop's stage still
// owns this way, which `sleep` cannot.
//
// **`--next`** passes over every run that had already ended when the watch started and whose verdict a
// watch or `--status` printed before, so the watch waits for the next push's runs. A session that has
// just pushed its fixes for a red run watches this way, since that red run would otherwise end the
// watch before the push's own runs are listed. `SEEN` lists the runs whose verdict was printed. A run
// that ended between two watches, such as the `Dev build` run CI's end starts, is not in it, so `--next`
// judges it.
//
// **`--status`** prints what `--watch` would read, once, and exits 1 when a newest run is red.
// **`--rerun`** reruns the failed jobs of each red newest run, for a failure that a fix cannot reach,
// such as a runner that went away. It never reruns a real `Release` run or a `Promote release images`
// run (`publishes`): each publishes something, and a person reruns it.
//
// **`--dispatch full`** starts `ci.yml` on `main` with `workflow_dispatch`, which runs every job.
// **`--dispatch dry-run`** starts `release.yml` on `main` as a dry run, which writes nothing, tags
// nothing and pushes no image. Each argv is one constant in `DISPATCH`, and the dry run's carries
// `dry_run=true`, so no argument turns it into a real release.
// Both refuse with 1 while local `main` has commits `origin/main` lacks that change a file outside
// `NOTES_ONLY`, since the run would test `origin`'s tip and not the tree. Commits to the loop's notes
// alone change nothing a run builds, and `--release-status` passes over them in the same way, so a
// dry run dispatched beside a handoff commit still lands on the commit a full run proved, and skips
// its `verify` job. Both then wait until GitHub lists a run newer than every one before
// the dispatch, and print its address, so a `--watch` started after reads the new run. Five minutes
// with no new run exits 2. A dry run and a real release then cancel each push run of `ci.yml` and
// `dev-build.yml` still queued or going on the commit the `Release` run tests (`REPLACED_BY_RELEASE`),
// and print each: its `verify` job runs every `ci.yml` job and its `build` job runs `build.yml`, so
// the two runs would only compete with it for the account's 5 macOS runners. They never cancel a run on
// another commit, a scheduled, dispatched or pull request run, or a `Release` run. The cancelled commit
// gets no `dev` image, and the next push's CI run still covers it: it diffs against the newest commit
// whose `ci.yml` run went green (`bun nv ci-changes`), and a `Release` run is not one. `--watch` and `--status` pass over the cancelled `CI`
// run.
//
// **`--watch --release`** watches the newest `Release` run on `main` alone,
// with the same polling, report and exit codes. A run waiting on the `release` environment keeps it
// from exiting 0. No command ever cancels a `Release` run: a
// cancelled `publish` is the torn state `release.yml`'s `concurrency` comment exists to prevent.
//
// **`--release-status`** judges the newest completed run of `release.yml` on `main`, dry or real, prints
// each job, and exits 0 green, 1 red and 2 when `gh` or `git` cannot answer. Its `run-name` titles a dry
// run `Release (dry run)`. `judgeRun` is the judgement. The commit the run stands for is local `main`, or
// one that only `NOTES_ONLY` paths changed since: a dry run's own commit, and a real run's tagged commit,
// which `publish` pushed onto `main`. Every job passed, except a `publish` a dry run skips, a `verify`
// skipped beside a green full `ci.yml` run or another green `release.yml` run of the commit
// (`greenFullRun`, the lookup `plan` makes), and
// a failed leg `build.yml` marks optional, which is a warning. It only reads: it never fetches, pushes,
// dispatches or cancels. `--watch --release` passes over a failed optional leg in the same way.
//
// **`--dispatch release`** starts the real release: the one argv with `dry_run=false`. Only a live goal
// that says so runs it (`docs/release.md`). It refuses as the other two do, and also unless the newest
// completed `release.yml` run is a dry run `--release-status` calls green, and while any `release.yml`
// run is going. **`--approve-release`** approves the `release` environment `publish` waits on, in the
// newest real run. It refuses a dry run, a run not waiting, and a run where a job before `publish` did
// not pass, and prints the notes `plan` rendered before it approves. **`--publish-release`** publishes
// that run's draft once every job passed, `docker` included, and the draft carries each asset
// `draftAssets` names: every required leg's archive from `build.yml`, the `.vsix` and the checksums.
// **`--release-published`** is the read-only verdict on the newest release: not a draft, its tag on
// `origin/main`, its `release-promote.yml` run green, and its version tag's manifest read from `ghcr.io`
// with an anonymous token. A `401` or `403` there is a private package, and it prints the visibility
// click as the one step left to the user, since no API makes a package public.
//
// What it spends: three `gh api` calls for the run listings and one per run in progress, every 15
// seconds while a run is in progress, about 1,400 calls an hour of GitHub's 5,000 with three runs
// going, up to `LOGS_KEPT` job logs on disk, and `SEEN_KEPT` run ids in `SEEN`.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";
import { LOCKFILE_DIRS } from "./ci-changes.ts";
import { greenFullRun, type WorkflowRun } from "./release.ts";

export const summary =
  "pushes main, watches GitHub CI and cuts a release: nv ci --push | --check | --watch [--release] [--max-min <n>] [--past <job>] [--next] | --status | --rerun | --dispatch full|dry-run|release | --release-status | --approve-release | --publish-release | --release-published";

const USAGE =
  "usage: bun nv ci --push\n       bun nv ci --check\n       bun nv ci --watch [--release] [--max-min <n>] [--past <job>]... [--next]\n       bun nv ci --status\n       bun nv ci --rerun\n       bun nv ci --dispatch full|dry-run|release\n       bun nv ci --release-status\n       bun nv ci --approve-release\n       bun nv ci --publish-release\n       bun nv ci --release-published";

/**
 * The workflows, by their `name:`, whose run on `main` a newer run of the same workflow replaces and
 * cancels (`rule:testing/one-run-on-main`). Every workflow is watched; only these are cancelled.
 */
export const ONE_RUN_ON_MAIN = ["CI", "Website"];
/** `release.yml`'s `name:`, the one workflow `--watch --release` watches. No command cancels its runs. */
export const RELEASE = "Release";
/** The workflows, by their `name:`, whose runs call `build.yml`, where a failed optional leg is passed over. */
const CALLS_BUILD = ["Release", "Dev build"];
/** `release-promote.yml`'s `name:`. A published release starts it on the release's tag. */
export const PROMOTE = "Promote release images";
/** The title `release.yml`'s `run-name` gives a dry run. */
export const DRY_RUN_TITLE = "Release (dry run)";
/**
 * The `name:` of each workflow file that starts runs of its own, which `--watch` judges. The runs API
 * gives a run's `run-name` title as its `name`, so a dry run's `name` is `Release (dry run)`, and
 * `workflowOf` reads the workflow from `path`. A file that only `workflow_call` starts is not here: its
 * jobs are judged inside the run that calls it. A test holds this table to `.github/workflows/`.
 */
export const WORKFLOW_FILES: Record<string, string> = {
  "ci.yml": "CI",
  "pages.yml": "Website",
  "dev-build.yml": "Dev build",
  "release.yml": RELEASE,
  "release-promote.yml": PROMOTE,
};
/**
 * The workflow files, with their `name:`, whose push run on a commit a `Release` run on the same commit
 * replaces: its `verify` job is `ci.yml` with every job on, and its `build` job is `build.yml`, which
 * the push's `dev-build.yml` run calls. `--dispatch dry-run` and `--dispatch release` cancel them.
 */
export const REPLACED_BY_RELEASE: Record<string, string> = { "ci.yml": "CI", "dev-build.yml": "Dev build" };
const BRANCH = "main";

/**
 * What `--dispatch` starts: each argv is a constant, so no argument can change it. A `workflow_dispatch`
 * of `ci.yml` runs every job. `dry-run`'s argv always carries `dry_run=true`, and `release`'s is the one
 * argv with `dry_run=false`, which `dispatch` starts only beside a green dry run of `main`'s tip.
 */
export const DISPATCH = {
  full: { workflow: "CI", file: "ci.yml", argv: ["workflow", "run", "ci.yml", "--ref", BRANCH] },
  "dry-run": {
    workflow: RELEASE,
    file: "release.yml",
    argv: ["workflow", "run", "release.yml", "--ref", BRANCH, "-f", "bump=patch", "-f", "dry_run=true"],
  },
  release: {
    workflow: RELEASE,
    file: "release.yml",
    argv: ["workflow", "run", "release.yml", "--ref", BRANCH, "-f", "bump=patch", "-f", "dry_run=false"],
  },
} as const;
type DispatchKind = keyof typeof DISPATCH;
/** How long `--dispatch` waits for GitHub to list the run it started. */
const DISPATCH_WAIT_MS = 5 * 60_000;

/** The job `release.yml`'s `publish` is, which a dry run skips, and the one its `test` is. */
export const PUBLISH_JOB = "tag, changelog and draft release";
export const VERIFY_JOB = "verify";
/** The reusable-workflow job `release.yml`'s `docker` is, whose jobs run after `publish`. */
export const DOCKER_JOB = "docker";
/** The environment `publish` waits on, which `--approve-release` approves. */
export const RELEASE_ENV = "release";
/** The notes artifact `plan` uploads, and the file in it. */
const NOTES_ARTIFACT = "release-notes-preview";
const NOTES_FILE = "release-notes.md";
/** The workflow `release: published` starts, which moves `latest` and `MAJOR.MINOR`. */
const PROMOTE_FILE = "release-promote.yml";
/** The environment that workflow's Marketplace publish waits on, approved by a person in the Actions tab. */
const MARKETPLACE_ENV = "marketplace";
/** The container image a release pushes to `ghcr.io`, and the media types its manifest may have. */
export const IMAGE = "novis-lang/novis";
const MANIFEST_TYPES = [
  "application/vnd.oci.image.index.v1+json",
  "application/vnd.docker.distribution.manifest.list.v2+json",
  "application/vnd.oci.image.manifest.v1+json",
  "application/vnd.docker.distribution.manifest.v2+json",
].join(", ");
/** The paths a commit after a dry run may change and leave its verdict standing: the loop's own notes. */
export const NOTES_ONLY = ["docs/goals/", "docs/agent/playbook/", "docs/agent/playbook.md"];

/** How often a run in progress is polled, and how often an idle watch looks for a new run. */
export const POLL_BUSY_MS = 15_000;
export const POLL_IDLE_MS = 60_000;

/** How many unpushed commits an exit 4 or 5 of `--watch` lists. */
const UNPUSHED_SHOWN = 10;

/** How many failed polls in a row end the watch. */
const GH_GIVE_UP = 20;
/** The default `--max-min`: under a two-hour background limit. */
const MAX_MIN = 110;
/** How many lines of a failed job's log are printed, and how many logs `.agent-tmp/ci/` keeps. */
const TAIL_LINES = 40;
const LOGS_KEPT = 20;
/** How many earlier lines that name a failure are printed above the tail, and how one is told. nextest
 * indents its status lines, such as `        FAIL [   0.193s] nvs-cli::run_command <test>`. */
const MARKED_LINES = 20;
const FAILURE_LINE =
  /^(FAIL\b|FAILED\b|error(\[E\d+\])?:|test .* \.\.\. FAILED|thread '.*' panicked|failures:|---- )|^\s+(FAIL|TIMEOUT|SIGSEGV|SIGABRT|SIGKILL|ABORT) \[|##\[error\]/;
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
  // `lint`'s `rustdoc links` step. It takes about a minute once warm, and writes only `target/doc`.
  {
    name: "rustdoc links",
    argv: ["cargo", "doc", "--no-deps", "--workspace"],
    env: { RUSTDOCFLAGS: "-A rustdoc::private_intra_doc_links -D warnings" },
  },
  { name: "build", argv: ["cargo", "build"] },
  // `nv reference` runs each example in a folder of its own, so the binary's path is absolute.
  { name: "chapter examples", argv: ["bun", "nv", "reference"], env: { NVS_BIN: join(ROOT, DEBUG_NVS) } },
  { name: "feature proofs", argv: ["bun", "nv", "proofs", "--gate", "--limit", "0", "--nvs", DEBUG_NVS] },
  { name: "website render", argv: ["bun", "nv", "render", "--website", "--check"], env: { NVS_BIN: DEBUG_NVS } },
  { name: "website stale guard", argv: ["bun", "nv", "site", "--check", "structure", "snippets", "stale", "questions", "syntax", "guides", "tools", "in-depth", "roadmap", "prose", "apps", "code", "--nvs", DEBUG_NVS] },
];

/** Where a refused push leaves its red checks, under the root. */
export const REFUSED = ".agent-tmp/ci/refused-push.txt";
/** The ended runs whose verdict a watch or `--status` printed, one id per line, under the root. */
export const SEEN = ".agent-tmp/ci/seen-runs.txt";
const SEEN_KEPT = 500;
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
  /** One HTTP GET with no credential, for `--release-published`'s anonymous registry read. */
  fetch(url: string, headers: Record<string, string>): Promise<{ status: number; body: string }>;
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
  fetch: async (url, headers) => {
    try {
      const r = await fetch(url, { headers, signal: AbortSignal.timeout(30_000) });
      return { status: r.status, body: await r.text() };
    } catch (e) {
      throw new GhFailed(`GET ${url} failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  },
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
  /** When the run was created, as ISO 8601. */
  created_at?: string;
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

/**
 * The workflow files with no push trigger, whose runs are listed apart: a run of either may be older
 * than the 100 newest runs on `main`, and a published release starts `release-promote.yml` on its tag.
 * A test holds this list to `.github/workflows/`.
 */
export const LISTED_APART = ["release.yml", "release-promote.yml"];

/**
 * The newest runs on `main`, and the newest runs of each file of `LISTED_APART` on `main` or started by
 * a published release. Newest first, each once, each named by its workflow. The 100 runs on `main`
 * reach back about a day, past the nightly run before the newest.
 */
export async function listRuns(s: CiSeams): Promise<Run[]> {
  const jq = "[.workflow_runs[] | {id, name, event, status, conclusion, head_sha, head_branch, html_url, display_title, path}]";
  const onMain = await ghJson<Run[]>(s, ["api", `repos/{owner}/{repo}/actions/runs?branch=${BRANCH}&per_page=100`, "--jq", jq]);
  const apart: Run[] = [];
  for (const f of LISTED_APART) {
    const runs = await ghJson<(Run & { head_branch?: string })[]>(s, ["api", `repos/{owner}/{repo}/actions/workflows/${f}/runs?per_page=10`, "--jq", jq]);
    apart.push(...runs.filter((r) => r.head_branch === BRANCH || r.event === "release"));
  }
  const ids = new Set<number>();
  return [...onMain, ...apart]
    .filter((r) => !ids.has(r.id) && ids.add(r.id))
    .sort((a, b) => b.id - a.id)
    .map(workflowOf);
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
 * Whether `r` is a push run of a workflow in `REPLACED_BY_RELEASE` that a newer `Release` run in `runs`
 * on the same commit replaces. `--dispatch dry-run` and `--dispatch release` cancel such runs.
 */
function releaseReplaced(runs: Run[], r: Run): boolean {
  return r.event === "push" && Object.values(REPLACED_BY_RELEASE).includes(r.name)
    && runs.some((x) => x.name === RELEASE && x.head_sha === r.head_sha && x.id > r.id);
}

/** The lane a run is judged in: its workflow file and the event that started it. */
const lane = (r: Run) => `${r.path ?? r.name} ${r.event}`;

/**
 * Whether `r` is a `Release` run, and `runs` has another `Release` run of its lane on its commit that was
 * not cancelled. Such a run answers a cancelled `r`: two `Release` runs dispatched on one commit share
 * `release.yml`'s concurrency group, and GitHub cancels the waiting one before any job starts.
 */
function answeredBySibling(runs: Run[], r: Run): boolean {
  return r.name === RELEASE
    && runs.some((x) => x.id !== r.id && lane(x) === lane(r) && x.head_sha === r.head_sha && x.conclusion !== "cancelled");
}

/**
 * The newest run of each lane, by workflow name and then event, passing over each cancelled run on a
 * commit other than `tip`, `origin/main`'s. A newer push cancels those, and its own runs may not be
 * listed yet. A cancelled push run on `tip` that a newer `Release` run of `tip` replaces is passed over
 * too: `--dispatch` cancelled it, and the `Release` run is watched. So is a cancelled `Release` run on
 * `tip` that another `Release` run of its lane on `tip` answers (`answeredBySibling`). Any other cancelled run on
 * `tip` stopped for another reason, such as a job's time limit. A lane whose newest run is older than
 * a green `workflow_dispatch` run of its workflow is left out: a dispatched run runs every job of its
 * file, so a dispatched `CI` run answers the nightly run, and a dispatched promote run answers the
 * release's. With `only`, the lanes of those workflows alone.
 */
export function newest(runs: Run[], tip: string, only?: readonly string[]): Run[] {
  const kept = (x: Run) =>
    x.conclusion !== "cancelled" || (x.head_sha === tip && !releaseReplaced(runs, x) && !answeredBySibling(runs, x));
  const byLane = new Map<string, Run>();
  for (const r of runs.filter((x) => (only === undefined || only.includes(x.name)) && kept(x))) {
    const was = byLane.get(lane(r));
    if (was === undefined || r.id > was.id) byLane.set(lane(r), r);
  }
  const answered = (r: Run) =>
    runs.some((x) => x.name === r.name && x.event === "workflow_dispatch" && x.id > r.id && x.conclusion === "success");
  return [...byLane.values()]
    .filter((r) => !answered(r))
    .sort((a, b) => a.name.localeCompare(b.name) || a.event.localeCompare(b.event));
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
 * The runs of each workflow of `workflows` still going that a newer run replaces: every one but the newest,
 * and with `head` given, every one on another commit too, since its run may not be listed yet. It never
 * names a `Release` run: a cancelled `publish` is the torn state `release.yml`'s `concurrency` exists to
 * prevent.
 */
export function superseded(runs: Run[], head?: string, workflows: readonly string[] = ONE_RUN_ON_MAIN): Run[] {
  const out: Run[] = [];
  for (const w of workflows.filter((x) => x !== RELEASE)) {
    const going = runs.filter((x) => x.name === w && !done(x)).sort((a, b) => b.id - a.id);
    out.push(...going.filter((r, i) => i > 0 || (head !== undefined && r.head_sha !== head)));
  }
  return out;
}

/** Cancels each run `superseded` names that `asked` has not cancelled already. A failed cancel only warns. */
async function cancelSuperseded(s: CiSeams, runs: Run[], head: string | undefined, asked: Set<number>, workflows: readonly string[] = ONE_RUN_ON_MAIN): Promise<void> {
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
 * under `proofs`, a matrix shard, such as `miri units (nvs-stdlib 1/12)` under `miri units`, or a leg
 * that names several targets, such as `fuzz smoke / markdown, markdown_from_html` under
 * `fuzz smoke / markdown`.
 */
const under = (name: string, p: string) =>
  name === p || [" / ", " (", ", "].some((sep) => name.startsWith(`${p}${sep}`));

/** The runs `SEEN` names, none before a watch or `--status` wrote it. */
function readSeen(root: string): Set<number> {
  const path = join(root, SEEN);
  if (!existsSync(path)) return new Set();
  return new Set(readFileSync(path, "utf8").split(/\r?\n/).filter((l) => /^\d+$/.test(l)).map(Number));
}

/** Adds the runs `ids` to `SEEN`, which keeps the newest `SEEN_KEPT`. */
function addSeen(root: string, ids: number[]): void {
  if (ids.length === 0) return;
  const all = new Set([...readSeen(root), ...ids]);
  mkdirSync(join(root, ".agent-tmp", "ci"), { recursive: true });
  writeFileSync(join(root, SEEN), `${[...all].sort((a, b) => a - b).slice(-SEEN_KEPT).join("\n")}\n`);
}

/**
 * Watches until a job fails (1), every run it saw in progress is green (0), or `maxMin` passes (3). A
 * failed job that `past` names is passed over, and a watched run that ended red in those jobs alone
 * ends the watch with 4. With `next`, every run that had ended at the first poll and that `SEEN` names
 * is passed over. A
 * first poll with no run going, a passed-over red run and unpushed commits that change code exits 5.
 * It watches every lane, or with `only` (`[RELEASE]` under `--release`) those workflows' lanes alone,
 * where a run waiting on an environment's approval keeps the watch from exiting 0. A failed leg
 * `build.yml` marks optional is passed over in each run of `CALLS_BUILD`: a release ships without it.
 */
export async function watch(s: CiSeams, maxMin: number, past: string[] = [], next = false, only?: readonly string[]): Promise<number> {
  if (sayRefused(s)) return 1;
  const releaseOnly = only?.includes(RELEASE) === true;
  const until = s.now() + maxMin * 60_000;
  const watched = new Set<number>();
  const before = new Set<number>();
  const finished = new Map<number, number>();
  const passedOver = new Set<number>();
  const waitedOn = new Set<number>();
  const seenBefore = readSeen(s.root);
  // `release.yml`'s job `build` and `dev-build.yml`'s job `release binaries` both call `build.yml`.
  const optional = optionalLegs(ROOT).map((n) => ` / ${n.slice("build / ".length)}`);
  const passed = (r: Run, j: Job) =>
    past.some((p) => under(j.name, p)) || (CALLS_BUILD.includes(r.name) && optional.some((o) => j.name.endsWith(o)));
  /** Prints each failed job of `jobs` that `past` names, once, and reports whether any other failed. */
  const sort = (r: Run, jobs: Job[]): boolean => {
    for (const j of jobs.filter((x) => red(x) && passed(r, x) && !passedOver.has(x.id))) {
      passedOver.add(j.id);
      s.say(`  passing over FAILED ${r.name} / ${j.name}  ${j.html_url}`);
    }
    return jobs.some((j) => red(j) && !passed(r, j));
  };
  const cancelled = new Set<number>();
  // The runs whose verdict this watch printed, which `SEEN` records whatever the exit.
  const shown = new Set<number>();
  const end = (code: number): number => {
    addSeen(s.root, [...shown]);
    return code;
  };
  let endedRed = false;
  let failures = 0;
  let first = true;
  while (true) {
    let busy = false;
    let holding = false;
    try {
      const all = await listRuns(s);
      await cancelSuperseded(s, all, undefined, cancelled, only ?? ONE_RUN_ON_MAIN);
      const runs = newest(all, await tipOf(s), only);
      failures = 0;
      for (const r of runs) {
        if (first && next && done(r) && seenBefore.has(r.id)) {
          before.add(r.id);
          s.say(`passing over ${runLine(r)}`);
        }
        if (before.has(r.id)) continue;
        if (done(r) && red(r)) {
          const jobs = await listJobs(s, r.id);
          if (first || watched.has(r.id)) s.say(runLine(r));
          shown.add(r.id);
          if (sort(r, jobs) || !jobs.some(red)) {
            await report(s, r, jobs.filter((j) => !passed(r, j)));
            return end(1);
          }
          if (watched.has(r.id)) endedRed = true;
          continue;
        }
        if (first && done(r)) {
          s.say(runLine(r));
          shown.add(r.id);
        }
        if (done(r)) continue;
        if (r.status === "waiting") {
          // A person approves an environment, so the run is not polled as one in progress.
          if (!waitedOn.has(r.id)) s.say(`waiting on an approval: ${runLine(r)}`);
          waitedOn.add(r.id);
          holding ||= releaseOnly;
          continue;
        }
        busy = true;
        const jobs = await listJobs(s, r.id);
        if (!watched.has(r.id)) {
          watched.add(r.id);
          s.say(`watching ${runLine(r)}`);
        }
        if (sort(r, jobs)) {
          shown.add(r.id);
          await report(s, r, jobs.filter((j) => !passed(r, j)));
          return end(1);
        }
        const n = jobs.filter(done).length;
        if (finished.get(r.id) !== n) {
          finished.set(r.id, n);
          s.say(`  ${r.name}: ${n} of ${jobs.length} jobs finished green`);
        }
      }
      if (first && !busy && watched.size === 0 && !releaseOnly && runs.some((r) => done(r) && red(r))) {
        const owed = await unpushed(s);
        if (owed.length > 0) {
          s.say(`no run is going, the newest run ended red, and local ${BRANCH} has commits origin/${BRANCH} lacks:`);
          for (const l of owed) s.say(`  ${l}`);
          s.say("push them with `bun nv ci --push`: no run starts until they are pushed");
          return end(5);
        }
      }
      first = false;
      if (!busy && !holding && watched.size > 0) {
        if (endedRed) {
          s.say("every watched run ended, and only the jobs `--past` names failed");
          const owed = releaseOnly ? [] : await unpushed(s);
          if (owed.length > 0) {
            s.say(`local ${BRANCH} has commits origin/${BRANCH} lacks: push them with \`bun nv ci --push\` if one fixes a \`--past\` job`);
            for (const l of owed) s.say(`  ${l}`);
          }
          return end(4);
        }
        for (const r of runs.filter((x) => done(x) && !red(x))) {
          s.say(`GREEN ${runLine(r)}`);
          shown.add(r.id);
        }
        return end(0);
      }
    } catch (e) {
      if (!(e instanceof GhFailed) && !(e instanceof SyntaxError)) throw e;
      failures++;
      s.say(`WATCH ERROR (${failures} of ${GH_GIVE_UP}): ${e.message}`);
      if (failures >= GH_GIVE_UP) return end(2);
    }
    if (s.now() >= until) {
      s.say(`nothing failed in ${maxMin} minutes: run \`bun nv ci --watch\` again`);
      return end(3);
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

/** How many commits of local `main` `origin/main` lacks, as the last fetch or push left it. */
async function aheadCount(s: CiSeams): Promise<number> {
  const n = Number(await gitOut(s, ["rev-list", "--count", `origin/${BRANCH}..${BRANCH}`]));
  return Number.isFinite(n) ? n : 0;
}

/** The files local `main` changes that `origin/main` lacks, outside `NOTES_ONLY`. */
async function unpushedCodeFiles(s: CiSeams): Promise<string[]> {
  const changed = (await gitOut(s, ["diff", "--name-only", `origin/${BRANCH}`, BRANCH])).split(/\r?\n/).filter((l) => l !== "");
  return changed.filter((p) => !NOTES_ONLY.some((n) => p.startsWith(n)));
}

/**
 * The newest `UNPUSHED_SHOWN` commits local `main` has that `origin/main` lacks, one line each, when
 * any of them changes a file outside `NOTES_ONLY`. Otherwise none: a push of the loop's notes alone
 * changes nothing a run builds.
 */
async function unpushed(s: CiSeams): Promise<string[]> {
  if ((await aheadCount(s)) === 0 || (await unpushedCodeFiles(s)).length === 0) return [];
  const log = await gitOut(s, ["log", "--oneline", "-n", String(UNPUSHED_SHOWN), `origin/${BRANCH}..${BRANCH}`]);
  return log.split(/\r?\n/).filter((l) => l !== "");
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
    s.say(`pushed ${BRANCH} at ${short(head)}: the push starts its CI, Website and Dev build runs`);
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
  addSeen(s.root, runs.filter(done).map((r) => r.id));
  return bad > 0 ? 1 : 0;
}

/** Whether a rerun of `r` publishes something: a real `Release` run, or a `Promote release images` run. */
export const publishes = (r: Run) => (r.name === RELEASE && r.display_title !== DRY_RUN_TITLE) || r.name === PROMOTE;

async function rerun(s: CiSeams): Promise<number> {
  const all = newest(await listRuns(s), await tipOf(s)).filter((r) => done(r) && red(r));
  for (const r of all.filter(publishes)) s.say(`not rerun, since a rerun publishes: ${runLine(r)}. A person reruns it in the Actions tab.`);
  const reds = all.filter((r) => !publishes(r));
  if (reds.length === 0) s.say("no newest run that may be rerun is red: nothing to rerun");
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
    "[.workflow_runs[] | {id, name, event, status, conclusion, head_sha, head_branch, html_url, display_title, path, created_at}]",
  ]);
  return runs.map(workflowOf);
}

/**
 * The runs of `runs` that a `Release` run on `sha` replaces: each run a push to `main` started on `sha`
 * that is still queued or going. It never names a run on another commit, a scheduled, dispatched or
 * pull request run, or a `Release` run.
 */
export function replacedByRelease(runs: (Run & { head_branch?: string | null })[], sha: string): Run[] {
  return runs.filter((r) => r.event === "push" && r.head_sha === sha && (r.head_branch ?? BRANCH) === BRANCH && !done(r) && r.name !== RELEASE);
}

/**
 * Cancels each push run of a file in `REPLACED_BY_RELEASE` that `replacedByRelease` names for `sha`,
 * and prints each one. The `Release` run has started either way, so a failed listing or cancel only
 * warns.
 */
async function cancelReplaced(s: CiSeams, sha: string): Promise<void> {
  for (const file of Object.keys(REPLACED_BY_RELEASE)) {
    let runs: (Run & WorkflowRun)[];
    try {
      runs = await workflowRuns(s, file, `&event=push&head_sha=${sha}`);
    } catch (e) {
      if (!(e instanceof GhFailed) && !(e instanceof SyntaxError)) throw e;
      s.say(`the push runs of ${file} could not be listed: ${e.message}`);
      continue;
    }
    for (const r of replacedByRelease(runs, sha)) {
      const c = await s.gh(["run", "cancel", String(r.id)]);
      s.say(c.code === 0 ? `cancelled ${runLine(r)}: the Release run on ${short(sha)} runs every job it would` : `could not cancel ${r.id}: ${firstLine(c)}`);
    }
  }
}

/** Whether `r`, a run of `release.yml`, is a dry run: its `run-name` says so. */
const isDry = (r: Run) => r.display_title === DRY_RUN_TITLE;

/** The runs `--dispatch` may have started: a `workflow_dispatch` one, and of a release, a dry or a real run. */
function dispatched(kind: DispatchKind, r: Run): boolean {
  if (r.event !== "workflow_dispatch") return false;
  if (kind === "dry-run") return isDry(r);
  if (kind === "release") return r.display_title === RELEASE;
  return true;
}

/**
 * Starts the run `DISPATCH[kind]` names on `main`, then waits until GitHub lists a run of it newer than
 * every one before, and prints its address. It refuses while local `main` has commits `origin/main`
 * lacks that change a file outside `NOTES_ONLY`, since the run would test `origin`'s tip and not the
 * tree. A real release also refuses unless the newest completed `release.yml` run is a green dry run
 * that stands for local `main`, and while any `release.yml` run is going. Five minutes with no new run
 * exits 2.
 */
export async function dispatch(s: CiSeams, kind: DispatchKind): Promise<number> {
  const d = DISPATCH[kind];
  await gitOut(s, ["fetch", "-q", "origin", BRANCH]);
  const ahead = await aheadCount(s);
  if (ahead > 0) {
    const code = await unpushedCodeFiles(s);
    if (code.length > 0) {
      s.say(`local ${BRANCH} has ${ahead} commit(s) origin/${BRANCH} lacks, which change ${code.slice(0, 5).join(", ")}: run \`bun nv ci --push\` first, so the run tests the tree`);
      return 1;
    }
    s.say(`local ${BRANCH} has ${ahead} commit(s) origin/${BRANCH} lacks, which change only the loop's notes: the run tests origin/${BRANCH}'s tip`);
  }
  if (kind === "release") {
    const going = (await workflowRuns(s, d.file)).filter((r) => !done(r));
    if (going.length > 0) {
      for (const r of going) s.say(`going: ${runLine(r)}`);
      s.say("REFUSED a Release run is going: dispatch the real release once it ends");
      return 1;
    }
    const v = await releaseVerdict(s);
    for (const l of v.lines) s.say(l);
    if (!v.green || v.run === undefined || !isDry(v.run)) {
      s.say(`REFUSED a real release needs a green dry run of local ${BRANCH}'s tip as the newest completed Release run: \`bun nv ci --dispatch dry-run\``);
      return 1;
    }
  }
  const before = Math.max(0, ...(await workflowRuns(s, d.file)).map((r) => r.id));
  const started = await s.gh([...d.argv]);
  if (started.code !== 0) {
    s.say(`could not dispatch ${d.file}: ${firstLine(started)}`);
    return 1;
  }
  const as = kind === "dry-run" ? " as a dry run" : kind === "release" ? " as a real release" : "";
  s.say(`dispatched ${d.file} on ${BRANCH}${as}: waiting for GitHub to list the run`);
  const until = s.now() + DISPATCH_WAIT_MS;
  while (true) {
    await s.sleep(POLL_BUSY_MS);
    try {
      const fresh = (await workflowRuns(s, d.file)).filter((r) => r.id > before && dispatched(kind, r)).sort((a, b) => a.id - b.id)[0];
      if (fresh !== undefined) {
        s.say(`started ${runLine(fresh)}`);
        if (kind !== "full") await cancelReplaced(s, fresh.head_sha);
        s.say(`watch it with \`bun nv ci --watch${kind === "full" ? "" : " --release"}\``);
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

/**
 * Which run `judgeRun` judges: a completed dry run, a completed real run, or a real run waiting on the
 * `release` environment, which `--approve-release` judges before it approves.
 */
export type Stage = "dry" | "real" | "approve";

/** What `judgeRun` reads: the run, its jobs, the `ci.yml` runs of its commit, and what changed since. */
export interface RunFacts {
  run: Run;
  jobs: Job[];
  stage: Stage;
  /** The `ci.yml` runs of the run's commit, for a `verify` job it skipped. */
  ciRuns: WorkflowRun[];
  /**
   * The paths changed between the commit the run stands for and local `main`, or null when `main` does
   * not contain it. A dry run stands for its own commit, and a real run for the commit its tag names.
   * Left out, the path rule is not judged, as `--approve-release` and `--publish-release` do.
   */
  changed?: string[] | null;
  /** The job names of `build.yml`'s legs marked `optional: true`, as the release shows them. */
  optional: string[];
}

/** A judge's verdict: green, and the lines it prints. */
export interface Verdict {
  green: boolean;
  lines: string[];
}

/**
 * Judges one run of `release.yml` as the workflow's own `if:` lines do. Its commit stands for local
 * `main` when the paths changed since touch only `NOTES_ONLY`. Every job passed, with these exceptions.
 * `publish` is skipped in a dry run, and waits on `release` in a run to approve, whose `docker` jobs come
 * after it and are not judged. `verify` may be skipped only beside a green full `ci.yml` run of that
 * commit, the lookup `plan` makes. A failed optional build leg is a warning, since the release ships
 * without it.
 */
export function judgeRun(f: RunFacts): Verdict {
  const kind = f.stage === "dry" ? "dry run" : "release run";
  const lines = [`${kind} ${runLine(f.run)}`];
  let green = true;
  const fail = (l: string) => {
    green = false;
    lines.push(l);
  };
  const again = f.stage === "dry" ? "dispatch a new dry run" : "fetch and merge origin/main, or cut a new release";
  if (f.changed === null) fail(`RED the commit it stands for is not on local ${BRANCH}: ${again}`);
  else if (f.changed !== undefined) {
    const code = f.changed.filter((p) => !NOTES_ONLY.some((n) => p.startsWith(n)));
    if (code.length > 0) fail(`RED ${code.length} file(s) outside the loop's notes changed since its commit, such as ${code.slice(0, 5).join(", ")}: ${again}`);
  }
  if (f.jobs.length === 0) fail("RED the run lists no jobs");
  for (const j of f.jobs) {
    const step = j.steps.find(red)?.name;
    const where = `${j.name}${step === undefined ? "" : ` / step "${step}"`}  ${j.html_url}`;
    if (j.name.includes("${{")) fail(`RED ${j.name}: a job whose matrix did not expand  ${j.html_url}`);
    else if (j.name === PUBLISH_JOB && f.stage === "dry") {
      if (j.conclusion === "skipped") lines.push(`  ok ${j.name}: skipped, as in every dry run`);
      else fail(`RED ${j.name} ended ${j.conclusion ?? j.status}, and a dry run skips it  ${j.html_url}`);
    } else if (j.name === PUBLISH_JOB && f.stage === "approve") {
      if (j.status === "waiting") lines.push(`  waiting ${j.name}: on the ${RELEASE_ENV} environment`);
      else fail(`RED ${j.name} is ${j.conclusion ?? j.status}, and not waiting on the ${RELEASE_ENV} environment  ${j.html_url}`);
    } else if (under(j.name, DOCKER_JOB) && f.stage === "approve") {
      lines.push(`  after publish ${j.name}: ${j.conclusion ?? j.status}`);
    } else if (j.name === VERIFY_JOB && j.conclusion === "skipped") {
      const full = greenFullRun(f.ciRuns, f.run.head_sha);
      if (full !== null) lines.push(`  ok ${j.name}: skipped beside the green full run ${full.html_url}`);
      else fail(`RED ${j.name} was skipped, and no full ci.yml run or other release.yml run of ${short(f.run.head_sha)} on ${BRANCH} is green`);
    } else if (j.conclusion === "success") lines.push(`  ok ${j.name}`);
    else if (f.optional.includes(j.name) && (j.conclusion === "failure" || j.conclusion === "timed_out")) {
      lines.push(`  WARNING ${where}: an optional leg, so the release ships without it`);
    } else fail(`FAILED ${where} (${j.conclusion ?? j.status})`);
  }
  const at = short(f.run.head_sha);
  if (f.stage === "approve") lines.push(green ? `GREEN every job before ${PUBLISH_JOB} passed on ${at}` : "RED the run is not ready to approve");
  else lines.push(green ? `GREEN the ${kind} on ${at} passed` : `RED the ${kind} does not stand for ${BRANCH}`);
  return { green, lines };
}

/** One leg of `build.yml`'s matrix: its name, its archive's extension, and whether the release needs it. */
export interface Leg {
  name?: string;
  archive?: string;
  optional?: boolean;
}

const BUILD_YML = ".github/workflows/build.yml";

function parseLegs(text: string): Leg[] {
  const wf = (Bun.YAML.parse(text) ?? {}) as { jobs?: { build?: { strategy?: { matrix?: { include?: Leg[] } } } } };
  return wf.jobs?.build?.strategy?.matrix?.include ?? [];
}

function buildLegs(root: string): Leg[] {
  return parseLegs(readFileSync(join(root, BUILD_YML), "utf8"));
}

/**
 * The legs of `build.yml` at commit `sha`, which are the legs a run of that commit built. A judge of a
 * finished run reads these and not the tree's, since a later commit may add, drop or demote a leg.
 */
async function legsAt(s: CiSeams, sha: string): Promise<Leg[]> {
  return parseLegs(await gitOut(s, ["show", `${sha}:${BUILD_YML}`]));
}

/** The job names of `build.yml`'s optional legs, as a run of `release.yml` shows them. */
export function optionalLegs(root: string, legs: Leg[] = buildLegs(root)): string[] {
  return legs.filter((l) => l.optional === true && l.name !== undefined).map((l) => `build / build ${l.name}`);
}

/**
 * The assets a whole draft of `version` carries: each required leg's archive, the extension and the
 * checksums. They are the files `publish`'s *Every required archive arrived* step checks, and the names
 * `bun nv release --package` gives them.
 */
export function draftAssets(root: string, version: string, all: Leg[] = buildLegs(root)): string[] {
  const legs = all.filter((l) => l.optional !== true && l.name !== undefined && l.archive !== undefined);
  return [...legs.map((l) => `nvs-${version}-${l.name}.${l.archive}`), `nvs-${version}.vsix`, "SHA256SUMS"];
}

/** A GitHub release, as the releases API gives it. */
export interface GhRelease {
  tag_name: string;
  draft: boolean;
  prerelease: boolean;
  created_at: string;
  html_url: string;
  assets: string[];
}

/** The newest releases, drafts included, newest first. */
async function listReleases(s: CiSeams): Promise<GhRelease[]> {
  const rels = await ghJson<GhRelease[]>(s, [
    "api",
    "repos/{owner}/{repo}/releases?per_page=20",
    "--jq",
    "[.[] | {tag_name, draft, prerelease, created_at, html_url, assets: [.assets[].name]}]",
  ]);
  return rels.sort((a, b) => b.created_at.localeCompare(a.created_at));
}

/** The release real run `run` drafted: the newest one created after the run started, or null. */
async function releaseOf(s: CiSeams, run: Run): Promise<GhRelease | null> {
  return (await listReleases(s)).find((r) => r.created_at >= (run.created_at ?? "")) ?? null;
}

/** The commit tag `tag` names on GitHub. */
function tagCommit(s: CiSeams, tag: string): Promise<string> {
  return ghJson<string>(s, ["api", `repos/{owner}/{repo}/commits/${tag}`, "--jq", ".sha | tojson"]);
}

/** The `ci.yml` and `release.yml` runs of `run`'s commit, read only when its `verify` job was skipped.
 * `run` itself is left out, so a run is never its own proof. */
async function ciRunsFor(s: CiSeams, run: Run, jobs: Job[]): Promise<WorkflowRun[]> {
  const skipped = jobs.some((j) => j.name === VERIFY_JOB && j.conclusion === "skipped");
  if (!skipped) return [];
  const query = `&head_sha=${run.head_sha}`;
  const [ci, release] = await Promise.all([workflowRuns(s, "ci.yml", query), workflowRuns(s, "release.yml", query)]);
  return [...ci, ...release].filter((r) => r.id !== run.id);
}

/** The newest run of `release.yml` on `main`, dry or real, with every run before it, newest first. */
async function releaseRuns(s: CiSeams): Promise<(Run & WorkflowRun)[]> {
  return (await workflowRuns(s, "release.yml")).sort((a, b) => b.id - a.id);
}

/**
 * The verdict on the newest completed run of `release.yml` on `main`, dry or real, and that run. A dry
 * run stands for its own commit, and a real run for the commit its tag names, which `publish` pushed
 * onto `main`. It reads only, and never fetches, pushes, dispatches or cancels.
 */
async function releaseVerdict(s: CiSeams): Promise<Verdict & { run?: Run }> {
  const runs = await releaseRuns(s);
  const lines = runs.filter((x) => !done(x)).map((r) => `going, and not judged until it ends: ${runLine(r)}`);
  const run = runs.find(done);
  if (run === undefined) {
    lines.push(`RED no run of release.yml on ${BRANCH} has completed: start a dry run with \`bun nv ci --dispatch dry-run\``);
    return { green: false, lines };
  }
  const stage: Stage = isDry(run) ? "dry" : "real";
  let base = run.head_sha;
  if (stage === "real") {
    const rel = await releaseOf(s, run);
    if (rel !== null) {
      base = await tagCommit(s, rel.tag_name);
      lines.push(`its tag ${rel.tag_name} names ${short(base)}`);
    }
  }
  const main = await gitOut(s, ["rev-parse", BRANCH]);
  let changed: string[] | null = [];
  if (base !== main) {
    const anc = await s.git(["merge-base", "--is-ancestor", base, BRANCH]);
    if (anc.code !== 0) changed = null;
    else changed = (await gitOut(s, ["diff", "--name-only", base, BRANCH])).split(/\r?\n/).filter((l) => l !== "");
  }
  const jobs = await listJobs(s, run.id);
  const optional = optionalLegs(ROOT, await legsAt(s, run.head_sha));
  const v = judgeRun({ run, jobs, stage, ciRuns: await ciRunsFor(s, run, jobs), changed, optional });
  return { green: v.green, lines: [...lines, ...v.lines], run };
}

/** `--release-status`: 0 green, 1 red, 2 when `gh` or `git` cannot answer. */
async function releaseStatus(s: CiSeams): Promise<number> {
  const v = await releaseVerdict(s);
  for (const l of v.lines) s.say(l);
  return v.green ? 0 : 1;
}

// ---- --approve-release and --publish-release ----------------------------------------------------------

/** The newest run of `release.yml`, which must be a real one: null, and a line said, when it is not. */
async function newestReal(s: CiSeams, verb: string): Promise<Run | null> {
  const run = (await releaseRuns(s))[0];
  if (run === undefined) s.say(`REFUSED no run of release.yml on ${BRANCH}: nothing to ${verb}`);
  else if (isDry(run)) s.say(`REFUSED the newest Release run is a dry run, which has nothing to ${verb}: ${runLine(run)}`);
  else return run;
  return null;
}

/**
 * Approves the `release` environment of the newest real `Release` run. It refuses a dry run, a run not
 * waiting on `release`, and a run where a job before `publish` did not pass, as `judgeRun` judges it. It
 * prints the notes `plan` rendered before it approves, and refuses when they cannot be read.
 */
async function approveRelease(s: CiSeams): Promise<number> {
  const run = await newestReal(s, "approve");
  if (run === null) return 1;
  if (run.status !== "waiting") {
    s.say(`REFUSED the run is not waiting on the ${RELEASE_ENV} environment: ${runLine(run)}`);
    return 1;
  }
  const pending = await ghJson<{ id: number; name: string }[]>(s, [
    "api",
    `repos/{owner}/{repo}/actions/runs/${run.id}/pending_deployments`,
    "--jq",
    "[.[] | {id: .environment.id, name: .environment.name}]",
  ]);
  const env = pending.find((p) => p.name === RELEASE_ENV);
  if (env === undefined) {
    s.say(`REFUSED the run waits on no ${RELEASE_ENV} environment: ${runLine(run)}`);
    return 1;
  }
  const jobs = await listJobs(s, run.id);
  const optional = optionalLegs(ROOT, await legsAt(s, run.head_sha));
  const v = judgeRun({ run, jobs, stage: "approve", ciRuns: await ciRunsFor(s, run, jobs), optional });
  for (const l of v.lines) s.say(l);
  if (!v.green) return 1;
  const notes = await readNotes(s, run.id);
  if (notes === null) {
    s.say(`REFUSED the run's ${NOTES_ARTIFACT} artifact could not be read, so its notes are not checked`);
    return 1;
  }
  s.say("the notes this release will carry:");
  for (const l of notes.split(/\r?\n/)) s.say(`  | ${l}`);
  const ok = await s.gh([
    "api",
    "-X",
    "POST",
    `repos/{owner}/{repo}/actions/runs/${run.id}/pending_deployments`,
    "-F",
    `environment_ids[]=${env.id}`,
    "-f",
    "state=approved",
    "-f",
    "comment=Approved by bun nv ci --approve-release: every job before publish passed.",
  ]);
  if (ok.code !== 0) {
    s.say(`could not approve the ${RELEASE_ENV} environment: ${firstLine(ok)}`);
    return 1;
  }
  s.say(`approved the ${RELEASE_ENV} environment of ${run.html_url}: watch it with \`bun nv ci --watch --release\``);
  return 0;
}

/** The notes of run `id`, from its `release-notes-preview` artifact, or null. The download is deleted. */
async function readNotes(s: CiSeams, id: number): Promise<string | null> {
  const dir = join(s.root, ".agent-tmp", "ci", `notes-${id}`);
  rmSync(dir, { recursive: true, force: true });
  try {
    const r = await s.gh(["run", "download", String(id), "-n", NOTES_ARTIFACT, "-D", dir]);
    const file = join(dir, NOTES_FILE);
    return r.code === 0 && existsSync(file) ? readFileSync(file, "utf8").trimEnd() : null;
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** The assets of `draftAssets` that release `rel` lacks. */
export function missingAssets(root: string, rel: GhRelease, legs: Leg[] = buildLegs(root)): string[] {
  const version = rel.tag_name.replace(/^v/, "");
  return draftAssets(root, version, legs).filter((a) => !rel.assets.includes(a));
}

/**
 * Publishes the draft the newest real `Release` run made. It refuses unless every job of that run passed,
 * `publish` and `docker` included, and the draft carries every asset `draftAssets` names.
 */
async function publishRelease(s: CiSeams): Promise<number> {
  const run = await newestReal(s, "publish");
  if (run === null) return 1;
  if (!done(run)) {
    s.say(`REFUSED the run has not ended: ${runLine(run)}`);
    return 1;
  }
  const jobs = await listJobs(s, run.id);
  const legs = await legsAt(s, run.head_sha);
  const v = judgeRun({ run, jobs, stage: "real", ciRuns: await ciRunsFor(s, run, jobs), optional: optionalLegs(ROOT, legs) });
  for (const l of v.lines) s.say(l);
  if (!v.green) return 1;
  const rel = await releaseOf(s, run);
  if (rel === null) {
    s.say("REFUSED no release was drafted after the run started");
    return 1;
  }
  if (!rel.draft) {
    s.say(`the release ${rel.tag_name} is published already: ${rel.html_url}`);
    return 0;
  }
  const missing = missingAssets(ROOT, rel, legs);
  if (missing.length > 0) {
    s.say(`REFUSED the draft ${rel.tag_name} lacks ${missing.join(", ")}  ${rel.html_url}`);
    return 1;
  }
  const ok = await s.gh(["release", "edit", rel.tag_name, "--draft=false"]);
  if (ok.code !== 0) {
    s.say(`could not publish ${rel.tag_name}: ${firstLine(ok)}`);
    return 1;
  }
  s.say(`published ${rel.tag_name}: ${rel.html_url}`);
  s.say("its promote run moves the image tags next: read it with `bun nv ci --release-published`");
  return 0;
}

// ---- --release-published ------------------------------------------------------------------------------

/** What `judgePublished` reads, each part fetched by `releasePublished`. */
export interface PublishedFacts {
  release: GhRelease | null;
  /** Whether the tag's commit is on `origin/main`, as GitHub's compare API says. */
  onMain: boolean;
  /** The newest `Promote release images` run for the tag, or null. */
  promote: Run | null;
  /** The HTTP status of the anonymous manifest read, or of the token request when that failed. */
  image: number;
}

/** The step a person takes once, which no API can: GitHub makes a new package private. */
export const VISIBILITY_CLICK = `make the GHCR package ${IMAGE.split("/")[1]} public: Packages → ${IMAGE.split("/")[1]} → Package settings → Change visibility → Public`;

/**
 * Judges the newest release: it is not a draft, its tag's commit is on `origin/main`, its promote run
 * passed, and its version tag pulls from `ghcr.io` with no login. A `401` or `403` on the pull is a
 * private package, and its line names the visibility click as the one step left to the user.
 */
export function judgePublished(f: PublishedFacts): Verdict {
  const lines: string[] = [];
  let green = true;
  const fail = (l: string) => {
    green = false;
    lines.push(l);
  };
  const rel = f.release;
  if (rel === null) {
    fail("RED there is no release: cut one with `bun nv ci --dispatch release`");
    return { green, lines };
  }
  const version = rel.tag_name.replace(/^v/, "");
  lines.push(`release ${rel.tag_name}  ${rel.html_url}`);
  if (rel.draft) fail(`RED ${rel.tag_name} is a draft: publish it with \`bun nv ci --publish-release\``);
  else lines.push(`  ok ${rel.tag_name} is published`);
  if (f.onMain) lines.push(`  ok its tag is on origin/${BRANCH}`);
  else fail(`RED its tag's commit is not on origin/${BRANCH}`);
  const p = f.promote;
  if (p === null) fail(`RED no ${PROMOTE_FILE} run for ${rel.tag_name}: it starts when the release is published`);
  else if (p.status === "waiting") fail(`RED its promote run waits on the ${MARKETPLACE_ENV} environment: a reviewer approves it in the Actions tab  ${p.html_url}`);
  else if (!done(p)) fail(`RED its promote run has not ended: ${runLine(p)}`);
  else if (p.conclusion !== "success") fail(`RED its promote run ended ${p.conclusion}: ${p.html_url}`);
  else lines.push(`  ok its promote run passed  ${p.html_url}`);
  const ref = `ghcr.io/${IMAGE}:${version}`;
  if (f.image === 200) lines.push(`  ok ${ref} pulls with no login`);
  else if (f.image === 401 || f.image === 403) {
    fail(`RED ${ref} does not pull with no login, since the package is private`);
    lines.push(`  the one step left is the user's: ${VISIBILITY_CLICK}`);
  } else fail(`RED ${ref} has no manifest (HTTP ${f.image})`);
  lines.push(green ? `GREEN ${rel.tag_name} is published: ${rel.html_url}` : `RED ${rel.tag_name} is not published whole`);
  return { green, lines };
}

/** The HTTP status of reading `version`'s manifest from `ghcr.io` with an anonymous token. */
async function imageStatus(s: CiSeams, version: string): Promise<number> {
  const t = await s.fetch(`https://ghcr.io/token?scope=repository:${IMAGE}:pull`, {});
  if (t.status !== 200) return t.status;
  const token = (JSON.parse(t.body) as { token?: string }).token ?? "";
  const m = await s.fetch(`https://ghcr.io/v2/${IMAGE}/manifests/${version}`, { Authorization: `Bearer ${token}`, Accept: MANIFEST_TYPES });
  return m.status;
}

/**
 * `--release-published`: 0 when the newest release is published whole, 1 when it is not, 2 when `gh` or
 * the registry cannot answer. It reads only.
 */
async function releasePublished(s: CiSeams): Promise<number> {
  const release = (await listReleases(s)).find((r) => !r.prerelease) ?? null;
  let onMain = false;
  let promote: Run | null = null;
  let image = 0;
  if (release !== null) {
    const sha = await tagCommit(s, release.tag_name);
    const cmp = await ghJson<string>(s, ["api", `repos/{owner}/{repo}/compare/${sha}...${BRANCH}`, "--jq", ".status | tojson"]);
    onMain = cmp === "identical" || cmp === "ahead";
    const promotes = await ghJson<(Run & WorkflowRun)[]>(s, [
      "api",
      `repos/{owner}/{repo}/actions/workflows/${PROMOTE_FILE}/runs?event=release&per_page=20`,
      "--jq",
      "[.workflow_runs[] | {id, name, event, status, conclusion, head_sha, head_branch, html_url, display_title, path, created_at}]",
    ]);
    promote = promotes.filter((r) => r.head_branch === release.tag_name).sort((a, b) => b.id - a.id)[0] ?? null;
    image = await imageStatus(s, release.tag_name.replace(/^v/, ""));
  }
  const v = judgePublished({ release, onMain, promote, image });
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
  const modes = ["--push", "--check", "--watch", "--status", "--rerun", "--dispatch", "--release-status", "--approve-release", "--publish-release", "--release-published"];
  let bad = modes.includes(mode ?? "") ? "" : "name one mode";
  if (bad === "" && mode === "--dispatch") {
    const k = args[1];
    if (k === "full" || k === "dry-run" || k === "release") kind = k;
    else bad = "--dispatch takes `full`, `dry-run` or `release`";
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
    if (mode === "--watch") return await watch(s, maxMin, past, next, release ? [RELEASE] : undefined);
    if (mode === "--status") return await status(s);
    if (mode === "--dispatch" && kind !== undefined) return await dispatch(s, kind);
    if (mode === "--release-status") return await releaseStatus(s);
    if (mode === "--approve-release") return await approveRelease(s);
    if (mode === "--publish-release") return await publishRelease(s);
    if (mode === "--release-published") return await releasePublished(s);
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
