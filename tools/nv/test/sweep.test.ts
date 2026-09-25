import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { run } from "../lib/proc.ts";
import { ROOT } from "../lib/paths.ts";
import {
  BLIND_WAIT,
  INTERRUPTED,
  LIMIT,
  RateLimit,
  SWEEP_SUBJECT,
  Touched,
  WRITTEN,
  backoff,
  markInterrupted,
  readLimit,
  rememberLimit,
  standingLimit,
  waitOutLimit,
  wallAfter,
} from "../driver/sweep.ts";

const made: string[] = [];
afterAll(() => {
  for (const dir of made) rmSync(dir, { recursive: true, force: true });
});

const scratch = () => {
  mkdirSync(join(ROOT, ".agent-tmp"), { recursive: true });
  const dir = mkdtempSync(join(ROOT, ".agent-tmp", "sweep-"));
  made.push(dir);
  mkdirSync(join(dir, ".loop"), { recursive: true });
  return dir;
};

const git = async (cwd: string, ...args: string[]) => {
  const r = await run(["git", ...args], { cwd });
  if (r.code !== 0) throw new Error(`git ${args.join(" ")}: ${r.stderr}`);
  return r.stdout.trim();
};

/** A repository with one commit, `a.txt`, and `.loop/` ignored the way the real tree ignores it. */
const repo = async () => {
  const dir = scratch();
  await git(dir, "init", "-q");
  await git(dir, "config", "user.email", "loop@example.com");
  await git(dir, "config", "user.name", "loop");
  await git(dir, "config", "core.hooksPath", ".no-hooks");
  writeFileSync(join(dir, ".gitignore"), ".loop/\n");
  writeFileSync(join(dir, "a.txt"), "one\n");
  await git(dir, "add", ".");
  await git(dir, "commit", "-q", "-m", "start");
  return dir;
};

const toolUse = (name: string, input: Record<string, unknown>) => ({
  type: "assistant",
  message: { content: [{ type: "tool_use", name, input }] },
});

describe("Touched", () => {
  test("takes a write tool's target off the stream, skips a read, and adds the tools' ledger", () => {
    const dir = scratch();
    const t = new Touched(dir);
    t.start();
    t.note(toolUse("Write", { file_path: join(dir, "src", "new.ts") }));
    t.note(toolUse("Edit", { file_path: "docs\\b.md" }));
    t.note(toolUse("Read", { file_path: join(dir, "read.md") }));
    writeFileSync(join(dir, WRITTEN), "docs/novis.md\n");
    expect([...t.paths()].sort()).toEqual(["docs/b.md", "docs/novis.md", "src/new.ts"]);
    t.start();
    expect(t.paths().size).toBe(0);
  });
});

describe("markInterrupted", () => {
  test("commits the session's own paths under the wip subject and leaves a person's alone", async () => {
    const dir = await repo();
    const t = new Touched(dir);
    t.start();
    writeFileSync(join(dir, "a.txt"), "two\n");
    writeFileSync(join(dir, "new.txt"), "new\n");
    writeFileSync(join(dir, "theirs.txt"), "a person's\n");
    t.note(toolUse("Edit", { file_path: join(dir, "a.txt") }));
    writeFileSync(join(dir, WRITTEN), "new.txt\n");

    const swept = await markInterrupted(7, "the CLI exited 1", t, dir);
    expect(swept).toEqual({ paths: 2, committed: true, left: 1 });
    expect(await git(dir, "log", "-1", "--format=%s")).toBe(`${SWEEP_SUBJECT} session 0007`);
    expect((await git(dir, "show", "--name-only", "--format=", "HEAD")).split("\n").sort()).toEqual(["a.txt", "new.txt"]);
    expect(await git(dir, "status", "--porcelain")).toBe("?? theirs.txt");
    const cut = JSON.parse(readFileSync(join(dir, INTERRUPTED), "utf8"));
    expect(cut.swept).toBe(true);
    expect(cut.head).toBe(await git(dir, "rev-parse", "HEAD"));
    expect(cut.why).toBe("the CLI exited 1");
    expect(cut.left).toEqual(["?? theirs.txt"]);
  });

  test("with nothing of its own dirty, it commits nothing and clears the interruption", async () => {
    const dir = await repo();
    const t = new Touched(dir);
    t.start();
    writeFileSync(join(dir, INTERRUPTED), "{}\n");
    writeFileSync(join(dir, "theirs.txt"), "a person's\n");
    const before = await git(dir, "rev-parse", "HEAD");
    expect(await markInterrupted(3, "it exited without wrapping", t, dir)).toEqual({ paths: 0, committed: false, left: 1 });
    expect(await git(dir, "rev-parse", "HEAD")).toBe(before);
    expect(existsSync(join(dir, INTERRUPTED))).toBe(false);
  });
});

describe("the usage wall", () => {
  const now = Date.UTC(2026, 8, 24, 12, 0, 0);
  const at = (secs: number) => Math.trunc(now / 1000) + secs;

  test("is a rejection with its reset still ahead, and a passed reset is stale", () => {
    expect(new RateLimit({ status: "rejected", resetsAt: at(600) }).blocked(now)).toBe(true);
    expect(new RateLimit({ status: "rejected", resetsAt: at(-5) }).blocked(now)).toBe(false);
    expect(new RateLimit({ status: "allowed", resetsAt: at(600), overageStatus: "rejected" }).blocked(now)).toBe(false);
    expect(readLimit({ type: "rate_limit_event", rate_limit_info: { status: "rejected", resetsAt: at(60) } })?.blocked(now)).toBe(true);
    expect(readLimit({ type: "assistant" })).toBeNull();
  });

  test("falls back to the result's text only on a non-zero exit, and then reopens BLIND_WAIT from now", () => {
    const result = { type: "result", result: "Claude AI usage limit reached" };
    expect(wallAfter(null, 1, result, now)?.resetsAt).toBe(at(BLIND_WAIT));
    expect(wallAfter(null, 0, result, now)).toBeNull();
    expect(wallAfter(null, 1, { type: "result", result: "boom" }, now)).toBeNull();
  });

  test("a remembered wall stands until it turns over, and the stale file is deleted", () => {
    const dir = scratch();
    rememberLimit(new RateLimit({ status: "rejected", resetsAt: at(600), rateLimitType: "five_hour" }), dir);
    expect(standingLimit(dir, now)?.kind).toBe("five_hour");
    expect(standingLimit(dir, now + 700_000)).toBeNull();
    expect(existsSync(join(dir, LIMIT))).toBe(false);
  });

  test("is slept through inside --max-limit-wait and ends the run past it", async () => {
    const dir = scratch();
    const soon = new RateLimit({ status: "rejected", resetsAt: Math.trunc(Date.now() / 1000) + 600 });
    const slept: number[] = [];
    const said: string[] = [];
    expect(await waitOutLimit(soon, 3600, (l) => said.push(l), dir, async (ms) => void slept.push(ms))).toBe("");
    expect(slept.length).toBe(1);
    expect(slept[0]! / 1000).toBeGreaterThan(600);
    expect(said[0]).toContain("usage wall:");
    expect(existsSync(join(dir, LIMIT))).toBe(false);
    expect(await waitOutLimit(soon, 60, () => {}, dir, async () => {})).toContain("--max-limit-wait");
  });

  test("a failed exit backs off 60s, then 120s, and never past 300s", () => {
    expect([1, 2, 3, 4].map(backoff)).toEqual([60, 120, 240, 300]);
  });
});
