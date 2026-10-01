import { describe, expect, test } from "bun:test";
import { descendants, type Proc, processTable, screen, startedAt, Tree } from "../driver/proctree.ts";
import { run } from "../lib/proc.ts";

const WIN = process.platform === "win32";

/** A synthetic table row. */
const row = (pid: number, parent: number, started: number | null): Proc => ({ pid, parent, started: started === null ? null : BigInt(started) });

/** Whether `pid` is still the process that started at `started`. */
const alive = (pid: number, started: bigint | null) => started !== null && startedAt(pid) === started;

describe("which processes a tree holds", () => {
  // A boot-time `smss.exe` (1268) exited long ago, and `wininit.exe` still names it as its creator.
  const desktop = [row(600, 1268, 10), row(700, 600, 11), row(800, 700, 12), row(900, 800, 13), row(1000, 900, 14), row(1100, 1000, 15)];

  test("a child handed a dead creator's id holds none of the processes that name that id", () => {
    expect(descendants(1268, 5000n, [...desktop, row(1268, 1100, 5000)])).toEqual([]);
    // The same once the child has exited and is gone from the table.
    expect(descendants(1268, 5000n, desktop)).toEqual([]);
  });

  test("a child's children and their children are found, nearest first", () => {
    const table = [...desktop, row(2000, 1100, 100), row(2001, 2000, 101), row(2002, 2001, 102), row(2003, 2000, 103)];
    expect(descendants(2000, 100n, table)).toEqual([2001, 2003, 2002]);
  });

  test("a process older than its creator, or with no start time, is not found", () => {
    const table = [row(2000, 1, 100), row(2001, 2000, 101), row(3000, 2001, 50), row(3001, 2000, null), row(3002, 3001, 105)];
    expect(descendants(2000, 100n, table)).toEqual([2001]);
  });

  test("an id now held by a process that started at another time holds nothing", () => {
    expect(descendants(2000, 100n, [row(2000, 1, 500), row(2001, 2000, 501)])).toEqual([]);
  });

  test("the screen refuses this process, its ancestors, and processes older than it", () => {
    const table = [row(8, 1, 1), row(9, 8, 2), row(10, 9, 100), row(20, 1, 50), row(21, 1, null), row(22, 10, 150)];
    const { kill, refused } = screen([10, 9, 8, 20, 21, 22], table, 10, 100n);
    expect(kill).toEqual([22]);
    expect(refused.map(([pid]) => pid)).toEqual([10, 9, 8, 20, 21]);
    expect(screen([22], table, 10, null).kill).toEqual([]);
  });
});

describe("a live tree", () => {
  test("the process table holds this process with its start time", () => {
    const me = processTable().find((p) => p.pid === process.pid);
    expect(me).toBeDefined();
    expect(me!.started).toBe(startedAt(process.pid));
  });

  test("a kill ends the child and what it started, and nothing else", async () => {
    const argv = WIN ? ["cmd", "/c", "ping -n 60 127.0.0.1 > NUL"] : ["sh", "-c", "sleep 60 & wait"];
    const child = Bun.spawn(argv, { stdout: "ignore", stderr: "ignore" });
    const tree = new Tree(child.pid, child);
    const rootStarted = startedAt(child.pid);
    const below = () => (WIN ? tree.pids().filter((p) => p !== child.pid) : descendants(child.pid, rootStarted!, processTable()));
    let held: number[] = [];
    for (let i = 0; i < 50 && held.length === 0; i++) {
      await Bun.sleep(100);
      held = below();
    }
    expect(held.length).toBeGreaterThan(0);
    const starts = held.map((p) => [p, startedAt(p)] as const);
    tree.kill();
    await child.exited;
    await Bun.sleep(200);
    tree.close();
    expect(starts.filter(([p, s]) => alive(p, s))).toEqual([]);
    expect(() => process.kill(process.pid, 0)).not.toThrow();
    expect(() => process.kill(process.ppid, 0)).not.toThrow();
  }, 20_000);

  test.if(WIN)("a reap ends a grandchild that outlived the child, and nothing else", async () => {
    const child = Bun.spawn(["cmd", "/c", "start /b ping -n 60 127.0.0.1 > NUL"], { stdout: "ignore", stderr: "ignore" });
    const tree = new Tree(child.pid, child);
    expect(await child.exited).toBe(0);
    const left = tree.pids().filter((p) => p !== child.pid);
    expect(left.length).toBeGreaterThan(0);
    const starts = left.map((p) => [p, startedAt(p)] as const);
    expect(tree.reap()).toBe(left.length);
    await Bun.sleep(200);
    tree.close();
    expect(starts.filter(([p, s]) => alive(p, s))).toEqual([]);
    expect(() => process.kill(process.pid, 0)).not.toThrow();
    expect(() => process.kill(process.ppid, 0)).not.toThrow();
  }, 20_000);

  test("a run past its timeout kills what its program started", async () => {
    const argv = WIN ? ["cmd", "/c", "ping -n 60 127.0.0.1 > NUL"] : ["sh", "-c", "sleep 60 & wait"];
    const started = performance.now();
    const r = await run(argv, { timeoutMs: 1_000 });
    expect(r.timedOut).toBe(true);
    expect(performance.now() - started).toBeLessThan(30_000);
  });
});
