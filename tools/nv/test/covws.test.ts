import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { COVWS_TARGET, covwrap, covwsCargo, hostTriple, llvmTool } from "../lib/covws.ts";
import { abs } from "../lib/paths.ts";

describe("the covws build", () => {
  test("the wrapper runs the compiler it is handed and answers with its exit status", () => {
    const ok = Bun.spawnSync([covwrap(), "rustc", "--version"], { stdout: "pipe", stderr: "pipe" });
    expect(ok.exitCode).toBe(0);
    expect(ok.stdout.toString()).toStartWith("rustc ");
    const bad = Bun.spawnSync([covwrap(), "rustc", "--no-such-flag"], { stdout: "pipe", stderr: "pipe" });
    expect(bad.exitCode).not.toBe(0);
    // The first call may build the wrapper.
  }, 180_000);

  test("cargo is pointed at its own target directory, the wrapper and the host triple", () => {
    const { env, args } = covwsCargo();
    expect(env.CARGO_TARGET_DIR).toBe(abs(COVWS_TARGET));
    expect(env.RUSTC_WORKSPACE_WRAPPER).toBe(covwrap());
    expect(args).toEqual(["--target", hostTriple()]);
    expect(hostTriple()).toMatch(/^[a-z0-9_]+-[a-z0-9_]+-[a-z0-9_]+/);
  });

  test("the toolchain carries the LLVM tools that read the counters", () => {
    expect(existsSync(llvmTool("llvm-profdata"))).toBe(true);
    expect(existsSync(llvmTool("llvm-cov"))).toBe(true);
  });
});
