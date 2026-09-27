import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { SESSION_TOOLS, StdinGate, claudeArgs, driverChanged, driverFiles, launch, loadRun, openingLine, pendingJudge, runName, saveRun } from "../driver/launch.ts";
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
    expect(plain.slice(0, 10)).toEqual(["-p", "--input-format", "stream-json", "--model", "opus", "--permission-mode", "bypassPermissions", "--output-format", "stream-json", "--verbose"]);
    expect(claudeArgs({ model: "opus", permissionMode: "default", effort: "high" }).slice(-2)).toEqual(["--effort", "high"]);
    expect(plain).not.toContain("--effort");
  });

  test("gives the session only SESSION_TOOLS, no MCP server and no auto-memory", () => {
    const plain = claudeArgs({ model: "opus", permissionMode: "bypassPermissions" });
    expect(plain[plain.indexOf("--tools") + 1]).toBe(SESSION_TOOLS.join(","));
    expect(plain).toContain("--strict-mcp-config");
    expect(JSON.parse(plain[plain.indexOf("--settings") + 1]!)).toEqual({ autoMemoryEnabled: false });
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

// The events the CLI prints around a background task, in the shapes a real session logged them.
const tasks = (...ids: string[]) => ({ type: "system", subtype: "background_tasks_changed", tasks: ids.map((task_id) => ({ task_id, task_type: "local_bash" })) });
const init = { type: "system", subtype: "init", session_id: "s" };
const said = { type: "assistant", message: { content: [{ type: "text", text: "x" }] } };
const result = { type: "result", subtype: "success" };
const notified = (id: string) => ({ type: "system", subtype: "task_notification", task_id: id, status: "completed" });

/** Feeds `events` a second apart, and returns the index of each event `feed` said closes stdin. */
function closes(gate: StdinGate, events: Record<string, any>[]): number[] {
  const out: number[] = [];
  events.forEach((e, i) => {
    if (gate.feed(e, i * 1000)) out.push(i);
  });
  return out;
}

describe("StdinGate", () => {
  test("a result with no background task live closes stdin", () => {
    expect(closes(new StdinGate(), [init, said, result])).toEqual([2]);
  });

  test("a task that ended before the result does not hold stdin open", () => {
    expect(closes(new StdinGate(), [init, tasks("b1"), said, tasks(), notified("b1"), said, result])).toEqual([6]);
  });

  test("a result with a task live waits, and the result of the turn its end starts closes stdin", () => {
    const gate = new StdinGate();
    const events = [init, said, tasks("b1"), said, result, tasks(), notified("b1"), init, said, result];
    expect(closes(gate, events)).toEqual([9]);
  });

  test("a follow-up turn that starts another task keeps waiting", () => {
    const events = [init, tasks("b1"), result, tasks(), init, said, tasks("b2"), result, tasks(), init, said, result];
    expect(closes(new StdinGate(), events)).toEqual([11]);
  });

  test("the wait is over at the cap, counted from the first result that waited", () => {
    const gate = new StdinGate(10_000, 60_000);
    gate.feed(tasks("b1"), 0);
    gate.feed(result, 1000);
    expect(gate.waiting).toBe(true);
    expect(gate.due(10_999)).toBe("");
    expect(gate.due(11_000)).toContain("b1");
  });

  test("a running turn is never cut by the cap", () => {
    const gate = new StdinGate(10_000, 60_000);
    gate.feed(tasks("b1"), 0);
    gate.feed(result, 0);
    gate.feed(tasks(), 5000);
    gate.feed(init, 5000);
    expect(gate.waiting).toBe(false);
    expect(gate.due(60_000)).toBe("");
  });

  test("the wait is over when the tasks ended and no next turn started within the grace", () => {
    const gate = new StdinGate(600_000, 2000);
    gate.feed(tasks("b1"), 0);
    gate.feed(result, 0);
    gate.feed(tasks(), 5000);
    expect(gate.due(6999)).toBe("");
    expect(gate.due(7000)).toContain("no next turn");
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

  // A stand-in for `claude` whose turn ends with a background task live. It then looks at its stdin for
  // `holdMs`, prints `closed_early` if stdin closed meanwhile, and otherwise runs the turn the task's end
  // starts when `followUp` is set. Last it waits for stdin to close and exits.
  const backgroundFake = (dir: string, holdMs: number, followUp: boolean) => {
    const fake = join(dir, "fake.ts");
    const out = (e: object) => `console.log(${JSON.stringify(JSON.stringify(e))});`;
    writeFileSync(
      fake,
      [
        "const reader = Bun.stdin.stream().getReader();",
        "await reader.read();",
        out(init),
        out(tasks("b1")),
        out(result),
        "let next = reader.read();",
        `const early = await Promise.race([next.then((r) => r.done), Bun.sleep(${holdMs}).then(() => false)]);`,
        `if (early) ${out({ type: "closed_early" })}`,
        followUp ? [out(tasks()), out(notified("b1")), out(init), out(said), out({ ...result, num_turns: 2 })].join("\n") : "",
        "while (!(await next).done) next = reader.read();",
        "process.exit(0);",
      ].join("\n"),
    );
    return fake;
  };

  test("keeps stdin open past a result with a background task live, until the turn its end starts", async () => {
    const dir = scratch();
    const fake = backgroundFake(dir, 300, true);
    const seen: string[] = [];
    const got = await launch([process.execPath, fake], { model: "opus", permissionMode: "default" }, openingLine("hi", ""), join(dir, "run-0001.log"), { bytes: 0, goal: "g" }, (e) => seen.push(e.subtype ?? e.type));
    expect(got.code).toBe(0);
    expect(seen).not.toContain("closed_early");
    expect(seen.filter((k) => k === "success")).toHaveLength(2);
    expect(got.result).toMatchObject({ num_turns: 2 });
  });

  test("closes stdin at the cap when no follow-up turn comes", async () => {
    const dir = scratch();
    const fake = backgroundFake(dir, 60_000, false);
    const seen: string[] = [];
    const got = await launch([process.execPath, fake], { model: "opus", permissionMode: "default" }, openingLine("hi", ""), join(dir, "run-0001.log"), { bytes: 0, goal: "g" }, (e) => seen.push(e.subtype ?? e.type), {}, undefined, new StdinGate(1, 60_000));
    expect(got.code).toBe(0);
    expect(seen).toContain("closed_early");
  });
});
