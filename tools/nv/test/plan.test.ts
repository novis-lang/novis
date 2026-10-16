import { describe, expect, test } from "bun:test";
import { expectedRow, tableRows } from "../cmd/plan.ts";

describe("the milestone table", () => {
  const text = [
    "| Carried by | Milestone | What it builds | Loop-days |",
    "|---|---|---|---|",
    "| ongoing | [M12](plan/m12.md) | Optimising JIT tier (ongoing) | measurement-bound |",
    "| goals `fmt`, `lsp` | [M10](plan/m10.md) | Developer tooling (~14 weeks) | ~8 |",
    "| backlog 4 | [M15](plan/m15.md) | Packages (~8 weeks) | ~3 |",
  ].join("\n");

  test("each row's four cells are read, and the header and rule are not rows", () => {
    expect(tableRows(text)).toEqual([
      { line: 3, carried: "ongoing", id: "M12", link: "plan/m12.md", builds: "Optimising JIT tier (ongoing)", loopDays: "measurement-bound" },
      { line: 4, carried: "goals `fmt`, `lsp`", id: "M10", link: "plan/m10.md", builds: "Developer tooling (~14 weeks)", loopDays: "~8" },
      { line: 5, carried: "backlog 4", id: "M15", link: "plan/m15.md", builds: "Packages (~8 weeks)", loopDays: "~3" },
    ]);
  });

  test("the row a record owes: the goals that carry it, or where it stands on its own", () => {
    expect(expectedRow({ id: "M10", title: "Developer tooling", estimate: "~14 weeks", loopDays: "~8", state: "open", carriers: ["fmt", "lsp"] })).toEqual({
      carried: "goals `fmt`, `lsp`",
      id: "M10",
      link: "plan/m10.md",
      builds: "Developer tooling (~14 weeks)",
      loopDays: "~8",
    });
    expect(expectedRow({ id: "M16", title: "Framework", loopDays: "~4", state: "open", carriers: ["parses"] }).carried).toBe("goal `parses`");
    expect(expectedRow({ id: "M15", title: "Packages", loopDays: "~3", state: "open", backlog: 4, carriers: [] }).carried).toBe("backlog 4");
    expect(expectedRow({ id: "M12", title: "JIT", loopDays: "measurement-bound", state: "ongoing", carriers: [] }).carried).toBe("ongoing");
  });

  test("a drifted cell reads differently from the row its record owes", () => {
    const [row] = tableRows("| goal `fmt` | [M10](plan/m10.md) | Developer tooling (~14 weeks) | ~8 |");
    const want = expectedRow({ id: "M10", title: "Developer tooling", estimate: "~14 weeks", loopDays: "~8", state: "open", carriers: ["fmt", "lsp"] });
    expect(row!.carried).not.toBe(want.carried);
    expect(row!.builds).toBe(want.builds);
  });
});
