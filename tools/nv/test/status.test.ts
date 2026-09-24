import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { type Plan, type Results, Session, cut, goalTable, keyRow, memoResults, statusRow } from "../driver/status.ts";

const plan: Plan = {
  slug: "demo",
  stages: [
    { number: 1, title: "floor", summary: "Every check of every walked goal still passes." },
    { number: 2, title: "the parser", summary: "The parser reads every file in the corpus." },
    { number: 3, title: "the driver" },
  ],
  checks: [
    { id: "floor-a", stage: 1, argv: ["cargo", "test"] },
    { id: "floor-b", stage: 1 },
    { id: "parse-a", stage: 2, argv: ["bun", "test", "tools/nv/test/parse.test.ts"] },
    { id: "parse-b", stage: 2 },
    { id: "drive-a", stage: 3, argv: ["bun", "nv", "loop", "--list"] },
    { id: "drive-b", stage: 3 },
  ],
};

/** A recorded session stream: two turns, a subagent's turn, a check run by its argv, and a file edit. */
const stream: Record<string, any>[] = [
  { type: "system", subtype: "init" },
  {
    type: "assistant",
    message: {
      id: "m1",
      usage: { input_tokens: 10, cache_creation_input_tokens: 2000, cache_read_input_tokens: 18000, output_tokens: 300 },
      content: [{ type: "tool_use", id: "t1", name: "Bash", input: { command: 'bun test "tools/nv/test/parse.test.ts"', description: "Run the parser tests" } }],
    },
  },
  // The same message again, for its second content block, repeating its usage.
  { type: "assistant", message: { id: "m1", usage: { output_tokens: 300 }, content: [{ type: "text", text: "ok" }] } },
  { type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t1", is_error: false, content: " 3 pass" }] } },
  {
    type: "assistant",
    parent_tool_use_id: "t9",
    message: { id: "sub", usage: { input_tokens: 999999, output_tokens: 99999 }, content: [{ type: "tool_use", id: "s1", name: "Grep", input: {} }] },
  },
  {
    type: "assistant",
    message: {
      id: "m2",
      usage: { input_tokens: 5, cache_creation_input_tokens: 100, cache_read_input_tokens: 84095, output_tokens: 5800 },
      content: [
        { type: "tool_use", id: "t2", name: "PowerShell", input: { command: "bun nv loop --list", description: "List the plan" } },
        { type: "tool_use", id: "t3", name: "Edit", input: { file_path: join(ROOT, "tools", "nv", "cmd", "loop.ts") } },
      ],
    },
  },
  { type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t2", is_error: true, content: "exit 1" }] } },
];

function replay(): { results: Results; session: Session } {
  const results: Results = new Map([
    ["floor-a", true],
    ["floor-b", true],
    ["drive-a", true],
  ]);
  const session = new Session(plan, results);
  session.begin(14);
  for (const e of stream) session.feed(e);
  return { results, session };
}

describe("the status row", () => {
  test("every field comes from the plan, the results and the stream", () => {
    const { results, session } = replay();
    // parse-a went green by its argv, drive-a went red by its argv, and the subagent's turn is skipped.
    expect(results.get("parse-a")).toBe(true);
    expect(results.get("drive-a")).toBe(false);
    expect(session.context).toBe(84200);
    expect(session.out).toBe(6100);
    expect(session.calls).toBe(3);
    expect(statusRow(plan, results, session, 200)).toBe("goal demo/2 | 25% | 84.2kin/6.1kout | 3 tool calls | session 14 | Edit tools/nv/cmd/loop.ts");
  });

  test("the floor is left out of the percentage, and counts toward the stage", () => {
    const results: Results = new Map(plan.checks.filter((c) => c.stage !== 1).map((c) => [c.id, true]));
    const session = new Session(plan, results);
    session.begin(1);
    expect(statusRow(plan, results, session, 200)).toStartWith("goal demo/1 | 100% | 0in/0out | 0 tool calls | session 1 | ");
    for (const c of plan.checks) results.set(c.id, true);
    expect(statusRow(plan, results, session, 200)).toStartWith("goal demo/done | 100% |");
  });

  test("a shell call shows its description, and between sessions the phase shows with the last figures", () => {
    const { results, session } = replay();
    session.feed({ type: "assistant", message: { id: "m3", content: [{ type: "tool_use", id: "t4", name: "Bash", input: { command: "ls", description: "List files" } }] } });
    expect(statusRow(plan, results, session, 200)).toEndWith("| 4 tool calls | session 14 | List files");
    session.feed({ type: "assistant", message: { id: "m4", content: [{ type: "tool_use", id: "t5", name: "Glob", input: { pattern: "*" } }] } });
    expect(statusRow(plan, results, session, 200)).toEndWith("| Glob");
    session.phase("acceptance sweep 31/58");
    expect(statusRow(plan, results, session, 200)).toBe("goal demo/2 | 25% | 84.2kin/6.1kout | 5 tool calls | session 14 | acceptance sweep 31/58");
  });

  test("a row wider than the terminal is cut at its end", () => {
    const { results, session } = replay();
    const row = statusRow(plan, results, session, 30);
    expect(row).toBe("goal demo/2 | 25% | 84.2kin/6…");
    expect(Array.from(row).length).toBe(30);
    expect(cut("short", 30)).toBe("short");
  });

  test("a typed prompt ends the key row, shown by its tail", () => {
    expect(keyRow(["[h] halt now", "[i] type a prompt"], null, 80)).toBe("  [h] halt now · [i] type a prompt");
    expect(keyRow(["[h] halt now"], null, 80, false)).toBe("  [h] halt now");
    const row = keyRow([], "run the parser tests", 80);
    expect(row).toBe("  Enter sends, Esc cancels · prompt> run the parser tests");
    expect(row).toEndWith("tests");
    expect(keyRow([], "0123456789", 42)).toBe("  Enter sends, Esc cancels · prompt> 56789");
  });
});

describe("the goal table", () => {
  const table = (width: number, utf8 = true, commits = ["docs(agent): a", "feat(loop): the row"]) => {
    const { results, session } = replay();
    return goalTable({ plan, results, session, position: 3, total: 9, commits, width, utf8 });
  };

  test("one row per stage with its state and its green checks of all", () => {
    const t = table(120);
    expect(t[0]).toBe("goal demo · 3 of 9 · 25% · session 14");
    expect(t[1]).toBe(" #  stage       state  checks  what it does");
    expect(t[2]).toMatch(/^─+$/);
    expect(t[3]).toBe(" 1  floor       done      2/2  Every check of every walked goal still passes.");
    expect(t[4]).toBe(" 2  the parser  now       1/2  The parser reads every file in the corpus.");
    expect(t[5]).toBe(" 3  the driver  ahead     0/2  the driver");
    expect(t[6]).toBe("now: Edit tools/nv/cmd/loop.ts · this session: 2 commits, the last `feat(loop): the row`");
  });

  test("the Python driver's memo sets a check green by its name, its fixture or its kind", () => {
    const checks = [
      { id: "a", stage: 1, kind: "command", name: "A works" },
      { id: "b", stage: 1, kind: "exact", file: "examples/b.nvs" },
      { id: "c", stage: 2, kind: "min-bytes" },
      { id: "d", stage: 2, kind: "command", name: "D works" },
    ];
    const green = { "A works #0a1b2c3d4e5f": "h", "examples/b.nvs #ffffffffffff": "h", "min-bytes #000000000000": "h", "valgrind D works #123456789abc": "h" };
    expect([...memoResults(green, checks)]).toEqual([
      ["a", true],
      ["b", true],
      ["c", true],
    ]);
  });

  test("the sentence wraps inside its column, and ASCII replaces the box-drawing lines", () => {
    const t = table(50, false, []);
    expect(t[0]).toBe("goal demo - 3 of 9 - 25% - session 14");
    expect(t[2]).toMatch(/^-+$/);
    for (const line of t) expect(Array.from(line).length).toBeLessThanOrEqual(50);
    expect(t[3]).toBe(" 1  floor       done      2/2  Every check of");
    expect(t[4]).toBe("                               every walked goal");
    expect(t[5]).toBe("                               still passes.");
    expect(t.at(-1)).toBe("now: Edit tools/nv/cmd/loop.ts - this session: no…");
  });
});
