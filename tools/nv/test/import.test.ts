import { afterEach, describe, expect, test } from "bun:test";
import { decisions } from "../import/decisions.ts";
import { citations, readGaps } from "../import/gaps.ts";
import { goals } from "../import/goals.ts";
import { plan } from "../import/plan.ts";
import { playbook } from "../import/playbook.ts";
import { reference } from "../import/reference.ts";
import { rules } from "../import/rules.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

const values = (r: { records: { id: string; value: unknown }[] }) => Object.fromEntries(r.records.map((x) => [x.id, x.value]));

/** A rulebook of one topic and two rules, the first created by 0001 and amended by 0002. */
function rulebook(t: Scratch, indexTitle = "Types"): void {
  t.put("docs/rules/_index.json", JSON.stringify({ topics: [{ topic: "types", title: indexTitle, order: 20 }] }));
  t.put(
    "docs/rules/types.json",
    JSON.stringify({
      topic: "types",
      title: "Types",
      order: 20,
      rules: [
        { id: "types/a", title: "A", status: "shipped", because: ["0001", "0002"], divergesFromPhp: "d" },
        { id: "types/b", title: "B", status: "designed", because: ["0002"], seeAlso: ["types/a"], guardedBy: ["x.rs"] },
      ],
    }),
  );
  t.put("docs/rules/types/a.md", "A.\n");
  t.put("docs/rules/types/b.md", "B.\n");
}

describe("import", () => {
  test("a topic lists its rules by id, and a rule's missing lists are empty", () => {
    tmp = scratch();
    rulebook(tmp);
    const got = rules.read(tmp.root);
    expect(got.unread).toEqual([]);
    expect(values(got)).toEqual({
      types: { title: "Types", order: 20, rules: ["types/a", "types/b"] },
      "types/a": { title: "A", status: "shipped", because: ["0001", "0002"], seeAlso: [], guardedBy: [], divergesFromPhp: "d" },
      "types/b": { title: "B", status: "designed", because: ["0002"], seeAlso: ["types/a"], guardedBy: ["x.rs"] },
    });
  });

  test("a topic file that disagrees with the index is reported", () => {
    tmp = scratch();
    rulebook(tmp, "Kinds");
    expect(rules.read(tmp.root).unread.map((u) => u.reason)).toEqual(['its title "Types" is not the index\'s "Kinds"']);
  });

  test("a decision's fields come from its H1, its bullets and its summary entry", () => {
    tmp = scratch();
    rulebook(tmp);
    const front = (creates: string, modifies: string) => `---\nstatus: accepted\nchanges:\n  creates:${creates}\n  modifies:${modifies}\n---\n`;
    tmp.put("docs/decisions/0001.md", `${front("\n    - types/a", " []")}# ADR 0001 — One\n\n- **Scope:** all of it,\n  and more.\n\n> **In short:** x\n`);
    tmp.put(
      "docs/decisions/0002.md",
      `${front(" []\n    - types/b", "\n    - types/a")}# ADR 0002 — Two\n\n- **Scope:** s\n- **Depends on:** [0001](0001.md)\n\nBody.\n`,
    );
    tmp.put("docs/decisions/0003.md", `${front(" []", " []")}# ADR 0003 — Three\n\n- **Scope:** s\n- **Depends on:** [0001](0001.md) for the rule,\n  [0002](0002.md) for the rest.\n- **Validated by:** a test\n`);
    tmp.put("docs/decisions.toml", `[[entry]]\nadr = '0001'\ngroup = 'types'\ndigest = 'x'\nheadline = 'H'\nbody = '''\nB\n'''\n`);
    const got = decisions.read(tmp.root);
    expect(got.unread).toEqual([]);
    expect(values(got)).toEqual({
      "0001": { title: "One", status: "accepted", scope: "all of it,\nand more.", dependsOn: [], summary: { group: "types", headline: "H", body: "B" } },
      "0002": { title: "Two", status: "accepted", scope: "s", dependsOn: ["0001"] },
      "0003": {
        title: "Three",
        status: "accepted",
        scope: "s",
        dependsOn: ["0001", "0002"],
        dependsOnText: "[0001](0001.md) for the rule,\n[0002](0002.md) for the rest.",
        validatedBy: "a test",
      },
    });
  });

  test("a decision whose changes the rules do not derive is reported", () => {
    tmp = scratch();
    rulebook(tmp);
    tmp.put("docs/decisions/0001.md", `---\nstatus: accepted\nchanges:\n  creates: []\n  modifies: []\n---\n# ADR 0001 — One\n\n- **Scope:** s\n`);
    expect(decisions.read(tmp.root).unread.map((u) => u.reason)).toEqual([
      "`changes.creates` is not what the rules' `because` lists derive",
    ]);
  });

  test("a reference chapter's id is its path, and its front matter's id is its slug", () => {
    tmp = scratch();
    tmp.put("docs/reference/README.md", "# No front matter\n");
    tmp.put("docs/reference/lang/10-programs.md", "---\nid: programs\ntitle: T\nsummary: what: it is\nkeywords: a, <?=, b c\n---\n# P\n");
    tmp.put("docs/reference/core/Str.md", "---\nsummary: s\nkeywords: x\nextra: y\n---\n");
    const got = reference.read(tmp.root);
    expect(values(got)).toEqual({
      "core/Str": { summary: "s", keywords: ["x"] },
      "lang/10-programs": { summary: "what: it is", keywords: ["a", "<?=", "b c"], slug: "programs", title: "T" },
    });
    expect(got.unread).toEqual([{ path: "docs/reference/core/Str.md", reason: "its front matter carries extra, which no record holds" }]);
  });

  test("a bullet unwraps its lead and body, and names the files it anchors", () => {
    tmp = scratch();
    tmp.put("tools/playbook.py", 'SECTIONS = (\n    ("tooling", "Tooling"),\n    ("divergences", "Divergences"),\n)\n');
    tmp.put("tools/x.py", "");
    tmp.put("Cargo.toml", "");
    tmp.put(
      "docs/agent/playbook/tooling/a-trap.md",
      "- **A trap\n  wraps.** Why, in `tools/x.py:12` and\n  `Cargo.toml`, not `tools/gone.py`.\n  [until: gone tools/x.py:needle]\n",
    );
    tmp.put("docs/agent/playbook/tooling/plain.md", "- A bullet with no bold lead.\n  [until: reviewed 2026-09-01]\n");
    tmp.put("docs/agent/playbook/stray/b.md", "- **B.** c [until: reviewed 2026-09-01]\n");
    const got = playbook.read(tmp.root);
    expect(values(got)).toEqual({
      tooling: { title: "Tooling", order: 1 },
      divergences: { title: "Divergences", order: 2 },
      "tooling/a-trap": {
        lead: "A trap wraps.",
        body: "Why, in `tools/x.py:12` and `Cargo.toml`, not `tools/gone.py`.",
        files: ["tools/x.py", "Cargo.toml"],
        until: { kind: "gone", arg: "tools/x.py:needle" },
      },
    });
    expect(got.unread.map((u) => `${u.path}: ${u.reason}`)).toEqual([
      "docs/agent/playbook/stray: is no section in tools/playbook.py's `SECTIONS`",
      "docs/agent/playbook/tooling/plain.md: does not open on `- **`",
    ]);
  });

  test("goal numbers become the chain, and the installed goal is read from loop-goal and the handoff", () => {
    tmp = scratch();
    const prose = (n: number | null, title: string, extra = "") =>
      `---\nmilestone: M1\n${extra}---\n\n# ${n === null ? "Side goal" : `Loop goal ${n}`} — ${title}\n\nBody.\n`;
    tmp.put("docs/agent/goals/1-first.md", prose(1, "First"));
    tmp.put("docs/agent/goals/dossier/2-core-x.md", prose(2, "Core\\X"));
    tmp.put("docs/agent/goals/dossier/2-core-x.toml", "files = []\n[context]\nrules = []\n");
    tmp.put("docs/agent/goals/dossier/2-core-x.handoff.md", "# Handoff\n\n## State\n\nOld.\n\n## Next group\n\n- [ ] x\n");
    tmp.put("docs/agent/goals/3-last.md", prose(3, "Last", "position: last\n"));
    tmp.put("docs/agent/loop-goal.md", prose(2, "Core\\X"));
    tmp.put(
      "docs/agent/loop-goal.toml",
      [
        'files = ["examples/a.nvs"]',
        "[context]",
        'rules = ["types/a"]',
        "[context.stage.2]",
        'shapes = ["A commit message"]',
        "[docker]",
        'compose = "c.yaml"',
        'services = ["pg"]',
        'memoize_on = ["crates/x"]',
        "[[check]]",
        'kind = "exact"',
        'stage = "1 floor"',
        'file = "examples/a.nvs"',
        'want = ["a"]',
        "[[check]]",
        'kind = "command"',
        'stage = "2 the work"',
        'name = "It works!"',
        'argv = ["x"]',
        "min_passing = 2",
        "[[check]]",
        'kind = "command"',
        'stage = "2 the work"',
        'name = "it works"',
        'argv = ["y"]',
      ].join("\n"),
    );
    tmp.put(
      "docs/agent/handoff.md",
      [
        "# Handoff",
        "",
        "**Generated by `python tools/dossier.py --emit-goals` for goal `core-x` —",
        "Core\\X.**",
        "",
        "## State",
        "",
        "Where it stands.",
        "",
        "## Next group",
        "",
        "**Stage 2: the work.** One file set: `a.rs`, `b.rs` and",
        "`c.rs`. More.",
        "",
        "- [ ] **One** —",
        "  wrapped.",
        "- [x] Two",
        "",
        "After.",
        "",
        "## Backlog",
        "",
        "- Later.",
      ].join("\n"),
    );
    tmp.put("docs/agent/goals/side/s.md", prose(null, "Side"));
    const got = goals.read(tmp.root);
    expect(got.unread).toEqual([]);
    const of = (type: string) => Object.fromEntries(got.records.filter((r) => r.type.name === type).map((r) => [r.id, r.value]));
    expect(of("chain")).toEqual({ chain: ["first", "core-x", "last"] });
    const retired = { files: [], context: {}, stages: [], checks: [], env: {} };
    expect(of("goal")).toEqual({
      first: { title: "First", milestone: "M1", ...retired },
      "core-x": {
        title: "Core\\X",
        milestone: "M1",
        files: ["examples/a.nvs"],
        context: { rules: ["types/a"] },
        stages: [
          { number: 1, title: "floor" },
          { number: 2, title: "the work", context: { shapes: ["A commit message"] } },
        ],
        checks: [
          { id: "exact-examples-a-nvs", kind: "exact", stage: 1, file: "examples/a.nvs", want: ["a"] },
          { id: "it-works", kind: "command", stage: 2, name: "It works!", argv: ["x"], minPassing: 2 },
          { id: "it-works-2", kind: "command", stage: 2, name: "it works", argv: ["y"] },
        ],
        env: { docker: { compose: "c.yaml", services: ["pg"], memoizeOn: ["crates/x"] } },
      },
      last: { title: "Last", milestone: "M1", position: "last", ...retired },
    });
    expect(of("handoff")).toEqual({
      "core-x": {
        state: "Where it stands.",
        next: {
          stage: 2,
          title: "the work",
          files: ["a.rs", "b.rs", "c.rs"],
          note: "More.",
          items: [
            { done: false, text: "**One** — wrapped." },
            { done: true, text: "Two" },
          ],
          after: "After.",
        },
        backlog: ["Later."],
      },
    });
    expect(of("side_goal")).toEqual({ s: { title: "Side", milestone: "M1", ...retired } });
  });

  test("a file set that is more than paths stays whole in the note, and a stray goal file is reported", () => {
    tmp = scratch();
    tmp.put("docs/agent/goals/1-a.md", "---\nmilestone: M1\n---\n# Loop goal 1 — A\n");
    tmp.put("docs/agent/goals/1-a.toml", 'files = []\n[context]\n[[check]]\nkind = "exact"\nstage = "floor"\nfile = "x"\n');
    tmp.put("docs/agent/goals/1-a.handoff.md", "# Handoff\n\n## State\n\nS.\n\n## Next group\n\n**Stage 2: b** — one file set per slice: `x/` and the rest.\n");
    tmp.put("docs/agent/goals/3-c.md", "# Loop goal 3 — C\n");
    const got = goals.read(tmp.root);
    expect(got.unread.map((u) => `${u.path}: ${u.reason}`)).toEqual([
      'docs/agent/goals/3-c.md: it is number 3 at chain position 2',
      'docs/agent/goals/1-a.toml: check "x" has stage "floor", not "N title"',
      "docs/agent/goals/3-c.md: it opens on no front matter, so it names no milestone",
    ]);
    const h = got.records.find((r) => r.type.name === "handoff")!.value as { next: unknown };
    expect(got.records.find((r) => r.type.name === "goal")!.value).toMatchObject({ milestone: "M1" });
    expect(h.next).toEqual({ stage: 2, title: "b", files: [], note: "one file set per slice: `x/` and the rest.", items: [] });
  });

  test("a post-parity goal lands in no milestone", () => {
    tmp = scratch();
    tmp.put("docs/agent/goals/1-a.md", "---\nmilestone: post-parity\n---\n# Loop goal 1 — A\n");
    expect(goals.read(tmp.root).records.find((r) => r.type.name === "goal")!.value).toMatchObject({ milestone: null });
  });

  test("the plan's status fields unwrap, and each milestone row is checked against its file's H1", () => {
    tmp = scratch();
    tmp.put(
      "docs/implementation-plan.md",
      [
        "# Plan",
        "",
        "> **Status:** one",
        "> two.",
        ">",
        "> **Open now:** three.",
        "",
        "| Carried by | Milestone | What it builds | Loop-days |",
        "|---|---|---|---|",
        "| done | [M0](plan/m0.md) | Setup (~3 days) | 0.3 |",
        "| goals `a`, `b` | [M4S](plan/m4s.md) | The `Core` API (~5 weeks; more) | ~1.5 |",
        "| backlog 2 | [M9](plan/m9.md) | Extensions (not yet sized) | not estimated |",
        "",
      ].join("\n"),
    );
    tmp.put("docs/plan/m0.md", "# M0 — Setup (~3 days)\n");
    tmp.put("docs/plan/m4s.md", "# M4S — The `Core` API (~5 weeks; more)\n");
    tmp.put("docs/plan/m9.md", "# M9 — Extensions\n");
    const got = plan.read(tmp.root);
    const of = (type: string) => Object.fromEntries(got.records.filter((r) => r.type.name === type).map((r) => [r.id, r.value]));
    expect(of("plan_status")).toEqual({ plan_status: { status: "one two.", openNow: "three." } });
    expect(of("milestone")).toEqual({
      M0: { title: "Setup", order: 1, estimate: "~3 days", loopDays: "0.3", state: "done" },
      M4S: { title: "The `Core` API", order: 2, estimate: "~5 weeks; more", loopDays: "~1.5", state: "open" },
      M9: { title: "Extensions", order: 3, estimate: "not yet sized", loopDays: "not estimated", state: "open", backlog: 2 },
    });
    expect(got.unread).toEqual([
      { path: "docs/plan/m9.md", reason: "its H1 is not the table's `# M9 — Extensions (not yet sized)`" },
    ]);
  });

  test("a gap's slug is its title's first words, its owner a milestone or a goal, and a position is cited", () => {
    tmp = scratch();
    tmp.put(
      "crates/nvs-x/src/lib.rs",
      [
        "//! The crate.",
        "//!",
        "//! # Known gaps",
        "//!",
        "//! Each of these is open.",
        "//!",
        "//! 3. **A walk reads the subject it reshapes.** Every member",
        "//!    borrows it.",
        "//!    — owner: M10",
        "//! 4. **A walk reads the subject twice**, and pays for it.",
        "//!    — owner: arr-walks",
        "//! 5. § 6's entries are not embedded yet. Nothing loads them.",
        "//!    — owner: M9",
        "//! 6. **Nobody owns this.**",
        "//!",
        "//! # Other",
        "fn f() {}",
      ].join("\n"),
    );
    tmp.put("crates/nvs-x/src/one.rs", "//! One.\n//!\n//! **Known gaps.** Two things are open. Both are slow.\n//! — owner: M12\n");
    const got = readGaps(tmp.root);
    expect(values(got)).toEqual({
      "nvs-x/walk-reads-the-subject-it-reshapes": {
        module: "crates/nvs-x/src/lib.rs",
        title: "A walk reads the subject it reshapes.",
        text: "Every member borrows it.",
        milestone: "M10",
      },
      "nvs-x/walk-reads-the-subject-twice": {
        module: "crates/nvs-x/src/lib.rs",
        title: "A walk reads the subject twice",
        text: "and pays for it.",
        goal: "arr-walks",
      },
      "nvs-x/entries-are-not-embedded-yet": {
        module: "crates/nvs-x/src/lib.rs",
        title: "§ 6's entries are not embedded yet.",
        text: "Nothing loads them.",
        milestone: "M9",
      },
      "nvs-x/two-things-are-open": {
        module: "crates/nvs-x/src/one.rs",
        title: "Two things are open.",
        text: "Both are slow.",
        milestone: "M12",
      },
    });
    expect(got.unread.map((u) => `${u.path}: ${u.reason}`)).toEqual([
      "crates/nvs-x/src/lib.rs: line 5: the gap block at line 3 opens on prose no gap holds",
      "crates/nvs-x/src/lib.rs: line 14: gap 6: no owner tag",
    ]);
    const cited = citations(tmp.root, "docs/a.md", "`nvs_x`'s known gap 4 and\n`crates/nvs-x/src/one.rs`'s `# Known gaps` item 2, and known gap 9.", got.positions);
    expect(cited).toEqual([
      { path: "docs/a.md", line: 1, num: 4, module: "crates/nvs-x/src/lib.rs", gap: "nvs-x/walk-reads-the-subject-twice" },
      { path: "docs/a.md", line: 2, num: 2, module: "crates/nvs-x/src/one.rs", gap: null },
      { path: "docs/a.md", line: 2, num: 9, module: "crates/nvs-x/src/one.rs", gap: null },
    ]);
  });
});
