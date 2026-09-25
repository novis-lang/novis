// `proc.run` starts a program by its bare name wherever a shell would, handed an `env` or not.

import { describe, expect, test } from "bun:test";
import { resolved, run } from "../lib/proc.ts";

describe("proc", () => {
  test("a program named by a path is started as written", () => {
    expect(resolved(["./tools/x.cmd", "a"], process.env)).toEqual(["./tools/x.cmd", "a"]);
  });

  test("a bare name no PATH entry holds is left for the spawn to refuse", () => {
    expect(resolved(["no-such-program-anywhere", "a"], process.env)).toEqual(["no-such-program-anywhere", "a"]);
  });

  test.skipIf(process.platform !== "win32" || Bun.which("npm") === null)("npm, a `.cmd` on Windows, starts when an env is passed", async () => {
    const r = await run(["npm", "--version"], { env: { NV_PROC_TEST: "1" } });
    expect(r.code).toBe(0);
    expect(r.stdout.trim()).toMatch(/^\d+\.\d+/);
  });
});
