// `bun nv fuzz`: one `cargo fuzz run` of one target under `fuzz/`, inside WSL.
//
//     bun nv fuzz markdown --runs 1000000        # a million inputs, then stop
//     bun nv fuzz markdown_from_html --seconds 300
//
// libFuzzer does not run on Windows, so a target runs in WSL with the distro's nightly toolchain and
// `cargo-fuzz`. This command brings the WSL leg's copy of the tree up to date (`accept/mirror.ts`
// `syncMirror`) and runs the target there, as CI's `fuzz-smoke` job runs it: the target's seeds from
// `fuzz/seeds/<target>/` (`parse`'s for `ast`) and its dictionary from `fuzz/dicts/<target>.dict`
// (`markdown`'s for `markdown_comrak`) when it has one.
//
// The sync runs `git clean -ffdx` in the copy, so nothing the run keeps lives there. The build goes to
// `FUZZ_TARGET`, the corpus a run grows to `CORPUS/<target>/`, and a crash's input to
// `ARTIFACTS/<target>/`, all on the distro's own disk. The corpus carries over from one run to the next.
// A crash prints the input's path there and its bytes in base64, so the input can be copied into a
// test. Its output is libFuzzer's and its exit status is cargo-fuzz's.
//
// Windows only, since on Linux `cargo +nightly fuzz run` is the same run. It exits 2 on a usage error or
// when the copy cannot be synced. What it spends: the first run builds the fuzz workspace into
// `FUZZ_TARGET`, which takes minutes cold, and the corpus grows with each run.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { mirrorPath, q, syncMirror, wslPath } from "../accept/mirror.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough } from "../lib/proc.ts";

export const summary = "runs one fuzz target in WSL, for a number of inputs or seconds: nv fuzz <target> --runs <n> | --seconds <n>";

const USAGE = "usage: bun nv fuzz <target> (--runs <n> | --seconds <n>)";

/** The tree copy `linux-test` syncs, named by its target directory. */
const WSL_TARGET = "/var/tmp/nvs-target-wsl";
/** The fuzz workspace's own build, outside the copy that each sync cleans. */
const FUZZ_TARGET = "/var/tmp/nvs-target-wsl-fuzz";
/** Each target's corpus, kept from one run to the next. */
const CORPUS = "/var/tmp/nvs-fuzz-corpus";
/** Each target's crash inputs. */
const ARTIFACTS = "/var/tmp/nvs-fuzz-artifacts";
/** A cold build and the longest run asked for. */
const TIMEOUT_MS = 4 * 60 * 60 * 1000;

/** What a command line asks for, or why it is not one this command takes. */
export function parsed(args: string[]): { target: string; limit: string } | string {
  const [target, flag, value, ...rest] = args;
  if (target === undefined || !/^[a-z_]+$/.test(target)) return "name one target, as `fuzz/Cargo.toml` names it";
  if (rest.length > 0 || value === undefined || !/^[1-9][0-9]*$/.test(value)) return "give --runs <n> or --seconds <n>";
  if (flag === "--runs") return { target, limit: `-runs=${value}` };
  if (flag === "--seconds") return { target, limit: `-max_total_time=${value}` };
  return "give --runs <n> or --seconds <n>";
}

/** The `bash` line that runs `target` in the copy at `mirror`, with `seeds` and `dict` when given. */
export function fuzzLine(mirror: string, target: string, limit: string, seeds: string, dict: string): string {
  const corpus = `${CORPUS}/${target}`;
  const artifacts = `${ARTIFACTS}/${target}/`;
  const dirs = [corpus, ...(seeds === "" ? [] : [seeds])].map(q).join(" ");
  const flags = [limit, `-artifact_prefix=${artifacts}`, ...(dict === "" ? [] : [`-dict=${dict}`])].map(q).join(" ");
  return (
    `mkdir -p ${q(corpus)} ${q(artifacts)} && cd ${q(mirror)} && ` +
    `cargo +nightly fuzz run --target-dir ${q(FUZZ_TARGET)} ${q(target)} ${dirs} -- ${flags}`
  );
}

export async function run(args: string[]): Promise<number> {
  if (args.length === 0 || args[0] === "--help" || args[0] === "-h") {
    console.log(`${USAGE}\n\n${summary}`);
    return args.length === 0 ? 2 : 0;
  }
  const asked = parsed(args);
  if (typeof asked === "string") {
    console.error(`${USAGE}\nnv fuzz: error: ${asked}`);
    return 2;
  }
  if (!existsSync(join(ROOT, "fuzz", "fuzz_targets", `${asked.target}.rs`))) {
    console.error(`nv fuzz: error: fuzz/fuzz_targets/${asked.target}.rs does not exist`);
    return 2;
  }
  if (process.platform !== "win32") {
    console.error("nv fuzz: this host is not Windows; run `cargo +nightly fuzz run` directly");
    return 2;
  }
  const seedName = asked.target === "ast" ? "parse" : asked.target;
  const seeds = existsSync(join(ROOT, "fuzz", "seeds", seedName)) ? `fuzz/seeds/${seedName}` : "";
  const dictName = asked.target === "markdown_comrak" ? "markdown" : asked.target;
  const dict = existsSync(join(ROOT, "fuzz", "dicts", `${dictName}.dict`)) ? `fuzz/dicts/${dictName}.dict` : "";
  const mirror = mirrorPath(WSL_TARGET, wslPath(ROOT));
  console.log(`fuzz: syncing the WSL copy of the tree at ${mirror}`);
  const synced = await syncMirror(mirror);
  if (synced !== "") {
    console.error(`nv fuzz: the WSL copy of the tree could not be synced: ${synced}`);
    return 2;
  }
  // `wsl.exe` hides the distro's CPU from the hang watch, and libFuzzer prints only at each new
  // coverage point, so the watch is off and the timeout bounds the run.
  const line = fuzzLine(mirror, asked.target, asked.limit, seeds, dict);
  return passthrough(["wsl.exe", "--exec", "bash", "-lc", line], { timeoutMs: TIMEOUT_MS, hang: false });
}
