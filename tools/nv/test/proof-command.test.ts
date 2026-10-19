import { describe, expect, test } from "bun:test";
import { existsSync, realpathSync } from "node:fs";
import { join, relative } from "node:path";
import { binary } from "../cmd/proofs.ts";
import { ROOT } from "../lib/paths.ts";
import { skippedOf } from "../proofs/collect.ts";
import { commandArgs, matchesOut, spawnProof } from "../proofs/run.ts";
import { scratch } from "./scratch.ts";

describe("a `.cmd` example", () => {
  test("its arguments are the words after `nvs`, and a quoted word keeps its spaces", () => {
    expect(commandArgs("a.cmd", "nvs agent init --agent codex\n")).toEqual(["agent", "init", "--agent", "codex"]);
    expect(commandArgs("a.cmd", "nvs run \"two words\" 'and more'\r\n")).toEqual(["run", "two words", "and more"]);
  });

  test("a relative `--nvs` is made absolute, so it still names the binary from the example's scratch directory", () => {
    const named = relative(process.cwd(), join(ROOT, "package.json"));
    expect(binary(named)).toBe(join(ROOT, "package.json"));
    expect(binary("no/such/nvs")).toBeNull();
  });

  test("a file that is not one `nvs` command line is no example", () => {
    expect(() => commandArgs("a.cmd", "")).toThrow("has 0 lines");
    expect(() => commandArgs("a.cmd", "nvs info\nnvs info\n")).toThrow("has 2 lines");
    expect(() => commandArgs("a.cmd", "rm -rf .\n")).toThrow("starts with `rm`");
  });

  test("a `...` line of its `.out` matches any number of lines, and every other line matches exactly", () => {
    const want = "Build\n...\nHost\n...\n";
    expect(matchesOut("Build\nHost\n", want, true)).toBe(true);
    expect(matchesOut("Build\r\n  commit abc\r\n  cpus 16\r\nHost\r\n  os linux\r\n", want, true)).toBe(true);
    expect(matchesOut("Build\n  commit abc\nhost\n", want, true)).toBe(false);
    expect(matchesOut("Version\nBuild\nHost\n", want, true)).toBe(false);
    expect(matchesOut("a\n...\nb\n", "a\n...\nb\n", true)).toBe(true);
    expect(matchesOut("a\nb\n", "a\n ...\nb\n", true)).toBe(false);
  });

  test("a program's `.out` has no `...` line: it matches only itself", () => {
    expect(matchesOut("a\nx\nb\n", "a\n...\nb\n", false)).toBe(false);
    expect(matchesOut("a\n...\nb", "a\n...\nb\n", false)).toBe(true);
  });

  test("it runs the binary in an empty directory of its own, which is gone afterwards", async () => {
    const s = scratch();
    try {
      s.put("01-where.cmd", "nvs -e \"console.log(process.cwd()); console.log(require('fs').readdirSync('.').length)\"\n");
      const ran = await spawnProof([process.execPath, "run", "ignored"], join(s.root, "01-where.cmd"), 60_000);
      expect(ran.code).toBe(0);
      const [cwd, entries] = ran.stdout.trim().split(/\r?\n/);
      expect(cwd!.startsWith(join(realpathSync(ROOT), ".agent-tmp", "proof-cmd"))).toBe(true);
      expect(entries).toBe("0");
      expect(existsSync(cwd!)).toBe(false);
    } finally {
      s.cleanup();
    }
  });
});

describe("`nv proofs --gate --no-skip PROOF`", () => {
  test("a feature the policy skips for PROOF owes it, and a skip of another proof is still a skip", () => {
    const skips = new Map([["tools:cli/x", { examples: "why", hostile: "why" }]]);
    expect(skippedOf("tools:cli/x", skips, ["examples"])).toEqual({ examples: "skipped in data/proofs/skip/" });
    expect(skippedOf("tools:cli/x", skips, [])).toEqual({});
    expect(skippedOf("tools:cli/y", skips, ["examples"])).toEqual({});
  });

  test("the gate fails on such a feature and names the proof", () => {
    // `nvs info` skips its attacks, because no program reaches the command.
    const run = (...args: string[]) => Bun.spawnSync(["bun", "tools/nv/main.ts", "proofs", "--gate", ...args, "--only", "tools:cli/nvs-info"], { cwd: ROOT });
    const failed = run("--no-skip", "hostile");
    expect(failed.exitCode).toBe(1);
    expect(failed.stdout.toString()).toContain("hostile: skipped in data/proofs/skip/");
    expect(run("--no-skip", "nonsense").exitCode).toBe(2);
  });
});
