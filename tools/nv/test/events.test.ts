// `lib/events.ts` writes looper's progress events: one JSON line per call to the file `LOOPER_EVENTS`
// names, nothing when it names none, and never a line over looper's 4 KiB bound.

import { afterAll, afterEach, describe, expect, test } from "bun:test";
import { mkdirSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { emit, ev, eventLine, MAX_LINE } from "../lib/events.ts";
import { ROOT } from "../lib/paths.ts";

const dir = join(ROOT, ".agent-tmp", `nv-events-test-${crypto.randomUUID()}`);
mkdirSync(dir, { recursive: true });
const was = process.env.LOOPER_EVENTS;
afterEach(() => {
  if (was === undefined) delete process.env.LOOPER_EVENTS;
  else process.env.LOOPER_EVENTS = was;
});
afterAll(() => rmSync(dir, { recursive: true, force: true }));

const lines = (file: string) =>
  readFileSync(file, "utf8")
    .split("\n")
    .filter((l) => l !== "")
    .map((l) => JSON.parse(l) as Record<string, unknown>);

describe("events", () => {
  test("with LOOPER_EVENTS unset or empty, nothing is written", () => {
    const cwd = process.cwd();
    process.chdir(dir);
    try {
      delete process.env.LOOPER_EVENTS;
      ev.add("step:fmt", { kind: "step" });
      process.env.LOOPER_EVENTS = "";
      ev.done("green");
    } finally {
      process.chdir(cwd);
    }
    expect(readdirSync(dir)).toEqual([]);
  });

  test("each call appends one line that carries the version, the time and the event", () => {
    const file = join(dir, "one.ndjson");
    process.env.LOOPER_EVENTS = file;
    const before = Date.now();
    ev.add("test:nvs-ir lib nvs_ir", { parent: "step:test", name: "nvs-ir lib nvs_ir", kind: "test" });
    ev.planned();
    ev.start("test:nvs-ir lib nvs_ir");
    ev.end("test:nvs-ir lib nvs_ir", { result: "red", why: "exit 101" });
    ev.done("red");
    const got = lines(file);
    expect(got.map((l) => l.event)).toEqual(["add", "planned", "start", "end", "done"]);
    for (const l of got) {
      expect(l.v).toBe(1);
      expect(Number.isInteger(l.t)).toBe(true);
      expect(l.t as number).toBeGreaterThanOrEqual(before);
    }
    expect(got[0]).toMatchObject({ id: "test:nvs-ir lib nvs_ir", parent: "step:test", kind: "test" });
    expect(got[3]).toMatchObject({ result: "red", why: "exit 101" });
  });

  test("a field left out is not written, and a kept item's green time is whole milliseconds", () => {
    const file = join(dir, "kept.ndjson");
    process.env.LOOPER_EVENTS = file;
    ev.add("step:clippy", { kind: "step", kept: true, green_at: 1760600000000.7 });
    ev.add("step:build", { kind: "step", kept: false });
    ev.end("step:build");
    const [kept, run, end] = lines(file);
    expect(kept).toMatchObject({ kept: true, green_at: 1760600000001 });
    expect("kept" in run!).toBe(false);
    expect(Object.keys(end!).sort()).toEqual(["event", "id", "t", "v"]);
  });

  test("a long reason is cut to fit the bound, and a line nothing can cut is not written", () => {
    const file = join(dir, "long.ndjson");
    process.env.LOOPER_EVENTS = file;
    ev.end("case:tests/conformance/a.nvst", { result: "red", why: "é\"".repeat(5000) });
    emit("add", { id: "x".repeat(MAX_LINE) });
    const text = readFileSync(file, "utf8");
    expect(text.split("\n").filter((l) => l !== "")).toHaveLength(1);
    expect(Buffer.byteLength(text.trimEnd())).toBeLessThanOrEqual(MAX_LINE);
    expect(lines(file)[0]).toMatchObject({ id: "case:tests/conformance/a.nvst", result: "red" });
    expect(eventLine("note", { text: "y".repeat(10_000) })!.length).toBeLessThanOrEqual(MAX_LINE);
  });

  test("a write that fails is dropped, and the caller goes on", () => {
    process.env.LOOPER_EVENTS = dir;
    expect(() => ev.planned()).not.toThrow();
  });
});
