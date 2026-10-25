import { describe, expect, test } from "bun:test";
import { existsSync, realpathSync } from "node:fs";
import { join, relative } from "node:path";
import { binary } from "../cmd/proofs.ts";
import { ROOT } from "../lib/paths.ts";
import { detailsProblems, skippedOf } from "../proofs/collect.ts";
import { commandArgs, commandLine, exampleOutput, firstDifference, matchesOut, proofArgv, spawnProof, testReport } from "../proofs/run.ts";
import { scratch } from "./scratch.ts";

describe("a `.cmd` example", () => {
  test("its arguments are the words after `nvs`, and a quoted word keeps its spaces", () => {
    expect(commandArgs("a.cmd", "nvs agent init --agent codex\n")).toEqual(["agent", "init", "--agent", "codex"]);
    expect(commandArgs("a.cmd", "nvs run \"two words\" 'and more'\r\n")).toEqual(["run", "two words", "and more"]);
  });

  test("each `NAME=value` word before `nvs` sets a variable for the command", () => {
    expect(commandLine("a.cmd", "DO_NOT_TRACK=1 NVS_UPDATE_URL= nvs update-check\n")).toEqual({
      env: { DO_NOT_TRACK: "1", NVS_UPDATE_URL: "" },
      args: ["update-check"],
    });
    expect(() => commandArgs("a.cmd", "DO_NOT_TRACK=1 rm x\n")).toThrow("starts with `rm`");
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

  test("a failure names the first line the output and its `.out` differ on", () => {
    expect(firstDifference("Anna\r\nDora\n", "Anna\nCarla\n")).toBe("line 2 is `Dora` where the .out has `Carla`");
    expect(firstDifference("Anna\n", "Anna\nCarla\n")).toBe("line 2 is nothing where the .out has `Carla`");
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

  test("a bench row keeps its counts and loses its advisory wall time", () => {
    const row = "    ✓ b    1.0 ms\n      count: statements=6 calls=2 allocations=1 bytes=8 gc=0, wall 0.642 ms (advisory)\n";
    expect(testReport(row)).toBe("    ✓ b\n      count: statements=6 calls=2 allocations=1 bytes=8 gc=0\n");
  });

  test("it runs under `nvs test`, and under `nvs test --mutate` or `--bench` when it says so", () => {
    expect(proofArgv("nvs", "a.nvs", "<?nvs\n// proof: test\n")).toEqual(["nvs", "test", "a.nvs"]);
    expect(proofArgv("nvs", "a.nvs", "<?nvs\n// proof: test --mutate\n")).toEqual(["nvs", "test", "a.nvs", "--mutate"]);
    expect(proofArgv("nvs", "a.nvs", "<?nvs\n// proof: test --bench\n")).toEqual(["nvs", "test", "a.nvs", "--bench"]);
    expect(proofArgv("nvs", "a.nvs", "<?nvs\n")).toEqual(["nvs", "run", "a.nvs"]);
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
