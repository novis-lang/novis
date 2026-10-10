// The `nv` commands a session or the hook reaches by name, run against this tree: `brief --where`,
// `guard --check` and `orient --traps`, and the matcher behind `loop --list --feature`.
// Each test fails when the command stops answering the question a session asks it.

import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { type Check, featureMatches } from "../accept/accept.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { load } from "../lib/store.ts";
import { playbookBullet } from "../schema/playbook.ts";

const MAIN = join(ROOT, "tools", "nv", "main.ts");
const nv = (...args: string[]) => run([process.execPath, MAIN, ...args], { cwd: ROOT, timeoutMs: 60_000 });

describe("the commands a session reaches by name", () => {
  test("brief --where routes a keyword to the rules that hold it", async () => {
    const r = await nv("brief", "--where", "testing");
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("docs/rules/testing.md#");
    expect(r.stdout).toContain("rule:testing/");
  });

  test("guard --check finds the hook wired for all three tools in the committed settings", async () => {
    const r = await nv("guard", "--check");
    expect(r.code).toBe(0);
    expect(r.stdout).toStartWith("guard: wired for Read, Bash and PowerShell");
  });

  test("orient --traps prints the bullets that name a path, and says so when none does", async () => {
    const named = load(playbookBullet, ROOT).flatMap((b) => (b.value as { files: string[] }).files)[0]!;
    const hit = await nv("orient", "--traps", named);
    expect(hit.code).toBe(0);
    expect(hit.stdout).toContain("== THE TRAPS THAT APPLY HERE");
    expect(hit.stdout).not.toContain("No bullet names any of these paths.");
    const miss = await nv("orient", "--traps", "zzq-none/zzq-none.txt");
    expect(miss.code).toBe(0);
    expect(miss.stdout).toContain("No bullet names any of these paths.");
  });

  test("loop --list --feature matches a proofs check by its name, its --group or its --id", () => {
    const check = (over: Partial<Check>): Check => ({ id: "c", kind: "command", stage: 1, ...over });
    expect(featureMatches(check({ name: "proofs: tools:config (2/3)" }), "tools:config")).toBe(true);
    expect(featureMatches(check({ argv: ["bun", "nv", "proofs", "--verify", "--group", "Core\\Str"] }), "Core\\Str")).toBe(true);
    expect(featureMatches(check({ argv: ["bun", "nv", "proofs", "--id", "Core\\Str::length"] }), "Core\\Str::length")).toBe(true);
    expect(featureMatches(check({ name: "proofs: Core\\Str", argv: ["bun", "nv", "proofs", "--group", "Core\\Arr"] }), "Core\\Uri")).toBe(false);
  });
});
