import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { takeSweepLock } from "../driver/sweep-lock.ts";

const DIR = join(ROOT, ".agent-tmp", `sweep-lock-test-${process.pid}`);
mkdirSync(DIR, { recursive: true });
afterAll(() => rmSync(DIR, { recursive: true, force: true }));

/** A pid no process has: the highest a 32-bit pid can be. */
const GONE = 2 ** 31 - 2;

describe("the sweep lock", () => {
  test("a second taker waits until the first gives the lock back", async () => {
    const file = join(DIR, "wait.lock");
    const first = await takeSweepLock({ file, pollMs: 5 });
    expect(existsSync(file)).toBe(true);
    // The holder is this live process, but the lock treats its own pid as stale, so a live stranger is
    // written in its place: the parent of this test process.
    writeFileSync(file, JSON.stringify({ pid: process.ppid, root: "elsewhere", since: "then" }));
    const notes: string[] = [];
    let second = false;
    const taking = takeSweepLock({ file, pollMs: 5, note: (t) => notes.push(t) }).then((lock) => {
      second = true;
      return lock;
    });
    await Bun.sleep(50);
    expect(second).toBe(false);
    expect(notes).toEqual([expect.stringContaining("waiting for the sweep in elsewhere")]);
    first.release(); // names another process now, so it deletes nothing
    expect(existsSync(file)).toBe(true);
    rmSync(file);
    (await taking).release();
    expect(second).toBe(true);
    expect(existsSync(file)).toBe(false);
  });

  test("a lock whose process has exited is taken over", async () => {
    const file = join(DIR, "stale.lock");
    writeFileSync(file, JSON.stringify({ pid: GONE, root: "elsewhere", since: "then" }));
    const lock = await takeSweepLock({ file, pollMs: 5 });
    expect(JSON.parse(await Bun.file(file).text()).pid).toBe(process.pid);
    lock.release();
    expect(existsSync(file)).toBe(false);
  });

  test("a lock that reads as nothing on two passes is taken over", async () => {
    const file = join(DIR, "torn.lock");
    writeFileSync(file, "");
    const lock = await takeSweepLock({ file, pollMs: 5 });
    expect(JSON.parse(await Bun.file(file).text()).pid).toBe(process.pid);
    lock.release();
  });
});
