// `bun nv fuzz`: one `cargo fuzz run` of one target under `fuzz/`, or of every target CI fuzzes,
// inside WSL.
//
//     bun nv fuzz markdown --runs 1000000        # a million inputs, then stop
//     bun nv fuzz markdown_from_html --seconds 300
//     bun nv fuzz --all                          # every target CI fuzzes, and go on after a crash
//
// libFuzzer does not run on Windows, so a target runs in WSL with the distro's nightly toolchain and
// `cargo-fuzz`. This command brings the WSL leg's copy of the tree up to date (`accept/mirror.ts`
// `syncMirror`) and runs the target there, as CI's `fuzz-smoke` job runs it: the target's seeds from
// `fuzz/seeds/<target>/` (`parse`'s for `ast`) and its dictionary from `fuzz/dicts/<target>.dict`
// (`markdown`'s for `markdown_comrak`) when it has one.
//
// The target runs in libFuzzer's fork mode, on half the distro's cores but no more jobs than it has free
// GiB (`JOBS`), so a run takes about its time on one core divided by the jobs. `-runs`
// and `-max_total_time` bound the whole run, not each job. Fork mode would skip an input that times out
// or runs out of memory, so `-ignore_timeouts=0 -ignore_ooms=0` stop on one as a single process does.
// A crash stops every job and prints the failing job's log.
//
// **`--all` finds every crash it can in one run, so all of them are fixed in one pass.** It fuzzes each
// target the `fuzz-smoke` job's legs list in `ci.yml` (`ciTargets`), one after another, with
// `-ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1`: a job that crashes is replaced and the run goes
// on, and each input that crashed, hung or ran out of memory is saved. A target runs `ALL_SECONDS`, about
// three times the work of CI's 300 seconds on one core. A target whose file git added under
// `YOUNG_DAYS` ago runs `YOUNG_SECONDS`, because a young target finds a new bug every few minutes. At the
// end it lists every input the run saved, each with its bytes in base64, and exits 1 when it saved one
// or a target's run failed. It is `bun nv deep`'s `fuzz-smoke` step.
//
// The sync runs `git clean -ffdx` in the copy, so nothing the run keeps lives there. The build goes to
// `FUZZ_TARGET`, the corpus a run grows to `CORPUS/<target>/`, and a crash's input to
// `ARTIFACTS/<target>/`, all on the distro's own disk. The corpus carries over from one run to the next.
// A crash prints the input's path there and its bytes in base64, so the input can be copied into a
// test. Its output is libFuzzer's and its exit status is cargo-fuzz's.
//
// Windows only, since on Linux `cargo +nightly fuzz run` is the same run. It exits 2 on a usage error or
// when the copy cannot be synced. What it spends: the first run builds the fuzz workspace into
// `FUZZ_TARGET`, which takes minutes cold, the corpus grows with each run, and each job holds up to
// libFuzzer's 2 GiB `-rss_limit_mb`, which is why the free memory caps the jobs. `--all` takes 120
// seconds per target and 30 minutes per young one: about half an hour, plus an hour while two are young.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { mirrorPath, q, syncMirror, wslPath } from "../accept/mirror.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough, run as runProc } from "../lib/proc.ts";
import { wslArgv } from "../lib/wsl.ts";

export const summary =
  "runs one fuzz target in WSL, or every target CI fuzzes: nv fuzz <target> --runs <n> | --seconds <n>, or nv fuzz --all";

const USAGE = "usage: bun nv fuzz <target> (--runs <n> | --seconds <n>) | bun nv fuzz --all";

/** The tree copy `linux-test` syncs, named by its target directory. */
const WSL_TARGET = "/var/tmp/nvs-target-wsl";
/** The fuzz workspace's own build, outside the copy that each sync cleans. */
const FUZZ_TARGET = "/var/tmp/nvs-target-wsl-fuzz";
/** Each target's corpus, kept from one run to the next. */
const CORPUS = "/var/tmp/nvs-fuzz-corpus";
/** Each target's crash inputs. */
const ARTIFACTS = "/var/tmp/nvs-fuzz-artifacts";
/** Touched when `--all` starts, so the inputs it saved are the ones newer than it. */
const STAMP = "/var/tmp/nvs-fuzz-all.stamp";
/** Seconds each target runs under `--all`. */
export const ALL_SECONDS = 120;
/** Seconds a young target runs under `--all`. */
export const YOUNG_SECONDS = 1800;
/** A target git added fewer days ago than this is young. */
export const YOUNG_DAYS = 14;
/**
 * The `bash` that sets `j` to the fork mode's job count: half the cores, at most one per GiB the
 * distro has free, and at least one.
 */
const JOBS =
  "j=$(( $(nproc) / 2 )); m=$(( $(awk '/^MemAvailable:/ {print $2}' /proc/meminfo) / 1048576 )); " +
  "[ \"$m\" -lt \"$j\" ] && j=$m; [ \"$j\" -lt 1 ] && j=1; echo \"fuzz: $j job(s)\"; ";
/** A cold build and the longest run asked for. */
const TIMEOUT_MS = 4 * 60 * 60 * 1000;

/** What a command line asks for, or why it is not one this command takes. */
export function parsed(args: string[]): { target: string; limit: string } | { all: true } | string {
  if (args.length === 1 && args[0] === "--all") return { all: true };
  if (args.includes("--all")) return "--all takes no other argument";
  const [target, flag, value, ...rest] = args;
  if (target === undefined || !/^[a-z_]+$/.test(target)) return "name one target, as `fuzz/Cargo.toml` names it";
  if (rest.length > 0 || value === undefined || !/^[1-9][0-9]*$/.test(value)) return "give --runs <n> or --seconds <n>";
  if (flag === "--runs") return { target, limit: `-runs=${value}` };
  if (flag === "--seconds") return { target, limit: `-max_total_time=${value}` };
  return "give --runs <n> or --seconds <n>";
}

/** The targets the `fuzz-smoke` job's legs list in the text of `ci.yml`, in their order. */
export function ciTargets(ciYml: string): string[] {
  const yml = Bun.YAML.parse(ciYml) as {
    jobs?: Record<string, { strategy?: { matrix?: { include?: { targets?: string[] }[] } } }>;
  };
  return (yml.jobs?.["fuzz-smoke"]?.strategy?.matrix?.include ?? []).flatMap((leg) => leg.targets ?? []);
}

/** The seconds `--all` runs a target that git added at `addedSec`, at `nowSec`; an untracked one is young. */
export function allSeconds(addedSec: number | null, nowSec: number): number {
  if (addedSec === null) return YOUNG_SECONDS;
  return nowSec - addedSec < YOUNG_DAYS * 24 * 60 * 60 ? YOUNG_SECONDS : ALL_SECONDS;
}

/** When git first added `fuzz/fuzz_targets/<target>.rs`, in seconds, or null when it has not. */
function addedAt(target: string): number | null {
  const out = Bun.spawnSync(["git", "log", "--diff-filter=A", "--format=%ct", "--", `fuzz/fuzz_targets/${target}.rs`], { cwd: ROOT });
  const oldest = out.stdout.toString().trim().split("\n").pop() ?? "";
  return /^[0-9]+$/.test(oldest) ? Number(oldest) : null;
}

/** The `bash` line that runs `target` in the copy at `mirror`, with `seeds` and `dict` when given. With
 * `keepGoing`, a crash, a timeout or an out-of-memory input is saved and the run goes on. */
export function fuzzLine(mirror: string, target: string, limit: string, seeds: string, dict: string, keepGoing = false): string {
  const corpus = `${CORPUS}/${target}`;
  const artifacts = `${ARTIFACTS}/${target}/`;
  const dirs = [corpus, ...(seeds === "" ? [] : [seeds])].map(q).join(" ");
  const ignore = keepGoing
    ? ["-ignore_crashes=1", "-ignore_timeouts=1", "-ignore_ooms=1"]
    : ["-ignore_timeouts=0", "-ignore_ooms=0"];
  const fixed = [limit, ...ignore, `-artifact_prefix=${artifacts}`];
  const flags = [...fixed, ...(dict === "" ? [] : [`-dict=${dict}`])].map(q).join(" ");
  return (
    `${JOBS}mkdir -p ${q(corpus)} ${q(artifacts)} && cd ${q(mirror)} && ` +
    `cargo +nightly fuzz run --target-dir ${q(FUZZ_TARGET)} ${q(target)} ${dirs} -- -fork="$j" ${flags}`
  );
}

/** The `bash` line that prints each input saved since `STAMP` as `saved: <path>` and then its base64. */
export function savedLine(): string {
  return (
    `mkdir -p ${ARTIFACTS} && find ${ARTIFACTS} -type f -newer ${STAMP} | sort | ` +
    `while IFS= read -r f; do echo "saved: $f"; base64 -w0 "$f"; echo; done`
  );
}

/** The seeds directory and the dictionary of `target`, each "" when it has none. */
function inputsOf(target: string): { seeds: string; dict: string } {
  const seedName = target === "ast" ? "parse" : target;
  const seeds = existsSync(join(ROOT, "fuzz", "seeds", seedName)) ? `fuzz/seeds/${seedName}` : "";
  const dictName = target === "markdown_comrak" ? "markdown" : target;
  const dict = existsSync(join(ROOT, "fuzz", "dicts", `${dictName}.dict`)) ? `fuzz/dicts/${dictName}.dict` : "";
  return { seeds, dict };
}

const wsl = (line: string): string[] => wslArgv(line);

/** Fuzzes every target CI fuzzes in the copy at `mirror`, then lists what the run saved. */
async function runAll(mirror: string): Promise<number> {
  const targets = ciTargets(readFileSync(join(ROOT, ".github", "workflows", "ci.yml"), "utf8"));
  if (targets.length === 0) {
    console.error("nv fuzz: error: ci.yml's fuzz-smoke job lists no target");
    return 2;
  }
  const stamped = await runProc(wsl(`touch ${STAMP}`), { timeoutMs: 60_000, hang: false });
  if (stamped.code !== 0) {
    console.error(`nv fuzz: error: cannot touch ${STAMP} in WSL: ${stamped.stderr.trim()}`);
    return 2;
  }
  const now = Math.floor(Date.now() / 1000);
  const failed: string[] = [];
  for (const target of targets) {
    const seconds = allSeconds(addedAt(target), now);
    const { seeds, dict } = inputsOf(target);
    console.log(`fuzz: ${target} for ${seconds}s, going on after a crash`);
    const line = fuzzLine(mirror, target, `-max_total_time=${seconds}`, seeds, dict, true);
    const code = await passthrough(wsl(line), { timeoutMs: TIMEOUT_MS, hang: false });
    if (code !== 0) failed.push(`${target} (exit ${code})`);
  }
  const listed = await runProc(wsl(savedLine()), { timeoutMs: 5 * 60_000, hang: false });
  const saved = listed.stdout.split("\n").filter((l) => l.startsWith("saved: ")).length;
  console.log(`\nfuzz --all: ${targets.length} target(s), ${saved} saved input(s), ${failed.length} failed run(s)`);
  if (listed.stdout.trim() !== "") console.log(listed.stdout.trimEnd());
  for (const f of failed) console.log(`failed: ${f}`);
  return saved === 0 && failed.length === 0 && listed.code === 0 ? 0 : 1;
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
  if ("target" in asked && !existsSync(join(ROOT, "fuzz", "fuzz_targets", `${asked.target}.rs`))) {
    console.error(`nv fuzz: error: fuzz/fuzz_targets/${asked.target}.rs does not exist`);
    return 2;
  }
  if (process.platform !== "win32") {
    console.error("nv fuzz: this host is not Windows; run `cargo +nightly fuzz run` directly");
    return 2;
  }
  const mirror = mirrorPath(WSL_TARGET, wslPath(ROOT));
  console.log(`fuzz: syncing the WSL copy of the tree at ${mirror}`);
  const synced = await syncMirror(mirror);
  if (synced !== "") {
    console.error(`nv fuzz: the WSL copy of the tree could not be synced: ${synced}`);
    return 2;
  }
  if ("all" in asked) return runAll(mirror);
  // `wsl.exe` hides the distro's CPU from the hang watch, and libFuzzer prints only at each new
  // coverage point, so the watch is off and the timeout bounds the run.
  const { seeds, dict } = inputsOf(asked.target);
  const line = fuzzLine(mirror, asked.target, asked.limit, seeds, dict);
  return passthrough(wsl(line), { timeoutMs: TIMEOUT_MS, hang: false });
}
