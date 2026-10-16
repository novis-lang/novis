// The `nv` commands a session or the hook reaches by name, run against this tree: `brief --where` and
// `guard --check`. Each test fails when the command stops answering the question a session asks it.

import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";

const MAIN = join(ROOT, "tools", "nv", "main.ts");
const nv = (...args: string[]) => run([process.execPath, MAIN, ...args], { cwd: ROOT, timeoutMs: 60_000 });

describe("the commands a session reaches by name", () => {
  test("brief --where routes a keyword to the rules that hold it", async () => {
    const r = await nv("brief", "--where", "testing");
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("docs/rules/testing.md#");
    expect(r.stdout).toContain("rule:testing/");
  });

  test("guard --check finds the hook wired for Read and Bash in the committed settings", async () => {
    const r = await nv("guard", "--check");
    expect(r.code).toBe(0);
    expect(r.stdout).toStartWith("guard: wired for Read and Bash");
  });
});
