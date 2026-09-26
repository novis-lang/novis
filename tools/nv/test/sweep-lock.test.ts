import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import type { Check } from "../driver/accept.ts";
import { needLine, sharedResources, takeSweepLock } from "../driver/sweep-lock.ts";

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

describe("what takes the sweep lock", () => {
  const http: Check = { id: "http", kind: "exact", stage: 1, file: "examples/http.nvs", want: ["status=200"] };
  const hello: Check = { id: "hello", kind: "exact", stage: 1, file: "examples/hello.nvs", want: ["hello"] };
  const tests: Check = { id: "tests", kind: "cargo-named", stage: 1, args: ["test", "-p", "nvs-host"] };
  const guard: Check = { id: "guard", kind: "cargo-named", stage: 1, args: ["test", "--release", "-p", "nvs-abi-probe", "--test", "perf_guards"] };
  const bench: Check = { id: "bench", kind: "command", stage: 1, argv: ["bun", "nv", "bench", "--check"] };
  const every = () => true;
  const none = () => false;

  test("a fixture of the origin's program holds the origin and takes the lock", () => {
    const need = sharedResources([hello, http, tests], false, none);
    expect([need.origin, need.lock]).toEqual([true, true]);
    expect(need.why).toEqual(["the local origin, for examples/http.nvs"]);
  });

  test("a check that only names the program in its arguments does not hold the origin", () => {
    const grep: Check = { id: "grep", kind: "command", stage: 1, argv: ["git", "grep", "examples/http.nvs"] };
    expect(sharedResources([grep], false, none).lock).toBe(false);
  });

  test("a reached Linux leg takes the lock only when the legs run", () => {
    const need = sharedResources([hello], true, (leg) => leg === "valgrind sweep");
    expect([need.origin, need.lock]).toEqual([false, true]);
    expect(need.why).toEqual(["the Linux legs: valgrind sweep"]);
    expect(sharedResources([hello], false, every).lock).toBe(false);
    expect(sharedResources([hello], true, none).lock).toBe(false);
  });

  test("a perf guard on the release profile takes the lock", () => {
    const need = sharedResources([tests, guard, bench], false, none);
    expect([need.origin, need.lock]).toEqual([false, true]);
    expect(need.why).toEqual(["the cores, for 2 perf guard(s) on the release profile"]);
  });

  test("a sweep that uses none of them takes neither the lock nor the origin", () => {
    const need = sharedResources([hello, tests], true, none);
    expect(need).toEqual({ origin: false, lock: false, why: [] });
    expect(needLine(need)).toBe("no shared resource: sweep lock not taken");
    expect(needLine(sharedResources([], false, none))).toBe("no shared resource: sweep lock not taken");
  });

  test("the line names every resource that took the lock", () => {
    const need = sharedResources([http, guard], true, every);
    expect(needLine(need)).toBe("sweep lock: taken for the local origin, for examples/http.nvs; the Linux legs: wsl leg, valgrind sweep; the cores, for 1 perf guard(s) on the release profile");
  });
});
