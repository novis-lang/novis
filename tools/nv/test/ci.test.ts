// `bun nv ci` with `gh`, `git` and the clock handed in, so nothing is pushed and nothing is asked of
// GitHub: when `--watch` exits and with which code, what it prints of a failed job, that `--push`
// runs its pre-push checks, pushes only when they are green and starts no run, and that both cancel a
// run a newer one replaces. The release half: `--dispatch` starts only a full `ci.yml` run or a dry-run
// Release, `--watch --release` cancels no run, and `--release-status` judges a dry run as `release.yml`.

import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { checkLines, type CiSeams, ci, DISPATCH, DRY_RUN_TITLE, type Job, optionalLegs, POLL_BUSY_MS, POLL_IDLE_MS, PRE_PUSH, type PrePush, PUBLISH_JOB, type Ran, REFUSED, type Run, tail } from "../cmd/ci.ts";
import { ROOT } from "../lib/paths.ts";

const TMP = join(ROOT, ".agent-tmp", `ci-test-${process.pid}`);
afterAll(() => rmSync(TMP, { recursive: true, force: true }));

const ok = (v: unknown): Ran => ({ code: 0, stdout: typeof v === "string" ? v : JSON.stringify(v), stderr: "" });

function runOf(id: number, name: string, status: string, conclusion: string | null, event = "workflow_dispatch", sha = "abc", title = name): Run {
  return { id, name, event, status, conclusion, head_sha: sha, html_url: `https://example.com/runs/${id}`, display_title: title };
}

function jobOf(id: number, name: string, status: string, conclusion: string | null, failedStep = ""): Job {
  const steps = failedStep === "" ? [] : [{ name: "checkout", conclusion: "success" }, { name: failedStep, conclusion: "failure" }];
  return { id, name, status, conclusion, html_url: `https://example.com/jobs/${id}`, steps };
}

/** A world whose runs and jobs `step` sets for each poll, counted from 0. */
interface World {
  runs: Run[];
  jobs: Record<number, Job[]>;
}

/** Seams over a scratch root, whose `gh` answers from `world(poll)`, where a sleep ends a poll. */
function seams(name: string, world: (poll: number) => World | "fail", git: (args: string[]) => Ran = () => ok(""), check: (c: PrePush) => Ran = () => ok("")) {
  const root = join(TMP, name);
  mkdirSync(root, { recursive: true });
  const said: string[] = [];
  const asked: string[][] = [];
  const checked: string[] = [];
  const slept: number[] = [];
  let poll = 0;
  let clock = 0;
  const s: Partial<CiSeams> & { root: string } = {
    root,
    say: (l) => said.push(l),
    now: () => clock,
    sleep: async (ms) => {
      slept.push(ms);
      clock += ms;
      poll++;
    },
    git: async (args) => (asked.push(["git", ...args]), git(args)),
    check: async (c) => (checked.push(c.name), check(c)),
    gh: async (args) => {
      asked.push(args);
      const w = world(poll);
      if (w === "fail") return { code: 1, stdout: "", stderr: "HTTP 502" };
      if (args[0] === "api" && args[1]?.includes("/actions/runs?")) return ok(w.runs);
      const jobsOf = args[1]?.match(/actions\/runs\/(\d+)\/jobs/);
      if (jobsOf) return ok(w.jobs[Number(jobsOf[1])] ?? []);
      if (args[1]?.endsWith("/annotations")) return ok("The job has exceeded the maximum execution time of 1h30m0s\n");
      if (args.includes("--allow-escape-sequences")) return ok("2026-10-20T10:00:00.0000000Z running\nFAIL tests/a.nvst\nsummary\n##[error]Process completed with exit code 1.\n");
      return ok("");
    },
  };
  return { s, said, asked, checked, slept };
}

describe("bun nv ci --watch", () => {
  test("it exits 1 the moment a job of a run in progress fails, and prints its step and log", async () => {
    const w = seams("fails", (p) => ({
      runs: [runOf(10, "CI", "in_progress", null)],
      jobs: { 10: [jobOf(1, "lint", "completed", p === 0 ? null : "failure", p === 0 ? "" : "clippy"), jobOf(2, "test", "in_progress", null)] },
    }));
    expect(await ci(["--watch"], w.s)).toBe(1);
    expect(w.slept).toEqual([POLL_BUSY_MS]);
    expect(w.said).toContain('FAILED CI / lint / step "clippy"  https://example.com/jobs/1');
    expect(w.said).toContain("  | FAIL tests/a.nvst");
    expect(existsSync(join(w.s.root, ".agent-tmp", "ci", "job-1.log"))).toBe(true);
  });

  test("a newest run that already ended red exits 1 at once", async () => {
    const w = seams("red", () => ({ runs: [runOf(11, "CI", "completed", "failure")], jobs: { 11: [jobOf(3, "proofs", "completed", "failure", "gate")] } }));
    expect(await ci(["--watch"], w.s)).toBe(1);
    expect(w.slept).toEqual([]);
  });

  test("`--next` passes over a run that had ended red, and watches the next one", async () => {
    const w = seams("next", (p) => ({
      runs: p < 2 ? [runOf(17, "CI", "completed", "failure")] : [runOf(18, "CI", "in_progress", null), runOf(17, "CI", "completed", "failure")],
      jobs: { 17: [jobOf(11, "proofs", "completed", "failure", "gate")], 18: [jobOf(12, "lint", "completed", "failure", "clippy")] },
    }));
    expect(await ci(["--watch", "--next"], w.s)).toBe(1);
    expect(w.slept).toEqual([POLL_IDLE_MS, POLL_IDLE_MS]);
    expect(w.said.some((l) => l.startsWith('FAILED CI / lint / step "clippy"'))).toBe(true);
    expect(w.said.some((l) => l.startsWith("FAILED CI / proofs"))).toBe(false);
  });

  test("a cancelled run on origin/main's commit is red, and its timed-out job's annotation is printed", async () => {
    const tip = (args: string[]) => ok(args.join(" ") === "rev-parse origin/main" ? "abc\n" : "");
    const w = seams(
      "cancelled-tip",
      () => ({
        runs: [runOf(15, "CI", "completed", "cancelled"), runOf(14, "CI", "completed", "success")],
        jobs: { 15: [jobOf(6, "lint", "completed", "success"), jobOf(7, "proofs", "completed", "cancelled")] },
      }),
      tip,
    );
    expect(await ci(["--watch"], w.s)).toBe(1);
    expect(w.said).toContain('FAILED CI / proofs / step "(no step named)"  https://example.com/jobs/7');
    expect(w.said).toContain("  annotation: The job has exceeded the maximum execution time of 1h30m0s");
  });

  test("a cancelled run on an older commit than origin/main's is passed over, so the run before it decides", async () => {
    const w = seams("cancelled", () => ({
      runs: [runOf(13, "CI", "completed", "cancelled"), runOf(12, "CI", "completed", "failure")],
      jobs: { 12: [jobOf(4, "docs", "completed", "failure", "doc")] },
    }));
    expect(await ci(["--watch"], w.s)).toBe(1);
    expect(w.said.some((l) => l.startsWith('FAILED CI / docs / step "doc"'))).toBe(true);
  });

  test("it exits 0 once every run it watched in progress ended green", async () => {
    const w = seams("green", (p) => ({
      runs: [runOf(20, "CI", p < 2 ? "in_progress" : "completed", p < 2 ? null : "success"), runOf(21, "Website", "completed", "success", "push")],
      jobs: { 20: [jobOf(5, "lint", p < 2 ? "in_progress" : "completed", p < 2 ? null : "success")] },
    }));
    expect(await ci(["--watch"], w.s)).toBe(0);
    expect(w.said.filter((l) => l.startsWith("GREEN "))).toHaveLength(2);
  });

  test("with everything green already it waits for a new run, and exits 3 at `--max-min`", async () => {
    const w = seams("idle", () => ({ runs: [runOf(30, "CI", "completed", "success")], jobs: {} }));
    expect(await ci(["--watch", "--max-min", "3"], w.s)).toBe(3);
    expect(w.slept).toEqual([POLL_IDLE_MS, POLL_IDLE_MS, POLL_IDLE_MS]);
  });

  test("`--past` passes over a failed job it names, and the watch goes on for the others", async () => {
    const w = seams("past", (p) => ({
      runs: [runOf(14, "CI", "in_progress", null)],
      jobs: {
        14: [
          jobOf(6, "proofs / linux-x86_64", "completed", "failure", "Feature proofs"),
          jobOf(7, "test", p === 0 ? "in_progress" : "completed", p === 0 ? null : "failure", p === 0 ? "" : "nextest"),
        ],
      },
    }));
    expect(await ci(["--watch", "--past", "proofs"], w.s)).toBe(1);
    expect(w.slept).toEqual([POLL_BUSY_MS]);
    expect(w.said.filter((l) => l.startsWith("  passing over FAILED CI / proofs / linux-x86_64"))).toHaveLength(1);
    expect(w.said.some((l) => l.startsWith('FAILED CI / test / step "nextest"'))).toBe(true);
    expect(w.said.some((l) => l.startsWith("FAILED CI / proofs"))).toBe(false);
  });

  test("a watched run that ends red only in `--past` jobs exits 4, and an old one is waited past", async () => {
    const w = seams("past-ends", (p) => ({
      runs: [runOf(15, "CI", p < 2 ? "in_progress" : "completed", p < 2 ? null : "failure")],
      jobs: { 15: [jobOf(8, "proofs / linux-x86_64", "completed", "failure", "Feature proofs"), jobOf(9, "test", p < 2 ? "in_progress" : "completed", p < 2 ? null : "success")] },
    }));
    expect(await ci(["--watch", "--past", "proofs"], w.s)).toBe(4);
    const old = seams("past-old", () => ({
      runs: [runOf(16, "CI", "completed", "failure")],
      jobs: { 16: [jobOf(10, "proofs / linux-x86_64", "completed", "failure", "Feature proofs")] },
    }));
    expect(await ci(["--watch", "--past", "proofs", "--max-min", "2"], old.s)).toBe(3);
    expect(old.slept).toEqual([POLL_IDLE_MS, POLL_IDLE_MS]);
  });

  test("`--past` covers each shard of a matrix job, and nothing that only starts with its name", async () => {
    const shards = (id: number) => [
      jobOf(11, "miri units (nvs-stdlib 1/12)", "completed", "failure", "miri nextest"),
      jobOf(12, "miri units (nvs-runtime 1/2)", "completed", "failure", "miri nextest"),
      jobOf(13, "miri", "completed", id === 17 ? "success" : "failure", "miri test"),
    ];
    const w = seams("past-shards", (p) => ({
      runs: [runOf(17, "CI", p < 1 ? "in_progress" : "completed", p < 1 ? null : "failure")],
      jobs: { 17: shards(17) },
    }));
    expect(await ci(["--watch", "--past", "miri units"], w.s)).toBe(4);
    expect(w.said.filter((l) => l.startsWith("  passing over FAILED CI / miri units ("))).toHaveLength(2);
    const other = seams("past-shards-other", () => ({
      runs: [runOf(18, "CI", "completed", "failure")],
      jobs: { 18: shards(18) },
    }));
    expect(await ci(["--watch", "--past", "miri units"], other.s)).toBe(1);
    expect(other.said.some((l) => l.startsWith('FAILED CI / miri / step "miri test"'))).toBe(true);
  });

  test("a run still going beside a newer one of its workflow is cancelled once, and the newer one watched", async () => {
    const w = seams("two", (p) => ({
      runs: [
        runOf(61, "CI", p < 2 ? "in_progress" : "completed", p < 2 ? null : "success", "workflow_dispatch", "new"),
        runOf(60, "CI", p < 1 ? "in_progress" : "completed", p < 1 ? null : "cancelled", "push", "old"),
      ],
      jobs: { 61: [jobOf(11, "lint", p < 2 ? "in_progress" : "completed", p < 2 ? null : "success")] },
    }));
    expect(await ci(["--watch"], w.s)).toBe(0);
    expect(w.asked.filter((a) => a[0] === "run")).toEqual([["run", "cancel", "60"]]);
    expect(w.said.some((l) => l.startsWith("watching CI (workflow_dispatch) on new"))).toBe(true);
  });

  test("a `gh` that keeps failing ends the watch with exit 2, and one failure does not", async () => {
    const w = seams("gh-down", () => "fail");
    expect(await ci(["--watch"], w.s)).toBe(2);
    expect(w.said.filter((l) => l.startsWith("WATCH ERROR"))).toHaveLength(20);
    const once = seams("gh-blip", (p) => (p === 0 ? "fail" : { runs: [runOf(31, "CI", "completed", "failure")], jobs: {} }));
    expect(await ci(["--watch"], once.s)).toBe(1);
  });
});

describe("bun nv ci --push", () => {
  const heads = (local: string, remote: string) => (args: string[]) =>
    args[0] === "rev-parse" ? ok(`${args[1] === "main" ? local : remote}\n`) : ok("");

  test("with origin at main already it pushes nothing", async () => {
    const w = seams("same", () => ({ runs: [], jobs: {} }), heads("abc", "abc"));
    expect(await ci(["--push"], w.s)).toBe(0);
    expect(w.asked.some((a) => a[1] === "push")).toBe(false);
  });

  const runAsked = (w: { asked: string[][] }) => w.asked.filter((a) => a[0] === "run" || a[0] === "workflow");

  test("it pushes, starts no run, and cancels each run still going on an older commit", async () => {
    const w = seams("push", () => ({
      runs: [
        runOf(40, "CI", "in_progress", null, "push", "old"),
        runOf(41, "Website", "queued", null, "push", "old"),
        runOf(39, "CI", "completed", "success", "push", "older"),
      ],
      jobs: {},
    }), heads("new", "old"));
    expect(await ci(["--push"], w.s)).toBe(0);
    expect(w.asked).toContainEqual(["git", "push", "origin", "main:main"]);
    expect(runAsked(w)).toEqual([["run", "cancel", "40"], ["run", "cancel", "41"]]);
  });

  test("the run of the pushed commit is left going, and a second run beside it is cancelled", async () => {
    const w = seams("push-kept", () => ({
      runs: [runOf(51, "CI", "in_progress", null, "workflow_dispatch", "new"), runOf(50, "CI", "in_progress", null, "push", "new")],
      jobs: {},
    }), heads("new", "new"));
    expect(await ci(["--push"], w.s)).toBe(0);
    expect(w.asked.some((a) => a[1] === "push")).toBe(false);
    expect(runAsked(w)).toEqual([["run", "cancel", "50"]]);
  });

  test("a refused push exits 1, and starts or cancels no run", async () => {
    const w = seams("refused", () => ({ runs: [runOf(42, "CI", "in_progress", null, "push", "old")], jobs: {} }), (args) =>
      args[0] === "push" ? { code: 1, stdout: "", stderr: "! [rejected] main -> main (fetch first)" } : heads("new", "old")(args));
    expect(await ci(["--push"], w.s)).toBe(1);
    expect(runAsked(w)).toEqual([]);
  });

  test("with nothing to push it runs no pre-push check", async () => {
    const w = seams("same-unchecked", () => ({ runs: [], jobs: {} }), heads("abc", "abc"));
    expect(await ci(["--push"], w.s)).toBe(0);
    expect(w.checked).toEqual([]);
  });

  test("every pre-push check runs before the push", async () => {
    const w = seams("checked", () => ({ runs: [], jobs: {} }), heads("new", "old"));
    expect(await ci(["--push"], w.s)).toBe(0);
    expect(w.checked).toEqual(PRE_PUSH.map((c) => c.name));
    expect(w.asked).toContainEqual(["git", "push", "origin", "main:main"]);
  });

  test("a red pre-push check refuses the push, and --watch and --status report it until a push goes out", async () => {
    let red = true;
    const w = seams("pre-push-red", () => ({ runs: [], jobs: {} }), heads("new", "old"), (c) =>
      red && c.name === "rustfmt" ? { code: 1, stdout: "Diff in crates/a/src/lib.rs:12:", stderr: "" } : ok(""));
    expect(await ci(["--push"], w.s)).toBe(1);
    expect(w.asked.some((a) => a[1] === "push")).toBe(false);
    expect(w.checked).toEqual(PRE_PUSH.map((c) => c.name));
    const file = join(w.s.root, REFUSED);
    expect(existsSync(file)).toBe(true);
    expect(readFileSync(file, "utf8")).toContain("rustfmt: red (`cargo fmt --all --check` exited 1)\n  Diff in crates/a/src/lib.rs:12:");
    expect(await ci(["--watch"], w.s)).toBe(1);
    expect(await ci(["--status"], w.s)).toBe(1);
    red = false;
    expect(await ci(["--push"], w.s)).toBe(0);
    expect(existsSync(file)).toBe(false);
  });

  test("--check runs the checks, exits 1 on a red one, and neither pushes nor leaves a refused push", async () => {
    const w = seams("check-only", () => ({ runs: [], jobs: {} }), heads("new", "old"), (c) => (c.name === "rules" ? { code: 1, stdout: "a red rule", stderr: "" } : ok("")));
    expect(await ci(["--check"], w.s)).toBe(1);
    expect(w.checked).toEqual(PRE_PUSH.map((c) => c.name));
    expect(w.asked).toEqual([]);
    expect(existsSync(join(w.s.root, REFUSED))).toBe(false);
  });

  test("a long red check keeps its first lines, where the summary is, and its end, without bun's script lines", () => {
    const out = ["$ bun tools/nv/main.ts proofs --gate", "nv proofs gate: 40 of 1516 features owe a proof.", ...Array.from({ length: 40 }, (_, i) => `row ${i}`), 'error: script "nv" exited with code 1'].join("\n");
    const lines = checkLines(out);
    expect(lines[0]).toBe("nv proofs gate: 40 of 1516 features owe a proof.");
    expect(lines).toContain("...");
    expect(lines.at(-1)).toBe("row 39");
    expect(lines.length).toBe(31);
  });

  test("a cargo-deny that is not installed is passed over", async () => {
    const w = seams("no-deny", () => ({ runs: [], jobs: {} }), heads("new", "old"), (c) =>
      c.argv[1] === "deny" ? { code: 101, stdout: "", stderr: "error: no such command: `deny`" } : ok(""));
    expect(await ci(["--push"], w.s)).toBe(0);
    expect(w.said).toContain("pre-push: advisories: cargo-deny is not installed, so CI's supply-chain job is the only check");
  });

  test("the checks after `cargo build` read the debug binary it made", () => {
    const after = PRE_PUSH.slice(PRE_PUSH.findIndex((c) => c.name === "build") + 1);
    expect(after.length).toBeGreaterThan(0);
    // A check that runs examples in folders of their own names the binary by its absolute path.
    const debug = ["target/debug/nvs", join(ROOT, "target/debug/nvs")];
    for (const c of after) expect([...c.argv, ...Object.values(c.env ?? {})].some((a) => debug.some((d) => a.startsWith(d)))).toBe(true);
  });
});

describe("bun nv ci: the log and the arguments", () => {
  test("the tail keeps an early FAIL line above the end of the log", () => {
    const lines = ["FAIL tests/a.nvst", ...Array.from({ length: 100 }, (_, i) => `line ${i}`), "##[error]exit 1", "cleanup"];
    const t = tail(lines);
    expect(t[0]).toBe("FAIL tests/a.nvst");
    expect(t.at(-1)).toBe("##[error]exit 1");
  });

  test("no mode, or a stray argument, is a usage error", async () => {
    const w = seams("usage", () => ({ runs: [], jobs: {} }));
    expect(await ci([], w.s)).toBe(2);
    expect(await ci(["--push", "--max-min", "3"], w.s)).toBe(2);
    expect(await ci(["--check", "--next"], w.s)).toBe(2);
  });
});

/** What GitHub lists at one poll: each workflow file's runs, the 40 newest of all, and each run's jobs. */
interface ReleaseWorld {
  release?: Run[];
  ci?: Run[];
  all?: Run[];
  jobs?: Record<number, Job[]>;
}

/** A run of one workflow file on `main`, as `workflowRuns` lists it. */
const onMain = (r: Run) => ({ ...r, head_branch: "main" });
const dryRun = (id: number, status: string, conclusion: string | null, sha = "tip") =>
  runOf(id, "Release", status, conclusion, "workflow_dispatch", sha, DRY_RUN_TITLE);

/** The job names a dry run shows, every one green and `publish` skipped. */
function dryJobs(over: Record<string, Job> = {}): Job[] {
  const names = ["plan", "verify / lint", "build / build linux-x86_64", "build / build the VS Code extension", PUBLISH_JOB, "docker / container images (slim)"];
  return names.map((n, i) => over[n] ?? jobOf(100 + i, n, "completed", n === PUBLISH_JOB ? "skipped" : "success"));
}

/** Seams whose `gh` answers from `world(poll)` and whose `git` says local `main` is `tip`, level with `origin`. */
function releaseSeams(name: string, world: (poll: number) => ReleaseWorld, git: (args: string[]) => Ran | undefined = () => undefined) {
  const w = seams("release-" + name, () => ({ runs: [], jobs: {} }), (args) => {
    const g = git(args);
    if (g !== undefined) return g;
    if (args[0] === "rev-parse") return ok("tip\n");
    if (args[0] === "rev-list") return ok("0\n");
    return ok("");
  });
  let poll = 0;
  const sleep = w.s.sleep!;
  w.s.sleep = async (ms) => (poll++, sleep(ms));
  w.s.gh = async (args) => {
    w.asked.push(args);
    const now = world(poll);
    const path = args[1] ?? "";
    if (path.includes("actions/workflows/release.yml/runs")) return ok((now.release ?? []).map(onMain));
    if (path.includes("actions/workflows/ci.yml/runs")) return ok((now.ci ?? []).map(onMain));
    if (args[0] === "api" && path.includes("/actions/runs?")) return ok(now.all ?? []);
    const jobsOf = path.match(/actions\/runs\/(\d+)\/jobs/);
    if (jobsOf) return ok(now.jobs?.[Number(jobsOf[1])] ?? []);
    return ok("");
  };
  const started = () => w.asked.filter((a) => a[0] === "workflow");
  const cancels = () => w.asked.filter((a) => a[0] === "run" && a[1] === "cancel");
  return { ...w, started, cancels };
}

describe("bun nv ci: the release", () => {
  test("dispatch dry-run starts release.yml on main with dry_run true, and no argument starts a real release", async () => {
    expect([...DISPATCH["dry-run"].argv]).toEqual(["workflow", "run", "release.yml", "--ref", "main", "-f", "bump=patch", "-f", "dry_run=true"]);
    for (const d of Object.values(DISPATCH)) {
      const argv: readonly string[] = d.argv;
      if (argv.includes("release.yml")) expect(argv).toContain("dry_run=true");
    }
    const w = releaseSeams("dry", (p) => ({ release: p === 0 ? [] : [dryRun(7, "queued", null)] }));
    for (const bad of [["--dispatch"], ["--dispatch", "release"], ["--dispatch", "dry-run", "-f", "dry_run=false"], ["--dispatch", "dry-run", "--real"]]) {
      expect(await ci(bad, w.s)).toBe(2);
    }
    expect(w.started()).toEqual([]);
    expect(await ci(["--dispatch", "dry-run"], w.s)).toBe(0);
    expect(w.started()).toEqual([[...DISPATCH["dry-run"].argv]]);
  });

  test("dispatch full starts ci.yml on main, and refuses while main has commits origin lacks", async () => {
    const w = releaseSeams("full", (p) => ({ ci: p === 0 ? [] : [runOf(8, "CI", "queued", null)] }));
    expect(await ci(["--dispatch", "full"], w.s)).toBe(0);
    expect(w.started()).toEqual([["workflow", "run", "ci.yml", "--ref", "main"]]);
    const ahead = releaseSeams("ahead", () => ({}), (args) => (args[0] === "rev-list" ? ok("2\n") : undefined));
    expect(await ci(["--dispatch", "full"], ahead.s)).toBe(1);
    expect(await ci(["--dispatch", "dry-run"], ahead.s)).toBe(1);
    expect(ahead.started()).toEqual([]);
    expect(ahead.said.some((l) => l.includes("2 commit(s) origin/main lacks"))).toBe(true);
  });

  test("dispatch waits until GitHub lists the new run, and prints its address", async () => {
    // The real run a person started at poll 1 is not the dry run, so the wait goes on to poll 2.
    const real = runOf(6, "Release", "queued", null);
    const w = releaseSeams("wait", (p) => ({ release: [dryRun(5, "completed", "failure"), ...(p >= 1 ? [real] : []), ...(p >= 2 ? [dryRun(9, "queued", null)] : [])] }));
    expect(await ci(["--dispatch", "dry-run"], w.s)).toBe(0);
    expect(w.slept).toEqual([POLL_BUSY_MS, POLL_BUSY_MS]);
    expect(w.said).toContain("started Release (workflow_dispatch) on tip: queued  https://example.com/runs/9");
    const never = releaseSeams("never", () => ({ release: [dryRun(5, "completed", "failure")] }));
    expect(await ci(["--dispatch", "dry-run"], never.s)).toBe(2);
    expect(never.slept.length).toBe(20);
  });

  test("watch release follows the newest Release run job by job, and cancels no run", async () => {
    // Without `--release`, the older Release and CI runs going beside newer ones would be cancelled.
    const w = releaseSeams("watch", (p) => ({
      all: [p === 0 ? dryRun(20, "in_progress", null) : dryRun(20, "completed", "success"), dryRun(19, "in_progress", null, "old"), runOf(18, "CI", "in_progress", null), runOf(17, "CI", "in_progress", null, "push", "old")],
      jobs: { 20: [jobOf(1, "plan", "completed", "success"), jobOf(2, "build / build linux-x86_64", p === 0 ? "in_progress" : "completed", p === 0 ? null : "success")] },
    }));
    expect(await ci(["--watch", "--release"], w.s)).toBe(0);
    expect(w.slept).toEqual([POLL_BUSY_MS]);
    expect(w.said).toContain("watching Release (workflow_dispatch) on tip: in_progress  https://example.com/runs/20");
    expect(w.said).toContain("  Release: 1 of 2 jobs finished green");
    expect(w.said.some((l) => l.startsWith("GREEN Release"))).toBe(true);
    expect(w.said.some((l) => l.includes("runs/18"))).toBe(false);
    expect(w.cancels()).toEqual([]);
  });

  test("release-status is green for a dry run on main whose every job passed and whose publish was skipped", async () => {
    const w = releaseSeams("green", () => ({ release: [dryRun(28, "completed", "success")], jobs: { 28: dryJobs() } }));
    expect(await ci(["--release-status"], w.s)).toBe(0);
    expect(w.said).toContain(`  ok ${PUBLISH_JOB}: skipped, as in every dry run`);
    expect(w.said).toContain("GREEN the dry run on tip passed");
    // It only reads: no fetch, no push, no dispatch and no cancel.
    expect(w.asked.some((a) => a[0] === "git" && (a[1] === "fetch" || a[1] === "push"))).toBe(false);
    expect(w.started()).toEqual([]);
    expect(w.cancels()).toEqual([]);
    const ran = releaseSeams("publish-ran", () => ({ release: [dryRun(28, "completed", "success")], jobs: { 28: dryJobs({ [PUBLISH_JOB]: jobOf(1, PUBLISH_JOB, "completed", "success") }) } }));
    expect(await ci(["--release-status"], ran.s)).toBe(1);
  });

  test("release-status counts a skipped verify job only beside a green full ci.yml run of that commit", async () => {
    const jobs = { 28: dryJobs({ "verify / lint": jobOf(1, "verify", "completed", "skipped") }) };
    const pushOnly = releaseSeams("push-only", () => ({ release: [dryRun(28, "completed", "success")], jobs, ci: [runOf(3, "CI", "completed", "success", "push", "tip")] }));
    expect(await ci(["--release-status"], pushOnly.s)).toBe(1);
    expect(pushOnly.said.some((l) => l.startsWith("RED verify was skipped"))).toBe(true);
    const full = releaseSeams("full-green", () => ({ release: [dryRun(28, "completed", "success")], jobs, ci: [runOf(4, "CI", "completed", "success", "schedule", "tip")] }));
    expect(await ci(["--release-status"], full.s)).toBe(0);
    expect(full.said).toContain("  ok verify: skipped beside the green full ci.yml run https://example.com/runs/4");
  });

  test("release-status is red when a commit after the dry run changes a file outside docs/goals and the playbook", async () => {
    const world = () => ({ release: [dryRun(28, "completed", "success", "old")], jobs: { 28: dryJobs() } });
    const diff = (paths: string, ancestor = true) => (args: string[]) =>
      args[0] === "diff" ? ok(paths) : args[0] === "merge-base" ? { code: ancestor ? 0 : 1, stdout: "", stderr: "" } : undefined;
    const notes = releaseSeams("notes", world, diff("docs/goals/release-run.md\ndocs/agent/playbook/ci.md\n"));
    expect(await ci(["--release-status"], notes.s)).toBe(0);
    const code = releaseSeams("code", world, diff("docs/goals/release-run.md\ntools/nv/cmd/ci.ts\n"));
    expect(await ci(["--release-status"], code.s)).toBe(1);
    expect(code.said.some((l) => l.startsWith("RED 1 file(s)") && l.includes("tools/nv/cmd/ci.ts"))).toBe(true);
    const gone = releaseSeams("not-ancestor", world, diff("", false));
    expect(await ci(["--release-status"], gone.s)).toBe(1);
  });

  test("release-status reads only dry runs, and the newest completed one decides", async () => {
    const w = releaseSeams("newest", () => ({
      release: [dryRun(31, "in_progress", null), runOf(30, "Release", "completed", "failure"), dryRun(28, "completed", "success"), dryRun(27, "completed", "failure")],
      jobs: { 28: dryJobs(), 27: dryJobs({ plan: jobOf(1, "plan", "completed", "failure", "plan") }), 30: [] },
    }));
    expect(await ci(["--release-status"], w.s)).toBe(0);
    expect(w.said).toContain("going, and not judged until it ends: Release (workflow_dispatch) on tip: in_progress  https://example.com/runs/31");
    expect(w.said).toContain("dry run Release (workflow_dispatch) on tip: success  https://example.com/runs/28");
    const none = releaseSeams("none", () => ({ release: [runOf(30, "Release", "completed", "success")] }));
    expect(await ci(["--release-status"], none.s)).toBe(1);
    expect(none.said.some((l) => l.includes(DRY_RUN_TITLE))).toBe(true);
  });

  test("release-status names a failed or cancelled job and its step, and passes over a failed optional build leg", async () => {
    // The optional legs are read from build.yml, so the fixture takes one from it.
    const optional = optionalLegs(ROOT);
    expect(optional.length).toBeGreaterThan(0);
    const leg = optional[0]!;
    const jobs = dryJobs({
      "build / build linux-x86_64": jobOf(1, "build / build linux-x86_64", "completed", "failure", "cargo build"),
      "docker / container images (slim)": jobOf(2, "docker / container images (slim)", "completed", "cancelled"),
    });
    const w = releaseSeams("failed", () => ({ release: [dryRun(28, "completed", "failure")], jobs: { 28: [...jobs, jobOf(3, leg, "completed", "failure", "cargo build")] } }));
    expect(await ci(["--release-status"], w.s)).toBe(1);
    expect(w.said).toContain('FAILED build / build linux-x86_64 / step "cargo build"  https://example.com/jobs/1 (failure)');
    expect(w.said).toContain("FAILED docker / container images (slim)  https://example.com/jobs/2 (cancelled)");
    expect(w.said).toContain(`  WARNING ${leg} / step "cargo build"  https://example.com/jobs/3: an optional leg, so the release ships without it`);
    const onlyOptional = releaseSeams("optional", () => ({ release: [dryRun(28, "completed", "success")], jobs: { 28: [...dryJobs(), jobOf(3, leg, "completed", "failure", "cargo build")] } }));
    expect(await ci(["--release-status"], onlyOptional.s)).toBe(0);
  });
});
