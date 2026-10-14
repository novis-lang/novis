import { describe, expect, test } from "bun:test";
import { existsSync, realpathSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { commandArgs, spawnProof } from "../proofs/run.ts";
import { scratch } from "./scratch.ts";

describe("a `.cmd` example", () => {
  test("its arguments are the words after `nvs`, and a quoted word keeps its spaces", () => {
    expect(commandArgs("a.cmd", "nvs agent init --agent codex\n")).toEqual(["agent", "init", "--agent", "codex"]);
    expect(commandArgs("a.cmd", "nvs run \"two words\" 'and more'\r\n")).toEqual(["run", "two words", "and more"]);
  });

  test("a file that is not one `nvs` command line is no example", () => {
    expect(() => commandArgs("a.cmd", "")).toThrow("has 0 lines");
    expect(() => commandArgs("a.cmd", "nvs info\nnvs info\n")).toThrow("has 2 lines");
    expect(() => commandArgs("a.cmd", "rm -rf .\n")).toThrow("starts with `rm`");
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
