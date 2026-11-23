// `bun nv release --fixes-built`: a tip with a commit that changes shipped code the build did not have
// fails the release, whatever the commit's type. The last test runs the command's own `git log`
// arguments in a scratch repository, so the format and the merge handling are the ones the release runs.
import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { isShipped, parseTouched, shippedChanges, shippedLogArgs, TOUCHED_FORMAT } from "../cmd/release.ts";

describe("nv release --fixes-built", () => {
  test("a path is shipped when it is under a shipped folder or is a shipped root file", () => {
    for (const p of ["crates/nvs-runtime/src/lib.rs", "extensions/pdf/src/lib.rs", "vendor/x/y.c", "editors/vscode/src/a.ts", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]) {
      expect(isShipped(p)).toBe(true);
    }
    for (const p of ["docs/rules/x.md", "website/src/a.astro", "tools/nv/cmd/release.ts", "crates", "fuzz/Cargo.toml", "Cargo.toml.bak", "tests/hostile/a.nvs"]) {
      expect(isShipped(p)).toBe(false);
    }
  });

  test("every type of commit that changes shipped code is named, and only its shipped paths", () => {
    const raw =
      "\x1ea1\x1fperf(runtime): faster\n\ncrates/nvs-runtime/src/a.rs\n" +
      "\x1eb2\x1fdocs: a page\n\ndocs/a.md\nwebsite/b.astro\n" +
      "\x1ec3\x1fno conventional subject\n\nCargo.lock\n" +
      "\x1ed4\x1ffix(stdlib): a fix\n\ncrates/nvs-stdlib/src/b.rs\ndocs/c.md\n" +
      "\x1ee5\x1frefactor: an empty commit\n";
    const commits = parseTouched(raw);
    expect(commits.map((c) => c.sha)).toEqual(["a1", "b2", "c3", "d4", "e5"]);
    expect(commits[3]!.paths).toEqual(["crates/nvs-stdlib/src/b.rs", "docs/c.md"]);
    const shipped = shippedChanges(commits);
    expect(shipped.map((c) => c.sha)).toEqual(["a1", "c3", "d4"]);
    expect(shipped[2]!.paths).toEqual(["crates/nvs-stdlib/src/b.rs"]);
  });

  test("the command's git log names a later commit and a merge that change shipped code, and nothing else", () => {
    const scratch = join(ROOT, ".agent-tmp");
    mkdirSync(scratch, { recursive: true });
    const dir = mkdtempSync(join(scratch, "fixes-built-"));
    const env = {
      ...process.env,
      GIT_CONFIG_GLOBAL: join(dir, "none"),
      GIT_CONFIG_NOSYSTEM: "1",
      GIT_AUTHOR_NAME: "t",
      GIT_AUTHOR_EMAIL: "t@example.com",
      GIT_COMMITTER_NAME: "t",
      GIT_COMMITTER_EMAIL: "t@example.com",
    };
    const git = (...args: string[]): string => {
      const r = Bun.spawnSync(["git", "-C", dir, "-c", "core.hooksPath=", ...args], { env });
      if (r.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${r.stderr.toString()}`);
      return r.stdout.toString();
    };
    const commit = (file: string, subject: string): void => {
      mkdirSync(join(dir, file, ".."), { recursive: true });
      writeFileSync(join(dir, file), `${subject}\n`);
      git("add", file);
      git("commit", "--quiet", "-m", subject);
    };
    try {
      git("init", "--quiet", "--initial-branch=main");
      commit("crates/a/src/lib.rs", "feat: the first");
      const built = git("rev-parse", "HEAD").trim();
      commit("docs/a.md", "docs: a page");
      commit("crates/a/src/lib.rs.orig", "chore: a file under crates");
      git("checkout", "--quiet", "-b", "side");
      commit("vendor/x.c", "refactor: vendored");
      git("checkout", "--quiet", "main");
      commit("website/a.astro", "docs: the site");
      git("merge", "--quiet", "--no-ff", "-m", "merge side", "side");
      expect(shippedLogArgs(built)).toContain(`--format=${TOUCHED_FORMAT}`);
      const shipped = shippedChanges(parseTouched(git(...shippedLogArgs(built))));
      expect(shipped.map((c) => c.subject)).toEqual(["chore: a file under crates", "merge side"]);
      expect(shipped[1]!.paths).toEqual(["vendor/x.c"]);
      expect(shippedChanges(parseTouched(git(...shippedLogArgs("HEAD"))))).toEqual([]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
