// `bun nv linux-test`: one `cargo test` run on a Linux build of the working tree, inside WSL.
//
//     bun nv linux-test --test env_context a_queued_job      # one integration test binary, filtered
//     bun nv linux-test --lib io::tests::within               # the unit tests whose names match
//
// It is for a change whose code behaves differently on Linux: paths, file permissions, sockets,
// processes and signals. CI runs every test on Linux and macOS, and a change checked on Windows alone
// meets those differences there, an hour or two after its push. This command meets the Linux half in
// minutes, before the commit.
//
// It brings the WSL leg's copy of the tree up to date (`accept/mirror.ts` `syncMirror`), then runs
// `cargo test` with the arguments as they are given, in that copy, with `CARGO_TARGET_DIR` at
// `WSL_TARGET` on the distro's own disk. Its output is cargo's, as it prints it, and its exit status is
// cargo's. `-p` and `--package` are refused, as every debug cargo command refuses them (AGENTS.md
// § *The rules only this project has*). An argument list with no filter, `--test` or `--lib` is
// refused too: the whole suite is CI's.
//
// Windows only, since on Linux `cargo test` is the same run. It exits 2 on a usage error or when the
// copy cannot be synced. What it spends: the first run builds the test profile into `WSL_TARGET`, which
// takes tens of minutes cold; each run after it rebuilds the crates the change reached.

import { mirrorPath, q, syncMirror, wslPath } from "../accept/mirror.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough } from "../lib/proc.ts";

export const summary = "runs one filtered `cargo test` on a Linux build of the tree, inside WSL: nv linux-test <cargo test args>";

const USAGE = "usage: bun nv linux-test [--test <name> | --lib] [<filter>] [<cargo test args>...]";

/** The WSL leg's target directory, which `garden` and `scaling` build into too. */
const WSL_TARGET = "/var/tmp/nvs-target-wsl";
/** A cold test-profile build of the workspace and the run after it. */
const TIMEOUT_MS = 2 * 60 * 60 * 1000;

/** Why `args` is not a run this command takes, or "" when it is. */
export function refusal(args: string[]): string {
  if (args.some((a) => a === "-p" || a === "--package" || a.startsWith("--package=") || /^-p./.test(a))) {
    return "a debug cargo command never takes -p; narrow the run with --test <name>, --lib or a filter";
  }
  const before = args.includes("--") ? args.slice(0, args.indexOf("--")) : args;
  const narrowed = before.some((a) => a === "--test" || a === "--lib" || !a.startsWith("-"));
  return narrowed ? "" : "name what runs: --test <name>, --lib or a test name filter; the whole suite is CI's";
}

/** The `bash` line that runs `cargo test` with `args` in the copy at `mirror`. */
export function testLine(mirror: string, args: string[]): string {
  return `cd ${q(mirror)} && CARGO_TARGET_DIR=${q(WSL_TARGET)} cargo test ${args.map(q).join(" ")}`;
}

export async function run(args: string[]): Promise<number> {
  if (args.length === 0 || args[0] === "--help" || args[0] === "-h") {
    console.log(`${USAGE}\n\n${summary}`);
    return args.length === 0 ? 2 : 0;
  }
  const bad = refusal(args);
  if (bad !== "") {
    console.error(`${USAGE}\nnv linux-test: error: ${bad}`);
    return 2;
  }
  if (process.platform !== "win32") {
    console.error("nv linux-test: this host is not Windows; run `cargo test` directly");
    return 2;
  }
  const mirror = mirrorPath(WSL_TARGET, wslPath(ROOT));
  console.log(`linux-test: syncing the WSL copy of the tree at ${mirror}`);
  const synced = await syncMirror(mirror);
  if (synced !== "") {
    console.error(`nv linux-test: the WSL copy of the tree could not be synced: ${synced}`);
    return 2;
  }
  // `wsl.exe` hides the distro's CPU from the hang watch, and cargo prints nothing while one crate
  // compiles, so the watch is off and the timeout bounds the run.
  return passthrough(["wsl.exe", "--exec", "bash", "-lc", testLine(mirror, args)], { timeoutMs: TIMEOUT_MS, hang: false });
}
