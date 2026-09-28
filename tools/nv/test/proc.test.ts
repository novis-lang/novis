// `proc.run` starts a program by its bare name wherever a shell would, handed an `env` or not, and
// every program it starts is told where an instrumented binary writes its coverage counters. An
// aborted signal kills the program it was handed to.

import { describe, expect, test } from "bun:test";
import { DISCARD_PROFILE } from "../lib/paths.ts";
import { childEnv, passthrough, resolved, run } from "../lib/proc.ts";

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

  test("aborting the signal kills the program, and the run reports it as aborted", async () => {
    const stop = new AbortController();
    const started = Date.now();
    const r = await run([process.execPath, "-e", "console.log('up'); setTimeout(() => {}, 60000)"], { signal: stop.signal, onLine: () => stop.abort() });
    expect(r.aborted).toBe(true);
    expect(r.timedOut).toBe(false);
    expect(r.code).toBe(130);
    expect(Date.now() - started).toBeLessThan(30000);
  });

  test("a run whose signal is already aborted starts nothing", async () => {
    const stop = new AbortController();
    stop.abort();
    const r = await run(["no-such-program-anywhere"], { signal: stop.signal });
    expect(r).toMatchObject({ aborted: true, code: 130, stdout: "" });
  });

  test("a run that ends before its signal is aborted is not aborted", async () => {
    const stop = new AbortController();
    const r = await run([process.execPath, "-e", "0"], { signal: stop.signal });
    stop.abort();
    expect(r).toMatchObject({ aborted: false, code: 0 });
  });
});

describe("no started program writes a default profile", () => {
  const PRINT = ["-e", "console.log(process.env.LLVM_PROFILE_FILE ?? 'unset')"];

  test("a program started with no LLVM_PROFILE_FILE anywhere gets the discard pattern", async () => {
    const was = process.env.LLVM_PROFILE_FILE;
    delete process.env.LLVM_PROFILE_FILE;
    try {
      expect(childEnv().LLVM_PROFILE_FILE).toBe(DISCARD_PROFILE);
      const r = await run([process.execPath, ...PRINT]);
      expect(r.stdout.trim()).toBe(DISCARD_PROFILE);
      const r2 = await run([process.execPath, ...PRINT], { env: { NV_PROC_TEST: "1" } });
      expect(r2.stdout.trim()).toBe(DISCARD_PROFILE);
    } finally {
      if (was !== undefined) process.env.LLVM_PROFILE_FILE = was;
    }
  });

  test("a pattern the caller or this process names is kept", async () => {
    const r = await run([process.execPath, ...PRINT], { env: { LLVM_PROFILE_FILE: "atom-%p.profraw" } });
    expect(r.stdout.trim()).toBe("atom-%p.profraw");
    const was = process.env.LLVM_PROFILE_FILE;
    process.env.LLVM_PROFILE_FILE = "mine-%p.profraw";
    try {
      expect(childEnv().LLVM_PROFILE_FILE).toBe("mine-%p.profraw");
    } finally {
      if (was === undefined) delete process.env.LLVM_PROFILE_FILE;
      else process.env.LLVM_PROFILE_FILE = was;
    }
  });

  test("passthrough gives its program the same environment", async () => {
    const was = process.env.LLVM_PROFILE_FILE;
    delete process.env.LLVM_PROFILE_FILE;
    try {
      const code = await passthrough([process.execPath, "-e", `process.exit(process.env.LLVM_PROFILE_FILE === ${JSON.stringify(DISCARD_PROFILE)} ? 0 : 3)`]);
      expect(code).toBe(0);
    } finally {
      if (was !== undefined) process.env.LLVM_PROFILE_FILE = was;
    }
  });
});
