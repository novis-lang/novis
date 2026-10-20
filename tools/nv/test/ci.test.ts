// `bun nv ci` with `gh`, `git` and the clock handed in, so nothing is pushed and nothing is asked of
// GitHub: when `--watch` exits and with which code, what it prints of a failed job, and that `--push`
// pushes and leaves the runs to the push.

import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { type CiSeams, ci, type Job, POLL_BUSY_MS, POLL_IDLE_MS, type Ran, type Run, tail } from "../cmd/ci.ts";
import { ROOT } from "../lib/paths.ts";

const TMP = join(ROOT, ".agent-tmp", `ci-test-${process.pid}`);
afterAll(() => rmSync(TMP, { recursive: true, force: true }));

const ok = (v: unknown): Ran => ({ code: 0, stdout: typeof v === "string" ? v : JSON.stringify(v), stderr: "" });

function runOf(id: number, name: string, status: string, conclusion: string | null, event = "workflow_dispatch", sha = "abc"): Run {
  return { id, name, event, status, conclusion, head_sha: sha, html_url: `https://example.com/runs/${id}` };
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
function seams(name: string, world: (poll: number) => World | "fail", git: (args: string[]) => Ran = () => ok("")) {
  const root = join(TMP, name);
  mkdirSync(root, { recursive: true });
  const said: string[] = [];
  const asked: string[][] = [];
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
    gh: async (args) => {
      asked.push(args);
      const w = world(poll);
      if (w === "fail") return { code: 1, stdout: "", stderr: "HTTP 502" };
      if (args[0] === "api" && args[1]?.includes("/actions/runs?")) return ok(w.runs);
      const jobsOf = args[1]?.match(/actions\/runs\/(\d+)\/jobs/);
      if (jobsOf) return ok(w.jobs[Number(jobsOf[1])] ?? []);
      if (args.includes("--allow-escape-sequences")) return ok("2026-10-20T10:00:00.0000000Z running\nFAIL tests/a.nvst\nsummary\n##[error]Process completed with exit code 1.\n");
      return ok("");
    },
  };
  return { s, said, asked, slept };
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

  test("a cancelled run is passed over, so the run before it decides", async () => {
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

  test("it pushes, and starts or cancels no run, because the push starts its own", async () => {
    const w = seams("push", () => ({ runs: [runOf(40, "CI", "in_progress", null, "push", "old")], jobs: {} }), heads("new", "old"));
    expect(await ci(["--push"], w.s)).toBe(0);
    expect(w.asked).toContainEqual(["git", "push", "origin", "main:main"]);
    expect(w.asked.filter((a) => a[0] !== "git")).toEqual([]);
  });

  test("a refused push exits 1 and starts no run", async () => {
    const w = seams("refused", () => ({ runs: [], jobs: {} }), (args) =>
      args[0] === "push" ? { code: 1, stdout: "", stderr: "! [rejected] main -> main (fetch first)" } : heads("new", "old")(args));
    expect(await ci(["--push"], w.s)).toBe(1);
    expect(w.asked.some((a) => a[0] === "workflow")).toBe(false);
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
  });
});
