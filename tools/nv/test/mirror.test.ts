import { describe, expect, test } from "bun:test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { CARRIED, carryLine, headLine, headOf, mirrorPath, q, targetLine, wslBuildLine, wslPath } from "../accept/mirror.ts";
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

  test("headLine gives a repository with no commit the checkout's HEAD, with its id and date", () => {
    const root = join(ROOT, ".agent-tmp", "mirror-test-head");
    const git = (cwd: string, ...args: string[]) => {
      const r = Bun.spawnSync(["git", ...args], { cwd, env: { ...process.env, GIT_AUTHOR_DATE: "2026-09-24T10:00:00Z", GIT_COMMITTER_DATE: "2026-09-24T10:00:00Z" } });
      expect(r.exitCode).toBe(0);
      return r.stdout.toString().trim();
    };
    try {
      const from = join(root, "from");
      const to = join(root, "to");
      mkdirSync(from, { recursive: true });
      mkdirSync(to, { recursive: true });
      git(from, "init", "-q");
      writeFileSync(join(from, "a.txt"), "a\n");
      git(from, "add", "a.txt");
      git(from, "-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-q", "-m", "first");
      git(from, "-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-q", "--allow-empty", "-m", "Straße\n\nsecond");
      git(to, "init", "-q");
      const head = headOf(from);
      expect(head).not.toBeNull();
      expect(head!.sha).toBe(git(from, "rev-parse", "HEAD"));
      const line = `true${headLine(head!)}`;
      for (const _ of [1, 2]) {
        const r = Bun.spawnSync(["bash", "-c", line], { cwd: to });
        expect(r.stderr.toString()).toBe("");
        expect(r.exitCode).toBe(0);
      }
      expect(git(to, "rev-parse", "HEAD")).toBe(head!.sha);
      expect(git(to, "log", "-1", "--format=%cs", "HEAD")).toBe("2026-09-24");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("q quotes a word only when bash would split or expand it", () => {
    expect(q("examples/a.nvs")).toBe("examples/a.nvs");
    expect(q("--flag=1")).toBe("--flag=1");
    expect(q("two words")).toBe("'two words'");
    expect(q("it's")).toBe("'it'\\''s'");
    expect(q("")).toBe("''");
  });
});
