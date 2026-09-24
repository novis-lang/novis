import { afterEach, describe, expect, test } from "bun:test";
import { decisions } from "../import/decisions.ts";
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
});
