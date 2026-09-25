import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { blocks, declaration, expiryReport, holds, retire, triage } from "../cmd/playbook.ts";
import { write } from "../lib/store.ts";
import { playbookBullet, playbookSection } from "../schema/playbook.ts";
import { scratch } from "./scratch.ts";

const TODAY = new Date(2026, 8, 24);

describe("nv playbook holds", () => {
  const tmp = scratch();
  tmp.put("tools/a.py", "def needle(): pass\n");

  test("exists and gone read the path and its needle", () => {
    expect(holds(tmp.root, "exists", "tools/a.py", TODAY, new Set())).toEqual([true, "tools/a.py"]);
    expect(holds(tmp.root, "gone", "tools/a.py:needle", TODAY, new Set())).toEqual([false, "tools/a.py holds 'needle'"]);
    expect(holds(tmp.root, "gone", "tools/a.py:other", TODAY, new Set())).toEqual([true, "tools/a.py no longer holds 'other'"]);
    expect(holds(tmp.root, "exists", "tools/b.py", TODAY, new Set())).toEqual([false, "tools/b.py does not exist"]);
    expect(holds(tmp.root, "gone", "tools/*.py", TODAY, new Set())[0]).toBeNull();
  });

  test("a reviewed date is no condition the tree decides", () => {
    expect(holds(tmp.root, "reviewed", "2026-09-20", TODAY, new Set())).toEqual([null, "unknown kind 'reviewed'"]);
    expect(declaration("- a trap [until: reviewed 2026-09-20]")).toBeNull();
  });

  test("test and rule", () => {
    expect(holds(tmp.root, "test", "a_test", TODAY, new Set(["a_test"]))).toEqual([true, "fn a_test exists"]);
    expect(holds(tmp.root, "test", "not a name", TODAY, new Set())[0]).toBeNull();
    expect(holds(tmp.root, "rule", "x/y", TODAY, new Set())).toEqual([false, "docs/rules/x/y.md does not exist"]);
  });
});

describe("nv playbook blocks", () => {
  test("an indented paragraph past a blank line stays in its entry", () => {
    const found = blocks("## Carried\n\n901. first\n\n    more of it [until: gone x]\n\n902. second\n");
    expect(found.map((b) => [b.section, b.start, b.end])).toEqual([
      ["Carried", 2, 4],
      ["Carried", 6, 6],
    ]);
    expect(declaration(found[0]!.body)).toEqual({ kind: "gone", arg: "x" });
    expect(declaration("- a trailer [until: gone tools/x.py:a\nneedle]")).toBeNull();
  });
});

describe("nv playbook triage", () => {
  const tmp = scratch();
  tmp.put("tools/a.py", "def needle_word(): pass\n");
  write(playbookSection, "open", { title: "Open", order: 1 }, tmp.root);
  write(playbookSection, "closed", { title: "Closed", order: 2 }, tmp.root);
  const about = "About `tools/a.py` and its `needle_word`.";
  write(playbookBullet, "open/a-trap", { lead: "A trap.", body: about, files: [], until: { kind: "test", arg: "a_missing_test" } }, tmp.root);
  write(playbookBullet, "open/bare", { lead: "Bare.", body: "Names nothing at all.", files: [], until: { kind: "test", arg: "a_missing_test" } }, tmp.root);
  write(playbookBullet, "closed/done", { lead: "Done.", body: about, files: ["tools/a.py"], until: { kind: "gone", arg: "tools/a.py:needle_word" } }, tmp.root);
  // A generated goal carries no checks, and its manifest still has to find the bullet.
  tmp.put("data/goals/live.json", JSON.stringify({ context: { playbook: ["Open > a trap"] } }));

  test("a section owing decisions gets a proposal, the manifest mark, its near-duplicates and the count", async () => {
    const said: string[] = [];
    expect(await triage("open", tmp.root, (l) => said.push(l))).toBe(0);
    const out = said.join("\n");
    expect(out).toContain("== open/a-trap  [until: test a_missing_test]  OWES A DECISION  LIVE MANIFEST");
    expect(out).toContain("  file   tools/a.py  (not in its files)");
    expect(out).toContain("  propose [until: gone tools/a.py:needle_word]");
    expect(out).toContain("  near   closed/done");
    expect(out).toMatch(/== open\/bare .*OWES A DECISION\n.*\n  NO NAMED WORD IN ITS FILE/);
    expect(said.at(-1)).toBe("triage: open: 2 bullet(s), 2 still owe a decision");
  });

  test("a decided section says so, and an unknown one exits 1", async () => {
    const said: string[] = [];
    expect(await triage("closed", tmp.root, (l) => said.push(l))).toBe(0);
    expect(said.join("\n")).not.toContain("propose");
    expect(said.at(-1)).toBe("triage: closed: every bullet names a file and declares a condition the tree decides");
    expect(await triage("nope", tmp.root, () => {})).toBe(1);
    tmp.cleanup();
  });
});

describe("nv playbook retire", () => {
  test("a retired bullet takes its fragment, its record and the manifest line that named only it", async () => {
    const tmp = scratch();
    write(playbookSection, "tooling", { title: "Tooling", order: 1 }, tmp.root);
    tmp.put("tools/playbook.py", "");
    tmp.put("docs/agent/playbook/tooling/dead.md", "- **A dead trap.** About `tools/playbook.py`. [until: exists tools/playbook.py]\n");
    tmp.put("docs/agent/playbook/tooling/live.md", "- **A live trap.** About `tools/playbook.py`. [until: gone tools/playbook.py]\n");
    write(playbookBullet, "tooling/dead", { lead: "A dead trap.", body: "About `tools/playbook.py`.", files: ["tools/playbook.py"], until: { kind: "exists", arg: "tools/playbook.py" } }, tmp.root);
    tmp.put("docs/agent/guard-name-debt.md", "# Debt\n\n- [ ] one [until: exists tools/playbook.py]\n\n- [ ] two [until: gone tools/playbook.py]\n");
    tmp.put("docs/agent/loop-goal.toml", "[context]\nplaybook = [\n  'Tooling > a live',\n  # the dead one\n  'Tooling > a dead',\n]\n");

    const { expired, bad } = await expiryReport(tmp.root, TODAY);
    expect(bad).toEqual([]);
    expect(expired.map((e) => `${e.file}:${e.line}`)).toEqual(["docs/agent/playbook/tooling/dead.md:1", "docs/agent/guard-name-debt.md:3"]);

    const said: string[] = [];
    const changed = retire(expired, false, tmp.root, (l) => said.push(l));
    expect(changed).toEqual(["docs/agent/playbook/tooling/dead.md", "data/playbook/tooling/dead.json", "docs/agent/guard-name-debt.md", "docs/agent/loop-goal.toml"]);
    expect(existsSync(join(tmp.root, "docs/agent/playbook/tooling/dead.md"))).toBe(false);
    expect(existsSync(join(tmp.root, "data/playbook/tooling/dead.json"))).toBe(false);
    expect(readFileSync(join(tmp.root, "docs/agent/guard-name-debt.md"), "utf8")).toBe("# Debt\n\n- [ ] two [until: gone tools/playbook.py]\n");
    expect(readFileSync(join(tmp.root, "docs/agent/loop-goal.toml"), "utf8")).toBe("[context]\nplaybook = [\n  'Tooling > a live',\n]\n");
    expect(said.at(-1)).toBe("nv playbook: deleted 2 bullet(s) across 2 file(s) and pruned 1 manifest(s) of the selectors that named nothing else.");
    tmp.cleanup();
  });
});
