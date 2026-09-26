// The `covws` build: the workspace compiled with LLVM coverage counters in its own crates and in
// nothing else, so a run of `nvs` or of a test binary can say which of the repository's functions it
// executed.
//
// `tools/covwrap` is the `RUSTC_WORKSPACE_WRAPPER` that adds `-C instrument-coverage`, and it adds it
// only to a crate compiled for an explicit `--target`. Cargo runs a workspace wrapper for workspace
// crates alone, and it compiles build scripts and proc-macros for the host without `--target`, so
// dependencies, build scripts and proc-macros carry no counters. The build goes to `target/covws`,
// and its binaries to `target/covws/<host triple>/debug`. A run writes its counters where
// `LLVM_PROFILE_FILE` says, and into `default_*.profraw` in its working directory when that is unset,
// so every run of these binaries sets it.
//
// `llvm-profdata` and `llvm-cov` read the counters. They come with the toolchain's `llvm-tools`
// component, which `rust-toolchain.toml` names.

import { existsSync, statSync } from "node:fs";
import { type TestExe, testExecutables } from "../driver/accept.ts";
import { abs } from "./paths.ts";
import { run } from "./proc.ts";

const WRAPPER = "tools/covwrap";
const WRAPPER_SOURCES = ["Cargo.toml", "Cargo.lock", "src/main.rs"].map((f) => `${WRAPPER}/${f}`);

/** Where the `covws` build goes, relative to the repository root. */
export const COVWS_TARGET = "target/covws";

const EXE = process.platform === "win32" ? ".exe" : "";

let wrapper = "";

/** The wrapper's executable, built first when it is missing or older than its sources. */
export function covwrap(): string {
  if (wrapper) return wrapper;
  const target = abs(`${WRAPPER}/target`);
  const exe = `${target}/release/covwrap${EXE}`;
  const stale = !existsSync(exe) || WRAPPER_SOURCES.some((f) => statSync(abs(f)).mtimeMs > statSync(exe).mtimeMs);
  if (stale) {
    const r = Bun.spawnSync(["cargo", "build", "--release", "--quiet", "--manifest-path", abs(`${WRAPPER}/Cargo.toml`)], {
      env: { ...process.env, CARGO_TARGET_DIR: target },
      stdout: "pipe",
      stderr: "pipe",
    });
    if (r.exitCode !== 0) throw new Error(`covwrap did not build:\n${r.stderr.toString()}`);
  }
  return (wrapper = exe);
}

let triple = "";

/** The host's target triple, as `rustc -vV` names it. */
export function hostTriple(): string {
  if (triple) return triple;
  const r = Bun.spawnSync(["rustc", "-vV"], { cwd: abs("."), stdout: "pipe", stderr: "pipe" });
  const host = /^host: (\S+)$/m.exec(r.stdout.toString());
  if (r.exitCode !== 0 || !host) throw new Error(`rustc -vV named no host:\n${r.stderr.toString()}`);
  return (triple = host[1]!);
}

/** The path of one of the toolchain's LLVM tools. */
export function llvmTool(name: "llvm-profdata" | "llvm-cov"): string {
  const r = Bun.spawnSync(["rustc", "--print", "sysroot"], { cwd: abs("."), stdout: "pipe", stderr: "pipe" });
  const sysroot = r.stdout.toString().trim();
  if (r.exitCode !== 0 || !sysroot) throw new Error(`rustc --print sysroot failed:\n${r.stderr.toString()}`);
  const tool = `${sysroot}/lib/rustlib/${hostTriple()}/bin/${name}${EXE}`;
  if (!existsSync(tool)) throw new Error(`${tool} is missing: the toolchain needs its \`llvm-tools\` component`);
  return tool;
}

/** The environment and arguments every cargo command of the `covws` build takes. */
export function covwsCargo(): { env: Record<string, string>; args: string[] } {
  return {
    env: { CARGO_TARGET_DIR: abs(COVWS_TARGET), RUSTC_WORKSPACE_WRAPPER: covwrap() },
    args: ["--target", hostTriple()],
  };
}

/** The `covws` debug build's `nvs`, absolute: the one every debug run of the pipeline uses. */
export function covwsNvs(): string {
  return abs(`${COVWS_TARGET}/${hostTriple()}/debug/nvs${EXE}`);
}

export interface CovwsBuild {
  triple: string;
  /** `target/covws/<triple>/debug`, absolute. */
  dir: string;
  /** The instrumented `nvs` binary. */
  nvs: string;
  /** Each workspace test executable, by package name, when the build was asked for them. */
  tests: Map<string, TestExe[]>;
}

export interface CovwsOptions {
  /** Also build every workspace test executable, as `cargo test --no-run` does. */
  tests?: boolean;
  /** Called with each line cargo prints. */
  onLine?: (line: string, stream: "stdout" | "stderr") => void;
  timeoutMs?: number;
}

/**
 * Builds the `covws` tree, incrementally like any debug build, and returns where its binaries are.
 * Throws with cargo's output when a build fails.
 */
export async function buildCovws(opts: CovwsOptions = {}): Promise<CovwsBuild> {
  const { env, args } = covwsCargo();
  const cargo = async (sub: string[]) => {
    const r = await run(["cargo", ...sub, ...args, "--message-format=json-render-diagnostics"], {
      env,
      ...(opts.onLine ? { onLine: opts.onLine } : {}),
      timeoutMs: opts.timeoutMs ?? 60 * 60 * 1000,
    });
    if (r.code !== 0) throw new Error(`\`cargo ${sub.join(" ")}\` for covws failed:\n${r.stderr}${r.stdout}`);
    return r.stdout;
  };
  await cargo(["build"]);
  const dir = abs(`${COVWS_TARGET}/${hostTriple()}/debug`);
  const nvs = `${dir}/nvs${EXE}`;
  if (!existsSync(nvs)) throw new Error(`the covws build left no ${nvs}`);
  const tests = opts.tests ? testExecutables(await cargo(["test", "--no-run"])) : new Map<string, TestExe[]>();
  return { triple: hostTriple(), dir, nvs, tests };
}
