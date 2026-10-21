// The hang watch: a tree that waits with no CPU and no output is a hang after `stallMs`, one that spins
// with no output after `spinMs`, and any output, I/O past `IO_STEP` or a process starting or ending
// clears both. A tree whose work runs in WSL or Docker is judged by its output alone. A real run that hangs is killed early with its report, and one that keeps printing is not.

import { rmSync } from "node:fs";
import { join } from "node:path";
import { afterEach, describe, expect, test } from "bun:test";
import { IO_STEP, Judge, longTest, report, worksOutside } from "../lib/hang.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc, type RunOptions } from "../lib/proc.ts";

/** Each report a run here wrote, deleted after its test. */
const written: string[] = [];
afterEach(() => {
  for (const path of written.splice(0)) rmSync(join(ROOT, path), { force: true });
});

async function run(argv: string[], opts: RunOptions) {
  const r = await runProc(argv, opts);
  const path = /^report: (\S+)$/m.exec(r.hung ?? "")?.[1];
  if (path !== undefined) written.push(path);
  return r;
}

const at = (cpuMs: number, ioBytes = 0, procs = 1) => ({ cpuMs, ioBytes, procs });

describe("hang judge", () => {
  test("a tree that waits with no CPU is suspected half way and stalled at the bound", () => {
    const j = new Judge(10_000, 60_000, at(0), 0, 0);
    expect(j.step(at(10), 0, 4_000)).toBe("ok");
    expect(j.step(at(20), 0, 6_000)).toBe("suspect");
    expect(j.step(at(30), 0, 10_000)).toBe("stalled");
  });

  test("output clears a stall", () => {
    const j = new Judge(10_000, 60_000, at(0), 0, 0);
    expect(j.step(at(0), 0, 8_000)).toBe("suspect");
    expect(j.step(at(0), 5, 9_000)).toBe("ok");
    expect(j.step(at(0), 5, 17_000)).toBe("suspect");
  });

  test("a process starting or ending, and I/O past the step, are progress; a small poll is not", () => {
    const j = new Judge(10_000, 60_000, at(0), 0, 0);
    expect(j.step(at(0, 0, 2), 0, 8_000)).toBe("ok");
    expect(j.step(at(0, IO_STEP, 2), 0, 16_000)).toBe("ok");
    expect(j.step(at(0, IO_STEP + 100, 2), 0, 26_000)).toBe("stalled");
  });

  test("a busy tree is not stalled, and is spinning once it shows nothing for the spin bound", () => {
    const j = new Judge(10_000, 60_000, at(0), 0, 0);
    let cpu = 0;
    for (let t = 5_000; t < 30_000; t += 5_000) expect(j.step(at((cpu += 5_000)), 0, t)).toBe("ok");
    expect(j.step(at((cpu += 5_000)), 0, 30_000)).toBe("suspect");
    expect(j.step(at((cpu += 30_000)), 0, 60_000)).toBe("spinning");
  });

  test("a tree that works in WSL or Docker is judged by its output alone", () => {
    expect(worksOutside([{ pid: 1, name: "bun.exe", cpuMs: 0 }, { pid: 2, name: "WSL.EXE", cpuMs: 0 }])).toBe(true);
    expect(worksOutside([{ pid: 1, name: "cargo.exe", cpuMs: 0 }])).toBe(false);
    const j = new Judge(10_000, 60_000, at(0), 0, 0);
    j.outside = true;
    expect(j.step(at(0), 0, 20_000)).toBe("ok");
    expect(j.step(at(0), 0, 30_000)).toBe("suspect");
    expect(j.step(at(0), 0, 60_000)).toBe("silent");
  });

  test("libtest's long-running line names the test, the last one wins", () => {
    const out = "test a::b has been running for over 60 seconds\nrunning\ntest c::d has been running for over 60 seconds\n";
    expect(longTest(out)).toBe("c::d");
    expect(longTest("test a ... ok\n")).toBeNull();
  });

  test("the report names the hang, the path, the long test, each process and the last output", () => {
    const text = report(["t.exe"], "stalled", 92_000, [{ pid: 7, name: "t.exe", cpuMs: 40 }], [{ pid: 7, name: "t.exe", cpuMs: 41 }, { pid: 9, name: "nvs.exe", cpuMs: 3 }], 4_000, "test x::y has been running for over 60 seconds\nlast line", ".agent-tmp/hangs/a.txt");
    expect(text).toStartWith("HANG: `t.exe` waited 92 s");
    expect(text).toContain("report: .agent-tmp/hangs/a.txt");
    expect(text).toContain("libtest's last long-running test: x::y");
    expect(text).toContain("  7 t.exe: 1 ms, 41 ms in all");
    expect(text).toContain("  9 nvs.exe: started since");
    expect(text).toContain("  last line");
  });
});

const watched = process.platform === "win32" || process.platform === "linux";

describe.skipIf(!watched)("hang watch on a real run", () => {
  test("a program that waits forever is killed as hung, long before its time limit", async () => {
    const started = Date.now();
    const r = await run([process.execPath, "-e", "console.log('waiting'); setTimeout(() => {}, 600000)"], { timeoutMs: 120_000, hang: { stallMs: 1_500, everyMs: 250 } });
    expect(Date.now() - started).toBeLessThan(30_000);
    expect(r.timedOut).toBe(true);
    expect(r.code).toBe(124);
    expect(r.hung).toStartWith("HANG: ");
    expect(r.hung).toContain("every process is blocked");
    expect(r.hung).toContain("  waiting");
    expect(r.stderr).toContain(r.hung!);
  });

  test("a program that spins with no output is killed as hung at the spin bound", async () => {
    const r = await run([process.execPath, "-e", "for (;;) {}"], { timeoutMs: 120_000, hang: { spinMs: 2_000, everyMs: 250 } });
    expect(r.hung).toContain("used the CPU");
  });

  test("a program that keeps printing while it waits is left to finish", async () => {
    const r = await run([process.execPath, "-e", "let n = 0; const t = setInterval(() => { console.log(n++); if (n === 12) clearInterval(t); }, 250)"], { hang: { stallMs: 1_500, everyMs: 250 } });
    expect(r.code).toBe(0);
    expect(r.hung).toBeUndefined();
  });

  test("`hang: false` leaves a waiting program to its time limit", async () => {
    const r = await run([process.execPath, "-e", "setTimeout(() => {}, 600000)"], { timeoutMs: 3_000, hang: false });
    expect(r.timedOut).toBe(true);
    expect(r.hung).toBeUndefined();
  });
});
