import { describe, expect, test } from "bun:test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { CARRIED, carryLine, mirrorPath, q, targetLine, wslBuildLine, wslPath } from "../accept/mirror.ts";
import { ROOT } from "../lib/paths.ts";

describe("the WSL copy's helpers", () => {
  test("wslPath maps a Windows drive path to its mount", () => {
    expect(wslPath("D:\\mwl")).toBe("/mnt/d/mwl");
    expect(wslPath("C:/Users/x/repo/")).toBe("/mnt/c/Users/x/repo");
  });

  test("wslBuildLine is quiet unless asked for cargo's progress lines", () => {
    expect(wslBuildLine("/r", "/t")).toBe("cd /r && CARGO_TARGET_DIR=/t cargo build --quiet");
    expect(wslBuildLine("/r", "/t", true)).toBe("cd /r && CARGO_TARGET_DIR=/t cargo build");
  });

  test("mirrorPath gives each checkout its own copy beside the target directory", () => {
    expect(mirrorPath("/var/tmp/nvs-target-wsl", "/mnt/d/mwl")).toBe("/var/tmp/nvs-target-wsl-src/mnt-d-mwl");
    expect(mirrorPath("/var/tmp/nvs-target-wsl", "/mnt/d/mwl/.agent-tmp/worktrees/x")).toBe(
      "/var/tmp/nvs-target-wsl-src/mnt-d-mwl-.agent-tmp-worktrees-x",
    );
  });

  test("carryLine copies an ignored file the configuration names, and only when the checkout has it", () => {
    const root = join(ROOT, ".agent-tmp", "mirror-test-carry");
    try {
      mkdirSync(join(root, "tests", "db"), { recursive: true });
      expect(carryLine(root)).toBe("");
      writeFileSync(join(root, "tests", "db", "ca.crt"), "x");
      expect(carryLine(root)).toBe(
        ` && mkdir -p tests/db && cp ${wslPath(root)}/tests/db/ca.crt tests/db/ca.crt`,
      );
      expect(CARRIED).toContain("tests/db/ca.crt");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("targetLine links the copy's target to the target directory the copy sits beside", () => {
    const mirror = mirrorPath("/var/tmp/nvs-target-wsl", "/mnt/d/mwl");
    expect(targetLine(mirror)).toBe(" && ln -sfn /var/tmp/nvs-target-wsl target");
  });

  test("q quotes a word only when bash would split or expand it", () => {
    expect(q("examples/a.nvs")).toBe("examples/a.nvs");
    expect(q("--flag=1")).toBe("--flag=1");
    expect(q("two words")).toBe("'two words'");
    expect(q("it's")).toBe("'it'\\''s'");
    expect(q("")).toBe("''");
  });
});
