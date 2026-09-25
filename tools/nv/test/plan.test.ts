import { describe, expect, test } from "bun:test";
import { expectedRow, tableRows } from "../cmd/plan.ts";

describe("the milestone table", () => {
  const text = [
    "| Carried by | Milestone | What it builds | Loop-days |",
    "|---|---|---|---|",
    "| done | [M0](plan/m0.md) | Project setup (~3 days) | 0.3 |",
    "| goals `fmt`, `lsp` | [M10](plan/m10.md) | Developer tooling (~14 weeks) | ~8 |",
    "| backlog 3 | [M11](plan/m11.md) | PHP transpiler (~10 weeks) | ~3 |",
  ].join("\n");

  test("each row's four cells are read, and the header and rule are not rows", () => {
    expect(tableRows(text)).toEqual([
      { line: 3, carried: "done", id: "M0", link: "plan/m0.md", builds: "Project setup (~3 days)", loopDays: "0.3" },
      { line: 4, carried: "goals `fmt`, `lsp`", id: "M10", link: "plan/m10.md", builds: "Developer tooling (~14 weeks)", loopDays: "~8" },
      { line: 5, carried: "backlog 3", id: "M11", link: "plan/m11.md", builds: "PHP transpiler (~10 weeks)", loopDays: "~3" },
    ]);
  });

  test("the row a record owes: done, the goals that carry it, or where it stands on its own", () => {
    expect(expectedRow({ id: "M0", title: "Project setup", estimate: "~3 days", loopDays: "0.3", state: "done", carriers: ["old"] }).carried).toBe("done");
    expect(expectedRow({ id: "M10", title: "Developer tooling", estimate: "~14 weeks", loopDays: "~8", state: "open", carriers: ["fmt", "lsp"] })).toEqual({
      carried: "goals `fmt`, `lsp`",
      id: "M10",
      link: "plan/m10.md",
      builds: "Developer tooling (~14 weeks)",
      loopDays: "~8",
    });
    expect(expectedRow({ id: "M7", title: "Server", loopDays: "~2", state: "open", carriers: ["parses"] }).carried).toBe("goal `parses`");
    expect(expectedRow({ id: "M11", title: "PHP", loopDays: "~3", state: "open", backlog: 3, carriers: [] }).carried).toBe("backlog 3");
    expect(expectedRow({ id: "M12", title: "JIT", loopDays: "measurement-bound", state: "ongoing", carriers: [] }).carried).toBe("ongoing");
  });

  test("a drifted cell reads differently from the row its record owes", () => {
    const [row] = tableRows("| goal `fmt` | [M10](plan/m10.md) | Developer tooling (~14 weeks) | ~8 |");
    const want = expectedRow({ id: "M10", title: "Developer tooling", estimate: "~14 weeks", loopDays: "~8", state: "open", carriers: ["fmt", "lsp"] });
    expect(row!.carried).not.toBe(want.carried);
    expect(row!.builds).toBe(want.builds);
  });
});
