import { describe, expect, test } from "bun:test";
import {
  checksSayingOld,
  crlfFiles,
  currentDocs,
  currentRecords,
  docNamingPython,
  goalCopies,
  numberedGoalFiles,
  pastRe,
  pythonCalls,
  pythonToolSteps,
  RENAMED_TOOLS,
  report,
  runsPython,
  saysOldDirective,
  strayPython,
  toolNameRe,
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

describe("nv audit python", () => {
  test("only the bench's Python twins may stay", () => {
    const paths = ["tools/loop.py", "tools/respawn.py", "benches/userland/01-arith-loop.py", "tools/peek.py", "editors/vscode/scripts/mkwoff.py", "tools/nv/main.ts"];
    expect(strayPython(paths)).toEqual(["tools/loop.py", "tools/respawn.py", "tools/peek.py", "editors/vscode/scripts/mkwoff.py"]);
  });

  test("a deleted home is a citation as a link, in backticks or as plain text", () => {
    const re = pastRe(["peek", ...RENAMED_TOOLS]);
    const cited = [
      "The handoff is [docs/agent/handoff.md](docs/agent/handoff.md).",
      "Overwrite `handoff.md` at the end.",
      "the checks in docs/agent/loop-goal.toml",
      "`loop-goal.md` § *Standing decisions*",
      "re-render `../docs/decisions.toml`",
      "`tools/nv/cmd/rules.ts` reads docs/rules/_index.json",
      "edit `tools/data/help-backlog.toml`",
      `under \`docs/agent/goals/${OLD}/\``,
      "docs/agent/goals/12-parses.md",
      "website/scripts/sync-core.mjs",
      "run `npm run sync:core`?",
      "`.loop/goal-green.json` remembers",
      `\`${OLD}.py --run hostile\``,
    ];
    for (const line of cited) expect(docNamingPython("x.md", line, re)).toBe("x.md:1: 1 line(s)");
  });

  test("a live home that only resembles a deleted one is not a citation", () => {
    const re = pastRe(["peek"]);
    const fine = [
      "data/goals/parses.handoff.json",
      "docs/agent/goals/side/restart-free.handoff.md is not written",
      "the playbook bullet a-loop-goal-toml-check-naming-a-test",
      "docs/decisions/0221.md",
      "npm run sync:render",
      "`.loop/accept-green.json` remembers",
      "tools/nv/cmd/rules.ts:_index.json",
      "docs/agent/goals/core-math-1-3.md",
    ];
    for (const line of fine) expect(docNamingPython("x.md", line, re)).toBeNull();
  });

  test("the renamed tools' old names are Python tools too", () => {
    const re = pastRe([...RENAMED_TOOLS]);
    expect(docNamingPython("x.md", "`check-links.py` is the gate\n`guard-read.py` blocks it\n", re)).toBe("x.md:1: 2 line(s)");
    expect(docNamingPython("x.md", "`bun nv links` is the gate\n", re)).toBeNull();
  });

  test("a record under data/ is read unless it is a decision's", () => {
    const paths = [
      "data/chain.json",
      "data/decisions/0221.json",
      "data/goals/webcrypto.json",
      "data/goals/webcrypto.handoff.json",
      "data/goals/tooling-overhaul.json",
      "data/goals/tooling-overhaul.handoff.json",
      "data/goals/side/restart-free.json",
      "data/playbook/tooling/a-bullet.json",
      "docs/agent/coordinator.md",
      "data/proofs/help-backlog.json",
    ];
    expect(currentRecords(paths)).toEqual([
      "data/chain.json",
      "data/goals/webcrypto.json",
      "data/goals/webcrypto.handoff.json",
      "data/goals/tooling-overhaul.json",
      "data/goals/tooling-overhaul.handoff.json",
      "data/goals/side/restart-free.json",
      "data/playbook/tooling/a-bullet.json",
      "data/proofs/help-backlog.json",
    ]);
  });

  test("a tool path or a known tool's bare name is a mention, a bench file or a longer name is not", () => {
    const re = toolNameRe(["peek", "session"]);
    const text = [
      "Read with `python tools/peek.py A.rs:1-9`.",
      "`session.py --wrap` applies it.",
      "`tools/check-links.py` is the gate.",
      "`00-baseline.py` is a bench twin.",
      "`mypeek.py` and `benches/peek.py` are other files.",
      "Read with `bun nv peek`.",
    ].join("\n");
    expect(docNamingPython("docs/x.md", text, re)).toBe("docs/x.md:1: 3 line(s)");
    expect(docNamingPython("docs/y.md", "bun nv session --wrap\n", re)).toBeNull();
  });

  test("history and the prose of the live goal are not read", () => {
    const paths = [
      "AGENTS.md",
      "CHANGELOG.md",
      "docs/decisions/0179.md",
      "docs/agent/goals/webcrypto.md",
      "docs/agent/goals/tooling-overhaul.md",
      "docs/agent/goals/ci-green.md",
      "docs/agent/goals/side/restart-free.md",
      "tools/nv/cmd/audit.ts",
      ".agent-tmp/notes.md",
    ];
    expect(currentDocs(paths, "webcrypto")).toEqual([
      "AGENTS.md",
      "docs/agent/goals/tooling-overhaul.md",
      "docs/agent/goals/ci-green.md",
      "docs/agent/goals/side/restart-free.md",
    ]);
  });
});

describe("nv audit ci", () => {
  test("a workflow line running a Python tool is an offender, a comment or setup line is not", () => {
    const yml = [
      "# Run `python tools/release.py --preview major` on a clone.",
      "      - uses: actions/setup-python@abc # v7",
      "        run: python tools/ci-changes.py",
      "        run: |",
      "          python3 tools/release.py --notes x",
      "        run: bun nv lints --check",
    ].join("\n");
    expect(pythonToolSteps("ci.yml", yml)).toEqual(["ci.yml:3: run: python tools/ci-changes.py", "ci.yml:5: python3 tools/release.py --notes x"]);
  });

  test("a hook line calling Python outside a comment is an offender", () => {
    const hook = ["#!/bin/sh", "# sh and awk, not python, on purpose.", "if ! command -v python >/dev/null; then", "bun nv verify"].join("\n");
    expect(pythonCalls("commit-msg", hook)).toEqual(["commit-msg:3: if ! command -v python >/dev/null; then"]);
    expect(pythonCalls("commit-msg", "#!/bin/sh\n# not python\nawk '{print}'\n")).toEqual([]);
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
