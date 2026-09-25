import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { claudeArgs, driverChanged, driverFiles, launch, loadRun, openingLine, pendingJudge, runName, saveRun } from "../driver/launch.ts";
import { ROOT } from "../lib/paths.ts";

const made: string[] = [];
afterAll(() => {
  for (const dir of made) rmSync(dir, { recursive: true, force: true });
});

const scratch = () => {
  mkdirSync(join(ROOT, ".agent-tmp"), { recursive: true });
  const dir = mkdtempSync(join(ROOT, ".agent-tmp", "launch-"));
  made.push(dir);
  return dir;
};

describe("claudeArgs", () => {
  test("streams both ways, and passes --effort only when it is asked for", () => {
    const plain = claudeArgs({ model: "opus", permissionMode: "bypassPermissions" });
    expect(plain).toEqual(["-p", "--input-format", "stream-json", "--model", "opus", "--permission-mode", "bypassPermissions", "--output-format", "stream-json", "--verbose"]);
    expect(claudeArgs({ model: "opus", permissionMode: "default", effort: "high" }).slice(-2)).toEqual(["--effort", "high"]);
  });
});

describe("openingLine", () => {
  test("is one user message, the prompt first and the pack behind it", () => {
    const line = openingLine("PROMPT", "PACK");
    expect(line.endsWith("\n")).toBe(true);
    expect(line.trimEnd().includes("\n")).toBe(false);
    const e = JSON.parse(line);
    expect(e.type).toBe("user");
    expect(e.message.content[0].text).toBe("PROMPT\nPACK");
    expect(JSON.parse(openingLine("PROMPT", "")).message.content[0].text).toBe("PROMPT");
  });
});

describe("run state", () => {
  test("continues the run the file names, and starts a different one fresh", () => {
    const root = scratch();
    const { state, fresh } = loadRun("20260924-104254-18704", root);
    expect(fresh).toBe(true);
    expect(state).toMatchObject({ run: "20260924-104254-18704", run_id: "20260924-104254", served: 0, index: 0 });
    saveRun({ ...state, served: 78, index: 78, judge: {} }, root);
    const next = loadRun("20260924-104254-18704", root);
    expect(next.fresh).toBe(false);
    expect(next.state).toMatchObject({ served: 78, index: 78, judge: {} });
    expect(loadRun("20260925-090000-1", root).fresh).toBe(true);
  });

  test("a session left to judge is read back, and an empty `judge` names none", () => {
    const base = { run: "r", run_id: "r", served: 1, index: 1, stalls: 0 };
    expect(pendingJudge({ ...base, judge: {} })).toBeNull();
    expect(pendingJudge(base)).toBeNull();
    expect(pendingJudge({ ...base, judge: { index: 7, line: "CONTINUE x", commits: 2, base: "abc" } })).toEqual({ index: 7, line: "CONTINUE x", commits: 2, base: "abc" });
  });

  test("a driver file whose bytes moved since the snapshot is named, and an unmoved one is not", () => {
    const before = driverFiles();
    const self = [...before.keys()].find((p) => p.replace(/\\/g, "/").endsWith("tools/nv/driver/launch.ts"));
    expect(self).toBeDefined();
    expect(driverChanged(before)).toEqual([]);
    const stale = new Map(before).set(self!, "0".repeat(64));
    expect(driverChanged(stale)).toEqual(["tools/nv/driver/launch.ts"]);
  });

  test("a run name is the local time and the process id", () => {
    expect(runName(new Date(2026, 8, 24, 7, 5, 3), 42)).toBe("20260924-070503-42");
  });
});

describe("launch", () => {
  test("sends the opening line, logs every event, and closes stdin on the result", async () => {
    const dir = scratch();
    // A stand-in for `claude`: it echoes the message it read, then waits for stdin to close before exiting.
    const fake = join(dir, "fake.ts");
    writeFileSync(
      fake,
      [
        "const reader = Bun.stdin.stream().getReader();",
        "let text = '';",
        "while (!text.includes('\\n')) { const r = await reader.read(); if (r.done) break; text += new TextDecoder().decode(r.value); }",
        "const got = JSON.parse(text.split('\\n')[0]).message.content[0].text;",
        "console.log(JSON.stringify({ type: 'system', subtype: 'init', session_id: 'abc' }));",
        "console.log(JSON.stringify({ type: 'assistant', message: { id: 'm1', content: [{ type: 'text', text: got }], usage: { input_tokens: 5, output_tokens: 2 } } }));",
        "console.log(JSON.stringify({ type: 'result', subtype: 'success' }));",
        "while (true) { const r = await reader.read(); if (r.done) break; }",
        "process.exit(Bun.env.NOVIS_LOOP_RUN ? 3 : 0);",
      ].join("\n"),
    );
    const log = join(dir, "logs", "run-0001.log");
    const seen: string[] = [];
    const before = process.env.NOVIS_LOOP_RUN;
    process.env.NOVIS_LOOP_RUN = "outer";
    try {
      const got = await launch([process.execPath, fake], { model: "opus", permissionMode: "default" }, openingLine("hello", "pack"), log, { bytes: 4, goal: "g" }, (e) => seen.push(e.type));
      expect(got.code).toBe(0);
      expect(got.sessionId).toBe("abc");
      expect(got.result).toMatchObject({ type: "result" });
    } finally {
      if (before === undefined) delete process.env.NOVIS_LOOP_RUN;
      else process.env.NOVIS_LOOP_RUN = before;
    }
    expect(seen).toEqual(["system", "assistant", "result"]);
    const lines = readFileSync(log, "utf8").trimEnd().split("\n").map((l) => JSON.parse(l));
    expect(lines[0]).toEqual({ type: "loop_pack", bytes: 4, goal: "g" });
    expect(lines[2].message.content[0].text).toBe("hello\npack");
  });
});
