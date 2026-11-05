// How the proofs runner judges a stop at a CPU or wall-time limit: by the sweep lines the run wrote,
// never by a fixed bound (`explainedMs` and `timeOvershoot` in `tools/nv/proofs/run.ts`). Each case
// gives the lines and the reading, so it holds at any speed.
import { describe, expect, test } from "bun:test";
import { explainedMs, type SweepLine, type TimeReading, timeOvershoot } from "../proofs/run.ts";

const cpu = (lateUs: number, threads = 1): SweepLine => ({ kind: "cpu", lateUs, intervalUs: 50_000, pollUs: 50_000, threads });
const reading = (used: number, sweeps: SweepLine[], kind: "CPU" | "wall" = "CPU"): TimeReading => ({ kind, used, limit: 200, crossed: 200, sweeps });

describe("a time stop is judged by what its run measured", () => {
  test("an overshoot within the interval, the lateness and one poll is green", () => {
    // 50 ms of interval, 12 ms the sweep woke late and 50 ms of poll explain 112 ms.
    expect(explainedMs(reading(312, [cpu(12_000)]))).toBe(112);
    expect(timeOvershoot(reading(312, [cpu(12_000)]))).toBeNull();
    // Every sweep that stopped the tree adds how late it woke.
    expect(timeOvershoot(reading(330, [cpu(12_000), cpu(18_000)]))).toBeNull();
    // A tree on two threads spends twice the CPU time in the same span.
    expect(timeOvershoot(reading(400, [cpu(0, 2)]))).toBeNull();
    // A wall-time claim's lateness counts from the deadline itself.
    const wall: SweepLine = { kind: "wall", lateUs: 70_000, intervalUs: 0, pollUs: 50_000, threads: 1 };
    expect(timeOvershoot(reading(320, [wall], "wall"))).toBeNull();
  });

  test("a CPU clock that moves in ticks adds one tick per thread", () => {
    // Windows reads a thread's CPU time at the clock interrupt, 15.625 ms apart, so a sweep may see
    // the crossing one tick late on each thread.
    const ticked: SweepLine = { ...cpu(12_000), tickUs: 15_625 };
    expect(explainedMs(reading(327, [ticked]))).toBeCloseTo(127.625);
    expect(timeOvershoot(reading(327, [ticked]))).toBeNull();
    expect(timeOvershoot(reading(328, [ticked]))).toContain("128 ms over");
    expect(explainedMs(reading(400, [{ ...ticked, threads: 2 }]))).toBeCloseTo(255.25);
  });

  test("an overshoot past the interval, the lateness and one poll is red", () => {
    const why = timeOvershoot(reading(313, [cpu(12_000)]));
    expect(why).toContain("used 313 ms against a CPU-time limit of 200 ms, 113 ms over");
    expect(why).toContain("explain 112.0 ms");
    // A run that wrote no line has nothing to explain any overshoot with.
    expect(timeOvershoot(reading(201, []))).toContain("no sweep line");
    // A CPU line does not explain a wall-time stop.
    expect(timeOvershoot(reading(300, [cpu(500_000)], "wall"))).not.toBeNull();
  });

  test("a sweep that computed a long wait explains no overshoot", () => {
    // The watchdog waited 400 ms because its code computed that wait, and woke on time. Its lateness
    // is measured against the wait it computed, so it is zero, and the bound stays one interval and one
    // poll: the 400 ms stop is red.
    const onTime = cpu(0);
    expect(explainedMs(reading(600, [onTime]))).toBe(100);
    expect(timeOvershoot(reading(600, [onTime]))).toContain("400 ms over");
  });
});
