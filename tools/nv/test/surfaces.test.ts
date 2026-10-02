// The `nv` commands a session or the hook reaches by name, run against this tree: `brief --where`,
// `guard --check`, `orient --traps`, `loop --list` with its filters, and `loop-stats --guard`'s count.
// Each test fails when the command stops answering the question a session asks it.

import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import type { Check } from "../driver/accept.ts";
import { featureMatches } from "../cmd/loop.ts";
import { readSession } from "../cmd/loop-stats.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { load } from "../lib/store.ts";
import { playbookBullet } from "../schema/playbook.ts";
import { scratch } from "./scratch.ts";

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

  test("loop --list narrows the live goal's checks by stage and by name", async () => {
    const all = await nv("loop", "--list");
    expect(all.code).toBe(0);
    const label = /^ {2}\[([^\]]+)\]/m.exec(all.stdout)![1]!;
    const stage = await nv("loop", "--list", "--stage", label.split(" ")[0]!);
    expect(stage.code).toBe(0);
    const shown = stage.stdout.split(/\r?\n/).filter((l) => /^ {2}\[/.test(l));
    expect(shown.length).toBeGreaterThan(0);
    expect(shown.every((l) => l.startsWith(`  [${label}]`))).toBe(true);
    const none = await nv("loop", "--list", "--name", "no check is named this");
    expect(none.code).toBe(0);
    expect(none.stdout).toContain("list: 0 checks match");
    expect((await nv("loop", "--list", "--stage", "no-such-stage")).code).toBe(2);
  });

  test("loop --list --feature matches a proofs check by its name, its --group or its --id", () => {
    const check = (over: Partial<Check>): Check => ({ id: "c", kind: "command", stage: 1, ...over });
    expect(featureMatches(check({ name: "proofs: tools:config (2/3)" }), "tools:config")).toBe(true);
    expect(featureMatches(check({ argv: ["bun", "nv", "proofs", "--verify", "--group", "Core\\Str"] }), "Core\\Str")).toBe(true);
    expect(featureMatches(check({ argv: ["bun", "nv", "proofs", "--id", "Core\\Str::length"] }), "Core\\Str::length")).toBe(true);
    expect(featureMatches(check({ name: "proofs: Core\\Str", argv: ["bun", "nv", "proofs", "--group", "Core\\Arr"] }), "Core\\Uri")).toBe(false);
  });

  test("loop-stats --guard counts a denied call under the rule that denied it", () => {
    const t = scratch();
    try {
      const events = [
        { type: "assistant", message: { id: "m1", content: [{ type: "tool_use", id: "t1", name: "Bash", input: { command: "cat a.rs" } }] } },
        { type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t1", is_error: true, content: "PreToolUse:Bash hook error: guard: whole-read: use nv peek" }] } },
        { type: "assistant", message: { id: "m2", content: [{ type: "tool_use", id: "t2", name: "Bash", input: { command: "ls" } }] } },
        { type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t2", content: "a.rs" }] } },
      ];
      t.put("session.jsonl", events.map((e) => JSON.stringify(e)).join("\n"));
      expect(readSession(join(t.root, "session.jsonl"))!.guard).toEqual({ "whole-read": 1 });
    } finally {
      t.cleanup();
    }
  });
});
