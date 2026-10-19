import { describe, expect, test } from "bun:test";
import { existsSync, realpathSync } from "node:fs";
import { join, relative } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { detailsProblems, skippedOf } from "../proofs/collect.ts";
import { commandArgs, exampleOutput, matchesOut, spawnProof, testReport } from "../proofs/run.ts";
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

describe("a `proof: compile-error` example", () => {
  const ran = { stdout: "", stderr: "error[E0475]: `break 2` names more levels than enclose it\n  --> docs/examples/lang/x/04-too-far.nvs:3:11\n" };

  test("its `.out` is the diagnostic, with the path shortened to the file name", () => {
    const source = "<?nvs\n// proof: compile-error\nbreak 2;\n";
    expect(exampleOutput("docs/examples/lang/x/04-too-far.nvs", source, ran)).toBe("error[E0475]: `break 2` names more levels than enclose it\n  --> 04-too-far.nvs:3:11\n");
    const windows = { stdout: "", stderr: ran.stderr.replace("docs/examples/lang/x/", "docs\\examples\\lang\\x\\") };
    expect(exampleOutput("docs/examples/lang/x/04-too-far.nvs", source, windows)).toContain("--> 04-too-far.nvs:3:11");
  });

  test("an example without the line freezes its standard output alone", () => {
    expect(exampleOutput("a.nvs", "<?nvs\necho 1;\n", { stdout: "1", stderr: "noise" })).toBe("1");
  });
});

describe("a `proof: test` example", () => {
  test("its `.out` is the `nvs test` report without a duration", () => {
    const report = "  SumTest\n    ✓ itAdds                           0.3 ms\n    ✗ itFails                          12.0 ms\n      Core\\Test::assertSame failed: `$actual` is 4, `$expected` is 5\n    - itWaits                          0.0 ms\n      skipped: not yet\n\n  1 failed, 1 passed, 1 skipped, 0 flaky in 14 ms\n";
    const source = "<?nvs\n// proof: test\n";
    expect(exampleOutput("a.nvs", source, { stdout: report, stderr: "" })).toBe(
      "  SumTest\n    ✓ itAdds\n    ✗ itFails\n      Core\\Test::assertSame failed: `$actual` is 4, `$expected` is 5\n    - itWaits\n      skipped: not yet\n\n  1 failed, 1 passed, 1 skipped, 0 flaky\n",
    );
  });

  test("a line the tests print keeps its text", () => {
    expect(testReport("  T\ncreate 2 ms\n    ✓ a  1.5 ms\n")).toBe("  T\ncreate 2 ms\n    ✓ a\n");
  });
});

describe("a feature's `details.md`", () => {
  const judge = (text: string) => {
    const s = scratch();
    try {
      s.put("details.md", text);
      return detailsProblems(relative(ROOT, join(s.root, "details.md")).replaceAll("\\", "/"));
    } finally {
      s.cleanup();
    }
  };

  test("plain paragraphs and list items inside the bounds pass, at any length", () => {
    const para = "A level counts every loop. ".repeat(40);
    expect(judge(`${para}\n\n- A \`switch\` is one level.\n- The number is written in the code.\n`)).toEqual([]);
  });

  test("a heading, a code block, a dash and a long sentence are each named with their line", () => {
    const long = Array.from({ length: 30 }, (_, i) => `w${i}`).join(" ") + ".";
    const problems = judge(`# Title\n\nOne idea — and another.\n\n\`\`\`nvs\n\`\`\`\n\n- ${long}\n`);
    expect(problems[0]).toStartWith("1: opens with a heading");
    expect(problems).toContain("3: a dash joins two sentences; write two");
    expect(problems).toContain("5: a code block, and the examples are where code goes");
    expect(problems.some((p) => p.startsWith("8: a 30-word sentence"))).toBe(true);
  });

  test("each list item is its own run of prose, so two short items do not add up", () => {
    const half = Array.from({ length: 15 }, (_, i) => `w${i}`).join(" ");
    expect(judge(`- ${half}\n- ${half}\n`)).toEqual([]);
    expect(judge(`${half}\n${half}\n`)).toHaveLength(1);
  });
});

describe("`nv proofs --gate --no-skip PROOF`", () => {
  test("a feature the policy skips for PROOF owes it, and a skip of another proof is still a skip", () => {
    const skips = new Map([["tools:cli/x", { examples: "why", hostile: "why" }]]);
    expect(skippedOf("tools:cli/x", skips, ["examples"])).toEqual({ examples: "skipped in data/proofs/policy.json" });
    expect(skippedOf("tools:cli/x", skips, [])).toEqual({});
    expect(skippedOf("tools:cli/y", skips, ["examples"])).toEqual({});
  });

  test("the gate fails on such a feature and names the proof", () => {
    // `nvs info` skips its attacks, because no program reaches the command.
    const run = (...args: string[]) => Bun.spawnSync(["bun", "tools/nv/main.ts", "proofs", "--gate", ...args, "--only", "tools:cli/nvs-info"], { cwd: ROOT });
    const failed = run("--no-skip", "hostile");
    expect(failed.exitCode).toBe(1);
    expect(failed.stdout.toString()).toContain("hostile: skipped in data/proofs/policy.json");
    expect(run("--no-skip", "nonsense").exitCode).toBe(2);
  });
});
