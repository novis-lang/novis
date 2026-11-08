// `bun nv deep`: every `ci.yml` job is a local step or CI-only with its reason, and the runner's
// choices of steps, report and lines.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { CI_ONLY, TOOLS, type Verdict, cmdLine, jobOf, merged, selected, steps, tail, wslLine } from "../cmd/deep.ts";
import { ROOT } from "../lib/paths.ts";

const ciJobs = (): string[] => {
  const yml = Bun.YAML.parse(readFileSync(join(ROOT, ".github/workflows/ci.yml"), "utf8")) as { jobs: Record<string, unknown> };
  return Object.keys(yml.jobs).sort();
};

const v = (id: string, ok: boolean): Verdict => ({ id, job: id.replace(/-(windows|linux)$/, ""), ok, seconds: 1, failed: ok ? "" : "x", log: "" });

describe("bun nv deep", () => {
  test("every ci.yml job is a local step or CI-only with its reason", () => {
    const covered = new Set([...steps().map((s) => s.job), ...Object.keys(CI_ONLY).map(jobOf)]);
    const uncovered = ciJobs().filter((j) => !covered.has(j));
    expect(uncovered).toEqual([]);
  });

  test("the table names no job ci.yml lacks, and every CI-only entry has a reason", () => {
    const jobs = new Set(ciJobs());
    expect(steps().map((s) => s.job).filter((j) => !jobs.has(j))).toEqual([]);
    expect(Object.keys(CI_ONLY).map(jobOf).filter((j) => !jobs.has(j))).toEqual([]);
    expect(Object.values(CI_ONLY).every((why) => why.length > 10)).toBe(true);
  });

  test("step ids are unique, every step runs something, and every tool it needs has an install line", () => {
    const all = steps();
    expect(new Set(all.map((s) => s.id)).size).toBe(all.length);
    for (const s of all) {
      expect(s.cmds.length).toBeGreaterThan(0);
      for (const t of s.tools ?? []) expect(TOOLS[t]?.install.length ?? 0).toBeGreaterThan(10);
    }
  });

  test("no debug cargo command takes -p", () => {
    for (const s of steps()) {
      for (const c of s.cmds) {
        const line = "wsl" in c ? c.wsl : c.argv.join(" ");
        if (!/\bcargo\b/.test(line) || /--release|\bmiri\b|\+nightly/.test(line) || /--profile/.test(line)) continue;
        expect(`${s.id}: ${line}`).not.toMatch(/\bcargo [^&|;]* (-p|--package) /);
      }
    }
  });

  test("--only picks a job's steps or one step, and names a job it does not know", () => {
    const all = steps();
    const picked = selected(all, { failed: false, only: "test" }, []);
    expect(Array.isArray(picked) && picked.map((s) => s.id)).toEqual(["test-windows", "test-linux"]);
    const one = selected(all, { failed: false, only: "proofs-linux" }, []);
    expect(Array.isArray(one) && one.map((s) => s.id)).toEqual(["proofs-linux"]);
    expect(typeof selected(all, { failed: false, only: "nope" }, [])).toBe("string");
  });

  test("--failed runs only the steps the last report names red, and the report keeps the others", () => {
    const all = steps();
    const last = [v("docs", true), v("miri", false), v("test-linux", false)];
    const picked = selected(all, { failed: true, only: null }, last);
    expect(Array.isArray(picked) && picked.map((s) => s.id).sort()).toEqual(["miri", "test-linux"]);
    expect(merged(last, [v("miri", true)]).map((x) => `${x.id}:${x.ok}`)).toEqual(["docs:true", "test-linux:false", "miri:true"]);
  });

  test("a WSL command runs in the copy, into the WSL target directory unless it names its own", () => {
    const line = wslLine("/var/tmp/copy", { wsl: "cargo test", env: { CARGO_TARGET_DIR: "/var/tmp/other", MIRIFLAGS: "-Zx -Zy" } });
    expect(line).toBe(
      "set -o pipefail; cd /var/tmp/copy && export CARGO_TARGET_DIR=/var/tmp/other; export INSTA_WORKSPACE_ROOT=/var/tmp/copy; export MIRIFLAGS='-Zx -Zy'; cargo test",
    );
    expect(wslLine("/c", { wsl: "x" })).toContain("export CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl; ");
  });

  test("a command's line names where it runs and the variables it sets", () => {
    expect(cmdLine({ argv: ["npm", "ci"], cwd: "editors/vscode", env: { A: "1" } })).toBe("(in editors/vscode) A=1 npm ci");
    expect(cmdLine({ wsl: "cargo doc" })).toBe("wsl: cargo doc");
    expect(tail("a\nb\nc\n", 2)).toBe("b\nc");
  });
});
