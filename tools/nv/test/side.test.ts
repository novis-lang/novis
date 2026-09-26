import { describe, expect, test } from "bun:test";
import { landing, sideBranch, sideDir } from "../driver/side.ts";

describe("a side run's landing steps", () => {
  test("name the goal's worktree, its branch and the three files that are deleted", () => {
    const text = landing("s", null);
    expect(sideBranch("s")).toBe("side/s");
    expect(sideDir("s")).toBe(".agent-tmp/worktrees/side/s");
    expect(text).toStartWith("SIDE GOAL GREEN: `s`");
    for (const want of ["git rebase main", "bun nv loop --side s --goal-only", "docs/agent/goals/side/s.md", "data/goals/side/s.json", "data/goals/side/s.handoff.json", "git merge --ff-only side/s", "git worktree remove .agent-tmp/worktrees/side/s", "git branch -d side/s"]) {
      expect(text).toContain(want);
    }
    expect(text).not.toContain("wsl rm");
  });

  test("delete the WSL target and its source mirror when the goal names one", () => {
    expect(landing("s", "/var/tmp/t-side-s")).toContain("wsl rm -rf /var/tmp/t-side-s /var/tmp/t-side-s-src");
  });
});
