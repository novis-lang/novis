import { describe, expect, test } from "bun:test";
import {
  checksSayingOld,
  crlfFiles,
  goalCopies,
  numberedGoalFiles,
  report,
  runsPython,
  saysOldDirective,
  toolSayingOld,
} from "../cmd/audit.ts";
import { parseEol } from "../lib/git.ts";

const OLD = ["doss", "ier"].join("");

describe("nv audit goals", () => {
  test("a goal file named by position is an offender, a slug-named one is not", () => {
    const paths = ["docs/agent/goals/12-parses.md", "docs/agent/goals/side/restart-free.md", "docs/agent/goals/parses.md", "data/goals/3-x.json", "docs/decisions/0001.md"];
    expect(numberedGoalFiles(paths)).toEqual(["docs/agent/goals/12-parses.md", "data/goals/3-x.json"]);
  });

  test("the driver's copies, the handoff and a goal's toml or handoff are copies", () => {
    const paths = [
      "docs/agent/loop-goal.md",
      "docs/agent/loop-goal.toml",
      "docs/agent/handoff.md",
      "docs/agent/goals/parses.toml",
      "docs/agent/goals/side/x.handoff.md",
      "docs/agent/goals/parses.md",
      "data/goals/parses.handoff.json",
    ];
    expect(goalCopies(paths)).toEqual(paths.slice(0, 5));
  });
});

describe("nv audit checks", () => {
  const line = (argv: string[], name = "n") => ({ where: "w", name, argv });

  test("a Python interpreter or a .py argument runs Python", () => {
    expect(runsPython(line(["python", "tools/x.py"]))).toBe(true);
    expect(runsPython(line(["C:\\Python312\\python3.exe", "-c", "1"]))).toBe(true);
    expect(runsPython(line(["bun", "nv", "run", "a.py"]))).toBe(true);
    expect(runsPython(line(["bun", "nv", "audit", "goals"]))).toBe(false);
    expect(runsPython(line(["pytest-ish"]))).toBe(false);
  });

  test("the old name in a check's name or argv is an offender", () => {
    const hits = checksSayingOld([line(["bun"], `${OLD}: Core\\Str::length`), line(["python", `tools/${OLD}.py`]), line(["bun"], "proofs: x")]);
    expect(hits).toHaveLength(2);
  });

  test("a directive is a comment line naming the old word with a colon", () => {
    expect(saysOldDirective(`<?nvs\n// ${OLD}: known-gap x\n`)).toBe(true);
    expect(saysOldDirective(`# ${OLD}:x`)).toBe(true);
    expect(saysOldDirective(`// the ${OLD} of a feature\n`)).toBe(false);
  });

  test("a tool says the old name by its path or by a line of its text", () => {
    expect(toolSayingOld(`tools/${OLD}.py`, "")).toBe(`tools/${OLD}.py: its path`);
    expect(toolSayingOld("tools/a.ts", `x\n${OLD.toUpperCase()}\ny ${OLD}\n`)).toBe("tools/a.ts: 2 line(s)");
    expect(toolSayingOld("tools/a.ts", "proofs\n")).toBeNull();
  });
});

describe("nv audit eol", () => {
  test("a crlf or mixed index entry is an offender, and binary is not", () => {
    const out = [
      "i/lf    w/lf    attr/text eol=lf      \ta.md",
      "i/crlf  w/crlf  attr/                 \tb.rs",
      "i/mixed w/mixed attr/                 \tc d.txt",
      "i/-text w/-text attr/                 \te.png",
      "",
    ].join("\0");
    expect(crlfFiles(parseEol(out))).toEqual(["b.rs: crlf", "c d.txt: mixed"]);
  });
});

test("a report prints the pass line, or a count and every offender, and says whether all passed", () => {
  const ok = report([{ pass: "audit: fine", fail: "thing(s) bad", offenders: [] }]);
  expect(ok).toEqual({ lines: ["audit: fine"], ok: true });
  const bad = report([
    { pass: "audit: fine", fail: "thing(s) bad", offenders: ["a", "b"] },
    { pass: "audit: also fine", fail: "x", offenders: [] },
  ]);
  expect(bad).toEqual({ lines: ["audit: 2 thing(s) bad", "  a", "  b", "audit: also fine"], ok: false });
});
