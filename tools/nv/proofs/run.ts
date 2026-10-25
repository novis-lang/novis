// Running the proofs on disk. An example runs and its standard output is compared with the `.out` frozen
// beside it, or its diagnostic when it declares `proof: compile-error`, or its `nvs test` report when it
// declares `proof: test` (`exampleOutput`). An attack runs and passes when the runtime survived it, whatever the program's own fate.
// `bun nv proofs --run` and `--verify` call this, over any number of groups in one pass: the files of
// every group go into one pool, and each group's verdict lines are counted from its own files. A program
// that runs out of time in the pool runs once more alone before its verdict stands (`timedOut`).
//
// A verdict is taken on the proof binary, `target/proof/`, the optimized `proof` profile built without
// coverage counters: counters slow optimized code several times over, and an attack's time limit is
// written for the binary a person ships. `proofBinary` runs cargo every time, and cargo rebuilds when
// what the binary is built from changed. `releaseBinary` is `target/release/`, which the perf ledger
// measures on, and it too is cargo's to bring up to date: a build that has nothing to do costs cargo's
// own look at its fingerprints. Both hand back a pinned copy of the binary (`lib/relink.ts`), so a build
// another process makes while the programs run does not move the file they run.
//
// Which programs run is the selection's (`tools/nv/select/`), and `cmd/proofs.ts` makes it; this file
// runs every program it is handed. What a program used is recorded in a second run of it, on the covws
// debug `nvs` (`recordingRun`), whose outcome is never a verdict. With `NV_PROOF_RECORD=<dir>` in the
// environment each run is recorded into that directory under the program's `recordName`:
// `proofRecording` owns what the program's processes are told. Without it, an instrumented binary's
// counters go to `DISCARD_PROFILE`, never into the working directory.
//
// A run over whole groups records each group's example and attack directories and bench file in the
// selection store (`proofReadsSlot`), and a run over named features each feature's (`featureGroup`),
// which is how `select/checks.ts` knows which proof programs a `bun nv proofs --group <group>` or
// `--only <feature>...` check is made of.

import { existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { digest } from "../keys/scan.ts";
import { abs, DISCARD_PROFILE, ROOT } from "../lib/paths.ts";
import { Tree } from "../lib/proctree.ts";
import { run as runProc } from "../lib/proc.ts";
import { inPart } from "../lib/reads.ts";
import { record } from "../lib/written.ts";
import { proofReadsSlot, SelectStore } from "../select/store.ts";
import { cargoLines, progress } from "../lib/progress.ts";
import { linked, pinned, RELEASE_ARGV } from "../lib/relink.ts";
import { gapTitle, knownGap } from "./collect.ts";
import { read } from "./roster.ts";

export type What = "examples" | "hostile";
export type Verdict = "ok" | "skip" | "known" | "fail";

export interface Binary {
  path: string;
  /** Which build the binary is, or the bytes of a binary named by hand. */
  key: string;
  /** The same, for what a run of it depends on. */
  runs: string;
  /** Whether the selection picks and records the programs judged on it: the proof binary's are, and a
   * binary named by hand runs every program and records nothing. */
  recorded?: boolean;
}

const EXE = process.platform === "win32" ? "nvs.exe" : "nvs";
/** The proof binary, repo-relative: the `proof` profile, built without coverage counters. */
export const PROOF_BINARY = `target/proof/${EXE}`;
export const RELEASE_BINARY = `target/release/${EXE}`;

/** The two optimized builds. The release build is the argv the acceptance sweep runs, so both share one
 * set of artefacts under `target/release/`. */
export const PROOF_BUILD = { name: "proof binary", key: "proof", path: PROOF_BINARY, argv: ["cargo", "build", "--profile", "proof", "--bin", "nvs"] };
const RELEASE = { name: "release binary", key: "release", path: RELEASE_BINARY, argv: RELEASE_ARGV };

/** How many times its own limit a recording run may take before it counts as hung: the covws debug
 * build is unoptimized and instrumented, so it is many times slower than the binary a verdict is
 * taken on. The same factor the valgrind run gets. */
export const HANG_FACTOR = 20;

/** `// requires: unimplemented`, the website's own skip marker. */
const UNIMPL_RE = /^(?:\/\/|#)\s*requires:\s*unimplemented/m;
/** `// requires: unix`: the program needs what this build has only on Unix — a Unix-domain socket, or a load average. */
const UNIX_RE = /^(?:\/\/|#)\s*requires:\s*unix\b/m;

/** `// requires: samba-ad`: the program talks to the Samba AD directory in `tests/db/compose.yaml`, and
 * runs in one of the `SAMBA_AD_JOBS` slots. */
const SAMBA_AD_RE = /^(?:\/\/|#)\s*requires:\s*samba-ad\b/m;
/** Every `// requires: <name>` line, with its name. */
const REQUIRES_RE = /^(?:\/\/|#)\s*requires:\s*(\S+)/gm;
/** The `requires:` names that say where a program runs, and name no service. */
const HOST_MARKERS = new Set(["unimplemented", "unix"]);
export const COMPOSE = "tests/db/compose.yaml";
/** A cold Samba provisions its domain before it reports healthy, the slowest service to come up. */
const SERVICE_UP_TIMEOUT_MS = 600_000;
/** Each service's `up -d --wait`, started once per process by the first program that names it: null
 * when it came up healthy, and the failure otherwise, which every program that names it then carries. */
const servicesUp = new Map<string, Promise<string | null>>();
let declared: Set<string> | null = null;

/** The services `tests/db/compose.yaml` declares, read once per process; empty when it does not parse. */
export function composeServices(): Set<string> {
  if (declared) return declared;
  try {
    const doc = Bun.YAML.parse(readFileSync(abs(COMPOSE), "utf8")) as { services?: Record<string, unknown> } | null;
    declared = new Set(Object.keys(doc?.services ?? {}));
  } catch {
    declared = new Set();
  }
  return declared;
}

/** The services `source`'s `requires:` lines name, in order and once each. */
export function requiredServices(source: string): string[] {
  const names = [...source.matchAll(REQUIRES_RE)].map((m) => m[1]!).filter((n) => !HOST_MARKERS.has(n));
  return [...new Set(names)];
}

/** Why this host runs none of the compose file's containers, read from `docker info --format
 * {{.OSType}}`: null when it runs them. `info` is null when no `docker` program could be started. A
 * Docker that is installed and not answering is a reason too, with the last line `info` wrote: that is
 * how a hosted Windows runner's Docker program meets a request for Linux containers, and one rule on
 * every platform makes it a skip there and on a developer's machine alike. */
export function noDockerReason(info: { code: number; stdout: string; stderr: string } | null): string | null {
  if (info === null) return "this host has no Docker";
  const os = info.stdout.trim();
  if (info.code === 0 && os === "linux") return null;
  if (info.code !== 0 || os === "") {
    const said = info.stderr.trim().split(/\r?\n/).at(-1) ?? "";
    return `this host's Docker is not answering (\`docker info\` exit ${info.code}${said ? `: ${said}` : ""})`;
  }
  return `this host's Docker runs ${os} containers, and ${COMPOSE} has Linux ones`;
}

/** `noDockerReason` for this host, probed once per process by the first program that names a service. */
let dockerProbe: Promise<string | null> | null = null;
export function dockerAbsent(): Promise<string | null> {
  dockerProbe ??= runProc(["docker", "info", "--format", "{{.OSType}}"], { cwd: ROOT, timeoutMs: 60_000 }).then(
    (r) => noDockerReason(r),
    () => noDockerReason(null),
  );
  return dockerProbe;
}

/**
 * Why a program is not run on this host because of the services it names, or null when it is run.
 * **One rule on every platform:** a program marked `requires: <service>` runs where Docker runs the
 * compose file's Linux containers, and is skipped with the reason everywhere else. A hosted macOS
 * runner has no Docker and a hosted Windows one runs Windows containers, so those programs run on the
 * Linux leg alone. A name the compose file does not declare is never a skip: `servicesFor` fails it.
 */
export async function serviceSkipReason(source: string, services: Set<string> = composeServices()): Promise<string | null> {
  const wanted = requiredServices(source);
  if (wanted.length === 0 || wanted.some((s) => !services.has(s))) return null;
  const absent = await dockerAbsent();
  return absent ? `marked \`requires: ${wanted[0]}\`, and ${absent}` : null;
}

/** The CA the compose file's `certs` service issued, at the path the repository's `nvs.toml` names. */
export const COMPOSE_CA = "tests/db/ca.crt";
/** `syncComposeCa`, run once per process, after the first service that came up. */
let caSynced: Promise<string | null> | null = null;

/**
 * Copies the CA out of the compose stack's `certs` volume to `COMPOSE_CA`, so a program that talks to
 * a compose service verifies it against the anchor that signed it. Every service mounts that volume, so
 * the first one up is where it is read. Null when the file now holds that CA, and the failure otherwise.
 * The file is replaced in one rename and only when it differs, because programs that read it may be
 * running beside this one.
 */
function syncComposeCa(service: string): Promise<string | null> {
  caSynced ??= (async () => {
    const tmp = `.agent-tmp/compose-ca-${process.pid}.crt`;
    mkdirSync(abs(".agent-tmp"), { recursive: true });
    const argv = ["docker", "compose", "-f", COMPOSE, "cp", `${service}:/certs/ca.crt`, tmp];
    try {
      const r = await runProc(argv, { cwd: ROOT, timeoutMs: 60_000 });
      if (r.code !== 0) return `\`${argv.join(" ")}\` failed (exit ${r.code}): ${r.stderr.trim().split(/\r?\n/).at(-1) ?? ""}`;
      const fresh = readFileSync(abs(tmp));
      const dest = abs(COMPOSE_CA);
      if (!existsSync(dest) || !readFileSync(dest).equals(fresh)) renameSync(abs(tmp), dest);
      return null;
    } finally {
      rmSync(abs(tmp), { force: true });
    }
  })();
  return caSynced;
}

/** Brings up each `tests/db/compose.yaml` service `source` names in a `requires:` line, and says why it
 * could not; null when it needs none or every one it needs is up. A name the compose file does not
 * declare is a failure, never a line read past. A program without a `requires:` service line never
 * touches Docker. Once a service is up, `COMPOSE_CA` holds the CA that signed its certificate
 * (`syncComposeCa`). `services` is what the compose file declares. */
export async function servicesFor(source: string, services: Set<string> = composeServices()): Promise<string | null> {
  const wanted = requiredServices(source);
  const unknown = wanted.filter((s) => !services.has(s));
  if (unknown.length > 0) return `is marked \`requires: ${unknown[0]}\`, which is no service in ${COMPOSE} (it declares ${[...services].sort().join(", ")})`;
  if (wanted.length === 0) return null;
  const absent = await dockerAbsent();
  if (absent) return `needs \`${wanted[0]}\`, and ${absent}`;
  for (const service of wanted) {
    let up = servicesUp.get(service);
    if (!up) {
      progress(`proofs: bringing up \`${service}\` (a cold first boot can take half a minute)`);
      const argv = ["docker", "compose", "-f", COMPOSE, "up", "-d", "--wait", service];
      up = runProc(argv, { cwd: ROOT, timeoutMs: SERVICE_UP_TIMEOUT_MS }).then(
        async (r) => {
          if (r.code !== 0) return `needs \`${service}\`, and \`${argv.join(" ")}\` failed (exit ${r.code}): ${r.stderr.trim().split(/\r?\n/).at(-1) ?? ""}`;
          const ca = await syncComposeCa(service);
          return ca ? `needs \`${service}\`, and its CA could not be copied to ${COMPOSE_CA}: ${ca}` : null;
        },
        (e) => `needs \`${service}\`, and Docker could not be started: ${e instanceof Error ? e.message : String(e)}`,
      );
      servicesUp.set(service, up);
    }
    const why = await up;
    if (why) return why;
  }
  return null;
}

/** Why a program is not run on this host, or null when it is. A skip is never remembered as green. */
export function skipReason(source: string): string | null {
  if (UNIMPL_RE.test(source)) return "marked `requires: unimplemented`";
  if (UNIX_RE.test(source) && process.platform === "win32") return "marked `requires: unix`, and this host is Windows";
  return null;
}
/** `// hostile: timeout-ms 4000`. */
const TIMEOUT_RE = /(?:\/\/|#)\s*hostile:\s*timeout-ms\s+([0-9]+)/;
/** `// hostile: expect-refusal`: this attack passes when the compiler refuses it. */
const REFUSAL_EXPECTED_RE = /(?:\/\/|#)\s*hostile:\s*expect-refusal/;
/** `// hostile: ends-early 4 FATAL`: this attack's last step, 4, ends the program with `FATAL`. */
const ENDS_EARLY_RE = /(?:\/\/|#)\s*hostile:\s*ends-early\b([^\r\n]*)/;
/** A step comment: `// 4.`, `/// 4.` above a method, or `// Step 4:`. */
const STEP_RE = /^\s*\/\/\/?\s*(?:Step\s+)?([0-9]+)[.:](?:\s|$)/i;
/** What an attack may end with: `FATAL` for a limit, `exit` for a non-zero `exit`, or a thrown class. */
const ENDING_RE = /^(?:FATAL|exit|[A-Za-z_][A-Za-z0-9_]*(?:\\[A-Za-z_][A-Za-z0-9_]*)*)$/;
/** `// proof: exit 1` in an example: the exact status it ends with. */
const EXAMPLE_EXIT_RE = /(?:\/\/|#)\s*proof:\s*exit\s+([0-9]+)/;
/** `// proof: compile-error` in an example: it shows code that does not compile, so the run must end
 * with `nvs`'s compile-error status and a diagnostic, and its `.out` is that diagnostic. */
const EXAMPLE_COMPILE_ERROR_RE = /(?:\/\/|#)\s*proof:\s*compile-error\b/;
/** `// proof: test` in an example: it shows the report `nvs test` prints for it, so it runs under
 * `nvs test`, and its `.out` is that report with every duration removed (`testReport`). An attack
 * that declares it runs under `nvs test` as well, because `nvs run` calls no test (`judgeHostile`). */
const EXAMPLE_TEST_RE = /(?:\/\/|#)\s*proof:\s*test\b/;
/** `// proof: test --mutate`: a `proof: test` program run under `nvs test --mutate`, whose report ends
 * with the mutation counts (`MUTATION_SUMMARY_RE`). */
const EXAMPLE_MUTATE_RE = /(?:\/\/|#)\s*proof:\s*test\s+--mutate\b/;
/** `// proof: test --bench`: a `proof: test` program run under `nvs test --bench`, which runs its
 * `#[Bench]` methods and no test. Its `.out` keeps each bench's counted row and drops the advisory
 * wall time beside it (`testReport`), since only the counts are the same on every run. */
const EXAMPLE_BENCH_RE = /(?:\/\/|#)\s*proof:\s*test\s+--bench\b/;
/** The counts line a `--mutate` report writes once every mutant has run. */
const MUTATION_SUMMARY_RE = /^\s*mutants: [0-9]+ killed, [0-9]+ survived, [0-9]+ timed out, [0-9]+ not reached\r?$/m;
/** The last line of an `nvs test` report, which is written once every test has run. */
const TEST_SUMMARY_RE = /^\s*[0-9]+ failed, [0-9]+ passed, [0-9]+ skipped, [0-9]+ flaky\b/m;
/** The status `nvs test` ends with when a test failed. */
const TEST_FAILED_EXIT = 1;
/** The status `nvs run` ends with when the program does not compile. */
const COMPILE_ERROR_EXIT = 1;
/** A compile diagnostic. An uncaught throw is a log line and does not look like this. */
const REFUSED_RE = /^error\[E[0-9]+\]/m;
/** What an attack's output must never carry: the runtime under the program came apart. */
const CRASH_MARKERS = [
  "panicked at",
  "internal error",
  "RUST_BACKTRACE",
  "note: run with",
  "double free",
  "Assertion failed",
  "AddressSanitizer",
  "SIGSEGV",
  "stack overflow",
];
const EXAMPLE_TIMEOUT_MS = 60_000;
export const HOSTILE_TIMEOUT_MS = 10_000;

/** The proof binary of the tree as it stands, uninstrumented, which verdicts are taken on. A string is
 * why there is none. */
export const proofBinary = () => builtBinary(PROOF_BUILD, true);

/** The release binary of the tree as it stands, which `--record-perf` measures on. */
export const releaseBinary = () => builtBinary(RELEASE, false);

async function builtBinary(build: typeof RELEASE, recorded: boolean): Promise<Binary | string> {
  progress(`proofs: bringing the ${build.name} up to date`);
  const path = abs(build.path);
  const command = build.argv.join(" ");
  const onLine = cargoLines(`proofs: building the ${build.name}`);
  const built = await linked(path, () => runProc(build.argv, { timeoutMs: 60 * 60 * 1000, onLine }), (r) => r.stderr);
  if (built.code !== 0 || !existsSync(path)) {
    const tail = built.stderr.trimEnd().split("\n").slice(-15).join("\n");
    return `\`${command}\` failed (exit ${built.code}):\n${tail}`;
  }
  return { path: pinned(path), key: build.key, runs: build.key, ...(recorded ? { recorded } : {}) };
}

/** A binary named with `--nvs`, taken as it is and remembered by its bytes. */
export function namedBinary(path: string): Binary {
  const key = `bytes:${digest(readFileSync(path))}`;
  return { path, key, runs: key };
}

export interface Ran {
  code: number;
  stdout: string;
  stderr: string;
  timedOut: boolean;
  ms: number;
  /** The binary that ran, `argv[0]`: its default data folder is `.nvsdata` beside it. A verdict kept
   * from before the field existed has none. */
  exe?: string;
}

const sibling = (proof: string, suffix: string) => proof.replace(/\.(nvs|cmd)$/, suffix);

/** Where a `.cmd` example runs: a directory of its own under this one, made for the run and deleted after it. */
const COMMAND_SCRATCH = join(ROOT, ".agent-tmp", "proof-cmd");

/** A word before `nvs` that sets a variable for the command, as a shell reads `NAME=value`. */
const ASSIGNMENT_RE = /^([A-Z_][A-Z0-9_]*)=(.*)$/s;

/** The variables a `.cmd` example's command never inherits, because each changes what `nvs` prints
 * and a CI machine sets the first two. A line that needs one sets it before `nvs`. */
export const COMMAND_UNSET = ["CI", "DO_NOT_TRACK", "NVS_TELEMETRY_URL", "NVS_UPDATE_URL"];

/**
 * The variables and the arguments of a `.cmd` example: each `NAME=value` word before the `nvs` its one
 * command line runs, and every word after it. A word in single or double quotes may hold spaces, and the
 * quotes are not part of it. Throws when the file is not one `nvs` command line.
 */
export function commandLine(proof: string, source: string): { env: Record<string, string>; args: string[] } {
  const lines = source.split(/\r?\n/).filter((l) => l.trim());
  if (lines.length !== 1) throw new Error(`${proof} has ${lines.length} lines, and a .cmd example is one \`nvs\` command line`);
  const words = [...lines[0]!.matchAll(/"([^"]*)"|'([^']*)'|(\S+)/g)].map((m) => m[1] ?? m[2] ?? m[3]!);
  const env: Record<string, string> = {};
  let at = 0;
  for (let m: RegExpMatchArray | null; at < words.length && (m = words[at]!.match(ASSIGNMENT_RE)); at++) env[m[1]!] = m[2]!;
  if (words[at] !== "nvs") throw new Error(`${proof} starts with \`${words[at]}\`, and a .cmd example runs \`nvs\``);
  return { env, args: words.slice(at + 1) };
}

/** The arguments of a `.cmd` example, after the `nvs` its one command line runs (`commandLine`). */
export function commandArgs(proof: string, source: string): string[] {
  return commandLine(proof, source).args;
}

/** The environment variable naming the directory each proof program's run is recorded into. */
export const RECORD_ENV = "NV_PROOF_RECORD";

/**
 * The file name a recorded program's files start with, from its repo-relative path: `/` and `\` become
 * `~`, and every other byte that is not an ASCII letter, digit, `.`, `_` or `-` becomes `@` and two hex
 * digits. A name longer than 96 bytes keeps its first 64 and ends in `@` and the 64-bit FNV-1a hash of
 * the whole name as sixteen hex digits, so a profile's path stays inside Windows' 260-character limit.
 * The same rule as `nvs test --record`'s (`nvs_test::record_name`), so one reader serves both.
 */
export function recordName(path: string): string {
  let name = "";
  for (const byte of new TextEncoder().encode(path)) {
    const c = String.fromCharCode(byte);
    if (c === "/" || c === "\\") name += "~";
    else if (/[A-Za-z0-9._-]/.test(c)) name += c;
    else name += `@${byte.toString(16).padStart(2, "0")}`;
  }
  if (name.length <= 96) return name;
  let hash = 0xcbf29ce484222325n;
  for (let i = 0; i < name.length; i++) {
    hash ^= BigInt(name.charCodeAt(i));
    hash = (hash * 0x100000001b3n) & 0xffffffffffffffffn;
  }
  return `${name.slice(0, 64)}@${hash.toString(16).padStart(16, "0")}`;
}

/**
 * The variables every process of one run of `proof` gets when `dir` names a record directory:
 * `LLVM_PROFILE_FILE` as `<dir>/<name>-%p.profraw`, one file per process, `NVS_FOOTPRINT_LOG` as
 * `<dir>/<name>.log`, which they share, and `NOVIS_NO_FILE_CACHE`, so the compile runs whole. A
 * relative `dir` is taken from the repository root. Empty when nothing is recorded.
 */
export function proofRecording(proof: string, dir: string | undefined = process.env[RECORD_ENV]): Record<string, string> {
  if (!dir) return {};
  const at = resolve(ROOT, dir);
  const name = recordName(proof);
  return {
    LLVM_PROFILE_FILE: resolve(at, `${name}-%p.profraw`),
    NVS_FOOTPRINT_LOG: resolve(at, `${name}.log`),
    NOVIS_NO_FILE_CACHE: "1",
  };
}

/** One run of a proof program from the repository root. A proof with a `<name>.in` beside it reads that
 * file as its standard input, and every other proof reads nothing, so no proof ever waits on a terminal.
 * A proof with a `<name>.nvsr` beside it answers the request that file describes, handed over as
 * `--request` after `run`, so a `Core\Request` member has a request to read; every other proof answers
 * none, and a `Core\Request` member there throws. A proof with a `<name>.nvsp` beside it runs as a
 * WebSocket connection whose peer sends what that file lists, handed over as `--peer`, so a
 * `Core\Socket` member has a peer to talk to. A proof with a `<name>.nvse` beside it runs as an
 * event-stream connection that is published the values that file lists, handed over as `--events`, so
 * a `Core\Sse` member has a stream to wait on. A proof whose directory holds an `nvs.toml` runs
 * under that file alone, handed over as `--config` after `run`, so a feature that needs a grant carries
 * it where its reader sees it; every other proof runs under the repository root's. `record` names the
 * directory the run is recorded into, which defaults to `NV_PROOF_RECORD`, and variables its processes
 * get beside the recording's. With `unlogged` and no record directory, the run writes no footprint log,
 * not even the one this process inherited. A proof marked `requires: <service>` runs once that service
 * of `tests/db/compose.yaml` is up (`servicesFor`), and throws the reason when it cannot be brought up.
 * A `.cmd` proof runs the binary `argv[0]` names with its command line's arguments and variables
 * (`commandLine`), without `COMMAND_UNSET`, in an empty directory of its own, and none of the files above
 * but the `.in` is read for it. */
export async function spawnProof(argv: string[], proof: string, timeoutMs: number, record?: { dir?: string; env?: Record<string, string>; unlogged?: boolean }): Promise<Ran> {
  const source = existsSync(abs(proof)) ? readFileSync(abs(proof), "utf8") : "";
  const missing = await servicesFor(source);
  if (missing) throw new Error(missing);
  let scratch: string | null = null;
  let commandEnv: Record<string, string> | null = null;
  if (proof.endsWith(".cmd")) {
    const line = commandLine(proof, source);
    argv = [argv[0]!, ...line.args];
    commandEnv = line.env;
    mkdirSync(COMMAND_SCRATCH, { recursive: true });
    scratch = mkdtempSync(join(COMMAND_SCRATCH, "run-"));
  }
  const feed = abs(sibling(proof, ".in"));
  const request = sibling(proof, ".nvsr");
  const peer = sibling(proof, ".nvsp");
  const events = sibling(proof, ".nvse");
  const config = `${dirname(proof)}/nvs.toml`;
  const at = scratch ? -1 : argv.indexOf("run");
  if (at >= 0 && existsSync(abs(request))) argv = [...argv.slice(0, at + 1), "--request", request, ...argv.slice(at + 1)];
  if (at >= 0 && existsSync(abs(peer))) argv = [...argv.slice(0, at + 1), "--peer", peer, ...argv.slice(at + 1)];
  if (at >= 0 && existsSync(abs(events))) argv = [...argv.slice(0, at + 1), "--events", events, ...argv.slice(at + 1)];
  if (at >= 0 && existsSync(abs(config))) argv = [...argv.slice(0, at + 1), "--config", config, ...argv.slice(at + 1)];
  const recording = proofRecording(proof, record?.dir ?? process.env[RECORD_ENV]);
  if (recording.NVS_FOOTPRINT_LOG) mkdirSync(dirname(recording.NVS_FOOTPRINT_LOG), { recursive: true });
  const env: Record<string, string | undefined> = { LLVM_PROFILE_FILE: DISCARD_PROFILE, ...process.env, ...record?.env, ...recording };
  if (record?.unlogged && !recording.NVS_FOOTPRINT_LOG) delete env.NVS_FOOTPRINT_LOG;
  if (commandEnv) {
    for (const name of COMMAND_UNSET) delete env[name];
    Object.assign(env, commandEnv);
  }
  const release = SAMBA_AD_RE.test(source) ? await sambaSlot() : () => {};
  try {
    return await spawnTimed(argv, env, feed, timeoutMs, scratch ?? ROOT);
  } finally {
    release();
    if (scratch) rmSync(scratch, { recursive: true, force: true });
  }
}

/** How many programs marked `requires: samba-ad` run at once, whatever the pool's width. One Samba
 * directory answers all of them, and a full pool of them slows it past the `timeout` the proof blocks
 * in `nvs.toml` set, so a call or a TLS handshake fails and the same program ends differently each run. */
const SAMBA_AD_JOBS = 2;
let sambaRunning = 0;
const sambaWaiting: (() => void)[] = [];

/** Waits for one of `SAMBA_AD_JOBS` slots, and gives the function that frees it. A freed slot passes
 * straight to the next waiter. */
async function sambaSlot(): Promise<() => void> {
  if (sambaRunning < SAMBA_AD_JOBS) sambaRunning++;
  else await new Promise<void>((go) => sambaWaiting.push(go));
  return () => {
    const next = sambaWaiting.shift();
    if (next) next();
    else sambaRunning--;
  };
}

/** Runs `argv` once in `cwd` under `timeoutMs`, which starts when the process does. */
async function spawnTimed(argv: string[], env: Record<string, string | undefined>, feed: string, timeoutMs: number, cwd: string): Promise<Ran> {
  const started = performance.now();
  const child = Bun.spawn(argv, {
    cwd,
    env,
    stdin: existsSync(feed) ? Bun.file(feed) : "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const tree = new Tree(child);
  let timedOut = false;
  // Once the program has exited, the timeout reaches only what it left holding the pipes.
  const timer = setTimeout(() => {
    timedOut = true;
    tree.kill();
  }, timeoutMs);
  try {
    const reading = Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text()]);
    const exited = await child.exited;
    // What a failed or killed program left running is killed with it, so it holds no pipe and no file.
    if (exited !== 0 || timedOut) tree.reap();
    const [stdout, stderr] = await reading;
    // A program ended by a signal has no exit status of its own, and that is crash-shaped.
    const code = child.signalCode && !timedOut ? -1 : exited;
    return { code, stdout, stderr, timedOut, ms: performance.now() - started, exe: argv[0]! };
  } finally {
    clearTimeout(timer);
    tree.close();
  }
}

const normalise = (text: string) => text.replace(/\r\n/g, "\n").trimEnd();

/** The line of a `.cmd` example's `.out` that matches any number of lines, none included. */
export const ELISION = "...";

/** The `.out` of the `.cmd` example `path` when it has an `ELISION` line, and null otherwise. */
export function elidedOut(path: string, root: string = ROOT): string | null {
  const out = abs(sibling(path, ".out"), root);
  if (!path.endsWith(".cmd") || !existsSync(out)) return null;
  const text = readFileSync(out, "utf8");
  return text.split(/\r?\n/).includes(ELISION) ? text : null;
}

/**
 * Whether `stdout` is what the `.out` text `want` says, line endings and trailing blank space aside. In a
 * `.cmd` example's `.out`, a line that is exactly `ELISION` matches any run of lines, so a command whose
 * report differs per machine still has the lines that do not differ checked byte for byte.
 */
export function matchesOut(stdout: string, want: string, command: boolean): boolean {
  const got = normalise(stdout);
  const expected = normalise(want);
  if (!command || !expected.split("\n").includes(ELISION)) return got === expected;
  const have = got.split("\n");
  const pattern = expected.split("\n");
  // fits[j]: the pattern so far matches the first j lines of the output.
  let fits: boolean[] = new Array(have.length + 1).fill(false);
  fits[0] = true;
  for (const line of pattern) {
    const next: boolean[] = new Array(have.length + 1).fill(false);
    for (let j = 0; j <= have.length; j++) {
      if (line === ELISION) next[j] = fits[j]! || (j > 0 && next[j - 1]!);
      else next[j] = j > 0 && fits[j - 1]! && have[j - 1] === line;
    }
    fits = next;
  }
  return fits[have.length]!;
}

/** Where `stdout` first leaves `want`, as `matchesOut` reads both without elision: the line number and
 * the two lines, so a failure on a host nobody can log into says what the program printed. */
export function firstDifference(stdout: string, want: string): string {
  const got = normalise(stdout).split("\n");
  const expected = normalise(want).split("\n");
  let i = 0;
  while (i < got.length && i < expected.length && got[i] === expected[i]) i++;
  const line = (lines: string[]) => (i < lines.length ? `\`${lines[i]!.slice(0, 160)}\`` : "nothing");
  return `line ${i + 1} is ${line(got)} where the .out has ${line(expected)}`;
}

/** The status an example says it ends with, and 0 for every example that says nothing. */
function declaredExit(source: string): number {
  if (EXAMPLE_COMPILE_ERROR_RE.test(source)) return COMPILE_ERROR_EXIT;
  const m = EXAMPLE_EXIT_RE.exec(source);
  return m ? Number(m[1]) : 0;
}

/**
 * What an example's `.out` freezes: its standard output, or for a `proof: compile-error` example the
 * diagnostic on standard error, with the example's path shortened to its file name in either slash
 * form, so the `.out` reads the same on every host and on the website.
 */
export function exampleOutput(path: string, source: string, out: Pick<Ran, "stdout" | "stderr">): string {
  if (EXAMPLE_TEST_RE.test(source)) return testReport(out.stdout);
  if (!EXAMPLE_COMPILE_ERROR_RE.test(source)) return out.stdout;
  const name = basename(path);
  return out.stderr.replaceAll(path.replaceAll("\\", "/"), name).replaceAll(path.replaceAll("/", "\\"), name);
}

/** `nvs test`'s human report without its timings: the duration that ends each test's line, the
 * summary's ` in N ms` and a bench row's `, wall N ms (advisory)`, which differ on every run. */
export function testReport(stdout: string): string {
  return stdout
    .replace(/^(\s+[✓✗!-] .*?) +[0-9]+(?:\.[0-9]+)? ms(\r?)$/gmu, "$1$2")
    .replace(/^(\s+[0-9]+ failed, .* flaky) in [0-9]+ ms(\r?)$/gm, "$1$2")
    .replace(/^(\s+count: .*?), wall [0-9]+(?:\.[0-9]+)? ms \(advisory\)(\r?)$/gm, "$1$2");
}

/** The command line an example or an attack runs with: `nvs test` for a `proof: test` program, with
 * `--mutate` or `--bench` for a `proof: test --mutate` or `proof: test --bench` one, and `nvs run`
 * otherwise. */
export function proofArgv(nvs: string, path: string, source: string): string[] {
  if (EXAMPLE_MUTATE_RE.test(source)) return [nvs, "test", path, "--mutate"];
  if (EXAMPLE_BENCH_RE.test(source)) return [nvs, "test", path, "--bench"];
  return [nvs, EXAMPLE_TEST_RE.test(source) ? "test" : "run", path];
}

/** One judged run: the verdict, why, and what the program did, which a program skipped on this host
 * does not have. */
type Judged = [Verdict, string, Ran?];

/** The line of a failed run's standard error that says what stopped it: the first that is not a
 * `warning:`, since a warning such as the queue worker's names something the run went on past. The
 * first line when every line is a warning. */
export function stoppedBy(stderr: string): string {
  const lines = stderr.split(/\r?\n/).filter((l) => l.trim());
  return (lines.find((l) => !l.startsWith("warning:")) ?? lines[0] ?? "").trim();
}

/** An example passes when it ends with the status it declares and prints exactly its `.out`. */
async function runExample(nvs: string, path: string, unlogged = false): Promise<Judged> {
  const source = read(path);
  const skip = skipReason(source) ?? (await serviceSkipReason(source));
  if (skip) return ["skip", skip];
  let out: Ran;
  try {
    out = await spawnProof(proofArgv(nvs, path, source), path, EXAMPLE_TIMEOUT_MS, unlogged ? { unlogged } : undefined);
  } catch (e) {
    return ["fail", `could not run: ${e instanceof Error ? e.message : String(e)}`];
  }
  if (out.timedOut) return ["fail", `timed out after ${EXAMPLE_TIMEOUT_MS / 1000}s`, out];
  const want = declaredExit(source);
  if (out.code !== want) {
    const first = stoppedBy(out.stderr);
    const declared = EXAMPLE_COMPILE_ERROR_RE.test(source) ? "proof: compile-error" : `proof: exit ${want}`;
    return ["fail", want ? `exit ${out.code} where it declares \`${declared}\`` : `exit ${out.code}: ${first}`, out];
  }
  const compileError = EXAMPLE_COMPILE_ERROR_RE.test(source);
  if (compileError && !REFUSED_RE.test(out.stderr)) return ["fail", "it declares `proof: compile-error`, and stderr has no compile diagnostic", out];
  const expected = sibling(path, ".out");
  if (!existsSync(abs(expected))) return ["fail", `no ${basename(expected)} beside it`, out];
  const printed = exampleOutput(path, source, out);
  if (!matchesOut(printed, read(expected), path.endsWith(".cmd"))) {
    return ["fail", `${compileError ? "the diagnostic" : "stdout"} differs from its .out: ${firstDifference(printed, read(expected))}`, out];
  }
  return ["ok", "", out];
}

/**
 * `--bless`: writes each example's `.out` from what it prints, and prints what it wrote. This is how an
 * expected output is created, never how a red example is made green, which is why the output is shown
 * rather than written silently. An example that does not end with the status it declares gets no `.out`.
 * Returns the lines to print and whether any example failed.
 */
export async function bless(nvs: string, paths: string[]): Promise<{ lines: string[]; failed: boolean }> {
  const lines: string[] = [];
  let failed = false;
  for (const path of paths) {
    const skip = skipReason(read(path));
    if (skip) {
      // What a skipped program prints here is not what it prints where it runs.
      lines.push(`  FAIL  ${path} is ${skip}: bless it on a host that runs it`);
      failed = true;
      continue;
    }
    const missing = await servicesFor(read(path));
    if (missing) {
      lines.push(`  FAIL  ${path} ${missing}`);
      failed = true;
      continue;
    }
    const source = read(path);
    const out = await spawnProof(proofArgv(nvs, path, source), path, 120_000);
    if (out.code !== declaredExit(source) || (EXAMPLE_COMPILE_ERROR_RE.test(source) && !REFUSED_RE.test(out.stderr))) {
      lines.push(`  FAIL  ${path} exited ${out.code}:`, "        " + out.stderr.trim().replace(/\r?\n/g, "\n        "));
      failed = true;
      continue;
    }
    const dest = sibling(path, ".out");
    const existed = existsSync(abs(dest));
    // An elided `.out` is written by hand, so one that still matches is kept as it is.
    if (existed && path.endsWith(".cmd") && read(dest).split(/\r?\n/).includes(ELISION) && matchesOut(out.stdout, read(dest), true)) {
      lines.push(`  kept  ${dest}: its elided lines still match`);
      continue;
    }
    const text = exampleOutput(path, source, out).replace(/\r\n/g, "\n");
    writeFileSync(abs(dest), text);
    record(abs(dest));
    lines.push(`  ${existed ? "rewrote" : "wrote"}  ${dest}`);
    for (const line of text.trimEnd().split("\n")) lines.push(`      | ${line}`);
  }
  if (!failed) lines.push("nv proofs: read what was written -- a blessed output is a claim, not a formality.");
  return { lines, failed };
}

/** How far an attack got before its clock ran out, from the lines its steps printed. */
function reached(stdout: string): string {
  const done = stdout.split(/\r?\n/).filter((l) => l.trim());
  if (done.length === 0) return ", having printed nothing";
  return `, having printed ${done.length} line(s), the last ${JSON.stringify([...done.at(-1)!].slice(0, 60).join(""))}`;
}

/** An attack's time limit: its `timeout-ms`, or the default. */
export function hostileLimitMs(source: string): number {
  const m = TIMEOUT_RE.exec(source);
  return m ? Number(m[1]) : HOSTILE_TIMEOUT_MS;
}

/** The step an attack's `ends-early` names, and the ending it names for it. */
export interface Ending {
  step: number;
  ending: string;
}

/** Each step comment as its 1-based line and the number it carries, in file order. */
function stepComments(source: string): [number, number][] {
  const steps: [number, number][] = [];
  source.split("\n").forEach((text, i) => {
    const m = STEP_RE.exec(text);
    if (m) steps.push([i + 1, Number(m[1])]);
  });
  return steps;
}

/**
 * The ending an attack declares, null when it declares none, and what is wrong with the marker when
 * something is. The step must be the file's last; a file with no step comment is one attack, step `1`.
 */
export function declaredEnding(source: string): Ending | string | null {
  const m = ENDS_EARLY_RE.exec(source);
  if (!m) return null;
  const said = (m[1] ?? "").trim();
  const [number, ending, ...more] = said.split(/\s+/).filter(Boolean);
  if (number === undefined || ending === undefined || more.length > 0 || !/^[0-9]+$/.test(number) || !ENDING_RE.test(ending)) {
    return `\`ends-early\` must name its step and its ending, as \`// hostile: ends-early 4 FATAL\`, and it says \`${said}\``;
  }
  const step = Number(number);
  const last = Math.max(1, ...stepComments(source).map(([, n]) => n));
  if (step !== last) return `\`ends-early\` names step ${step}, which is not the file's last step, ${last}`;
  return { step, ending };
}

/**
 * How a program that stopped early ended, and the line of `path` the error report's outermost frame
 * names. The first report is the throw that ended the program; a hook that throws while it is reported
 * writes a report of its own after it.
 */
function observedEnding(out: Ran, path: string): { ending: string; line: number | null } {
  if (/^FATAL: /m.test(out.stderr)) return { ending: "FATAL", line: null };
  const own = path.replace(/\\/g, "/");
  for (const text of out.stderr.split(/\r?\n/)) {
    if (!text.startsWith("{")) continue;
    let report: { fields?: { class?: unknown }; nodes?: { file?: string; line?: number }[] };
    try {
      report = JSON.parse(text);
    } catch {
      continue;
    }
    const cls = report.fields?.class;
    if (typeof cls !== "string") continue;
    const frames = (report.nodes ?? []).filter((n) => typeof n.line === "number" && n.file?.replace(/\\/g, "/").endsWith(own));
    return { ending: cls, line: frames.at(-1)?.line ?? null };
  }
  // The plaintext rendering of the report carries the class as a field of its own.
  const plain = /^\s*class[=:]\s*"?([A-Za-z_][A-Za-z0-9_\\]*)/m.exec(out.stderr);
  return { ending: plain?.[1] ?? "exit", line: null };
}

/** The step whose comment is the last one at or above `line`, or null when none is. */
function stepAt(source: string, line: number): number | null {
  let step: number | null = null;
  for (const [at, n] of stepComments(source)) if (at <= line) step = n;
  return step;
}

/**
 * An attack passes when the runtime survives it. A throw, a limit, a clean fatal and a clean run all pass.
 * A panic, a hang, a crash-shaped status and a definite leak do not. Two failures look like a pass, and
 * each is an attack that was never delivered: a compile diagnostic, unless the file declares
 * `expect-refusal`, and a program that ends before its last line, unless the file declares `ends-early`.
 * An attack that declares `proof: test` runs under `nvs test`, and its last line is the report's summary.
 */
async function runHostile(nvs: string, path: string, valgrind: boolean, unlogged = false): Promise<Judged> {
  const source = read(path);
  const skip = skipReason(source) ?? (await serviceSkipReason(source));
  if (skip) return ["skip", skip];
  let limit = hostileLimitMs(source);
  let argv = proofArgv(nvs, path, source);
  if (valgrind) {
    argv = ["valgrind", "--quiet", "--error-exitcode=97", "--leak-check=full", "--errors-for-leak-kinds=definite", ...argv];
    limit *= HANG_FACTOR;
  }
  let out: Ran;
  try {
    out = await spawnProof(argv, path, limit, unlogged ? { unlogged } : undefined);
  } catch (e) {
    return ["fail", `could not run: ${e instanceof Error ? e.message : String(e)}`];
  }
  return [...judgeHostile(path, source, out, limit, valgrind), out];
}

/** An attack's verdict from one run of it under `limit`. */
export function judgeHostile(path: string, source: string, out: Ran, limit: number, valgrind: boolean): [Verdict, string] {
  const declared = declaredEnding(source);
  if (typeof declared === "string") return ["fail", declared];
  if (out.timedOut) return ["fail", `still running after ${Math.round(limit / 1000)}s -- unbounded${reached(out.stdout)}`];
  const blob = out.stderr + out.stdout;
  for (const marker of CRASH_MARKERS) {
    const line = blob.split("\n").find((l) => l.includes(marker));
    if (line !== undefined) return ["fail", `stderr carries ${JSON.stringify(marker)}: ${line.trim().slice(0, 300)}`];
  }
  const refused = REFUSED_RE.test(out.stderr) && !out.stdout.trim();
  if (REFUSAL_EXPECTED_RE.test(source)) {
    return refused ? ["ok", ""] : ["fail", "declares `expect-refusal`, but the compiler accepted it"];
  }
  if (refused) {
    const first = out.stderr.split(/\r?\n/).find((l) => l.startsWith("error[")) ?? "";
    return ["fail", `never ran -- it does not compile: ${first}`];
  }
  if (valgrind && out.code === 97) return ["fail", "valgrind reports a definite leak or an invalid access"];
  // A negative status is a signal; anything past 255 on Windows is a structured exception.
  if (out.code < 0 || out.code > 255) return ["fail", `crash-shaped exit status ${out.code}`];
  // Under `nvs test` every test runs in its own isolate, so a test that fails stops nothing after it.
  // The attack was delivered when the report reached its summary line.
  if (EXAMPLE_MUTATE_RE.test(source) && !declared) {
    if (out.code === 0 && MUTATION_SUMMARY_RE.test(out.stdout)) return ["ok", ""];
    return ["fail", `the mutation report ended before its counts line (exit ${out.code}): ${[...stoppedBy(out.stderr)].slice(0, 120).join("")}`];
  }
  if (EXAMPLE_TEST_RE.test(source) && !declared) {
    if ((out.code === 0 || out.code === TEST_FAILED_EXIT) && TEST_SUMMARY_RE.test(out.stdout)) return ["ok", ""];
    return ["fail", `the test report ended before its summary line (exit ${out.code}): ${[...stoppedBy(out.stderr)].slice(0, 120).join("")}`];
  }
  if (out.code !== 0 && !declared) {
    const first = stoppedBy(out.stderr);
    return [
      "fail",
      `ended before its last line (exit ${out.code}), so the steps behind that point never ran -- catch what ` +
        `stopped it, or make that step the last one and declare \`ends-early <step> <ending>\`: ${[...first].slice(0, 120).join("")}`,
    ];
  }
  if (out.code === 0 && declared) return ["fail", "declares `ends-early`, but ran to its last line -- remove the marker"];
  if (!declared) return ["ok", ""];
  const seen = observedEnding(out, path);
  const wants = `declares step ${declared.step} with ${declared.ending}`;
  // The step's line ends with `step N`. Output before it, such as a response body, may not end its own line.
  if (!new RegExp(`step ${declared.step}\\r?$`, "m").test(out.stdout)) {
    const at = seen.line === null ? null : stepAt(source, seen.line);
    const where = at === null || at === declared.step ? `before step ${declared.step}` : `at step ${at}`;
    return ["fail", `stopped ${where} with ${seen.ending}, ${wants}`];
  }
  if (seen.ending !== declared.ending) return ["fail", `stopped at step ${declared.step} with ${seen.ending}, ${wants}`];
  return ["ok", ""];
}

/** One verdict re-judged against the file's `proof: gap` marker. The marker must name a gap record. A
 * marked proof that fails is counted as a known gap. A marked proof that passes fails, because the marker
 * has to go with the bug it names. */
function judgeGap(path: string, verdict: Verdict, why: string): [Verdict, string] {
  const id = knownGap(read(path));
  if (!id) return [verdict, why];
  const title = gapTitle(id);
  if (title === null) return ["fail", `\`proof: gap\` names ${id}, which is not a record under data/gaps/`];
  if (verdict === "ok") return ["fail", `passes, but is still marked \`proof: gap ${id}\` -- remove the marker`];
  if (verdict === "skip") return [verdict, why];
  return ["known", `${title} (gap ${id})`];
}

/** Records each group's own paths in the selection store, which a `proofs: <group>` unit keys on.
 * Groups this run did not name keep their record. */
export function saveReads(groups: [string, string[]][]): void {
  if (groups.length === 0) return;
  try {
    const store = new SelectStore();
    try {
      store.transaction(() => {
        for (const [group, paths] of groups) store.putVerdict(proofReadsSlot(group), "", JSON.stringify(paths));
      });
    } finally {
      store.close();
    }
  } catch {
    // A record that cannot be written keys the group on everything, which only re-runs it.
  }
}

/** How many programs run at once: `NVS_PROOF_JOBS`, or half the cores. */
export function jobsFor(count: number): number {
  const env = Number(process.env.NVS_PROOF_JOBS);
  const cores = navigator.hardwareConcurrency || 2;
  const width = Number.isInteger(env) && env > 0 ? env : Math.max(1, Math.floor(cores / 2));
  return Math.max(1, Math.min(width, count));
}

export interface RunOptions {
  valgrind: boolean;
  strict: boolean;
  /** The judged runs write no footprint log, not even one this process inherited: the selection records
   * each program on its own recording run instead. */
  unlogged?: boolean;
  /** Called with each program's result the moment it is final: at once, or for one that ran out of
   * time beside others, once its run alone ends. */
  onResult?: (program: Program, result: Result) => void;
}

export interface Result {
  verdict: Verdict;
  why: string;
  /** Not run: the change reaches nothing it ran, and its last run was green. */
  cached: boolean;
  /** What the judged run did; absent for a program not run or skipped on this host. */
  ran?: Ran;
}

export interface Pass {
  /** By `<what>:<path>`. */
  results: Map<string, Result>;
  width: number;
  seconds: number;
}

/** A program to run, and the parts of the process its reads are noted under (`inPart`); none notes them
 * for the process as a whole. */
export interface Program {
  what: What;
  path: string;
  parts?: readonly string[];
}

/** Runs every program named, once each, in one pool; each of `unchanged` is reported green without a
 * run. `opts.onResult` hears each result of a program that ran as soon as it is final. */
export async function runPrograms(bin: Binary, programs: Program[], opts: RunOptions, unchanged: Set<string> = new Set()): Promise<Pass> {
  const results = new Map<string, Result>();
  const todo: Program[] = [];
  const seen = new Set<string>();
  for (const p of programs) {
    const id = `${p.what}:${p.path}`;
    if (seen.has(id)) continue;
    seen.add(id);
    if (unchanged.has(p.path)) results.set(id, { verdict: "ok", why: "", cached: true });
    else todo.push(p);
  }
  const width = jobsFor(todo.length);
  const started = performance.now();
  const cached = results.size;
  let next = 0;
  let ran = 0;
  let failed = 0;
  const say = () => progress(`proofs: ${ran}/${todo.length} programs run${cached ? ` (${cached} more unchanged)` : ""}${failed ? `, ${failed} failed` : ""}`);
  if (todo.length > 0) say();
  const runOne = async (t: Program) => {
    const unlogged = opts.unlogged === true;
    const [verdict, why, out] = await inPart(t.parts ?? [], async () => {
      const [raw, rawWhy, ran] = t.what === "examples" ? await runExample(bin.path, t.path, unlogged) : await runHostile(bin.path, t.path, opts.valgrind, unlogged);
      return [...judgeGap(t.path, raw, rawWhy), ran] as const;
    });
    results.set(`${t.what}:${t.path}`, { verdict, why, cached: false, ...(out ? { ran: out } : {}) });
  };
  const worker = async () => {
    while (next < todo.length) {
      const t = todo[next++]!;
      await runOne(t);
      ran++;
      const got = results.get(`${t.what}:${t.path}`)!;
      if (got.verdict === "fail") failed++;
      if (got.ran?.timedOut !== true) opts.onResult?.(t, got);
      say();
    }
  };
  await Promise.all(Array.from({ length: width }, worker));
  const again = new Set(timedOut(results));
  for (const t of todo.filter((p) => again.has(`${p.what}:${p.path}`))) {
    progress(`proofs: ${t.path} ran out of time beside others, so it runs once more alone`);
    await runOne(t);
    opts.onResult?.(t, results.get(`${t.what}:${t.path}`)!);
  }
  return { results, width, seconds: (performance.now() - started) / 1000 };
}

/**
 * The programs whose run ran out of time, each of which is run once more alone before it is judged.
 * A sweep runs many checks beside one another, and that can slow a bounded program past its limit;
 * one that really does not end runs out of time alone too, so the retry keeps every hang a failure.
 */
export function timedOut(results: Map<string, Result>): string[] {
  return [...results].filter(([, r]) => r.ran?.timedOut === true).map(([id]) => id);
}

/** How long a recording run of `path` may take before it is killed as hung: its own limit times
 * `HANG_FACTOR`, or its own limit alone when the judged run already ran out of it. */
export function recordingLimitMs(what: What, path: string, judged?: Ran): number {
  const limit = what === "hostile" ? hostileLimitMs(read(path)) : EXAMPLE_TIMEOUT_MS;
  return judged?.timedOut ? limit : limit * HANG_FACTOR;
}

/** The recording run of one program on `nvs`, the covws debug build, recorded into `dir` with `env`
 * beside it. It has a hang limit and no verdict. It runs with the command its judged run used. */
export function recordingRun(nvs: string, what: What, path: string, dir: string, env: Record<string, string> = {}, judged?: Ran): Promise<Ran> {
  return spawnProof(proofArgv(nvs, path, read(path)), path, recordingLimitMs(what, path, judged), { dir, env });
}

/** `ran` the way two runs of `path` are compared: a `proof: test` program's report loses its timings
 * (`testReport`), which differ on every run, and any other program's run is unchanged. */
export function comparableRun(_what: What, path: string, ran: Ran): Ran {
  return EXAMPLE_TEST_RE.test(read(path)) ? { ...ran, stdout: testReport(ran.stdout) } : ran;
}

/**
 * Why a recording run ended differently from the judged run, or null when both ended alike: the same
 * exit status and the same standard output. A recording run that ends differently may have stopped
 * short of code the judged run reached, so its footprint is not trusted. A judged run that ran out of
 * time is red whatever the recording run did, and is compared with nothing.
 *
 * The two runs are different binaries in different folders, and a program may print its binary's
 * default data folder, `.nvsdata` beside the binary. Each run's own folder is replaced with one
 * placeholder in its output before the two are compared (`ownDataFolder`). A `.cmd` example whose `.out`
 * elides lines prints what differs per build in them, so with `elided`, that `.out`, the recording run
 * agrees when its output matches the `.out`.
 */
export function divergence(judged: Ran, recorded: Ran, elided: string | null = null): string | null {
  if (judged.timedOut) return null;
  if (recorded.timedOut) return `the recording run was still running after ${Math.round(recorded.ms / 1000)}s`;
  if (recorded.code !== judged.code) return `exit ${recorded.code} on the recording run, ${judged.code} on the judged run`;
  if (elided !== null) return matchesOut(recorded.stdout, elided, true) ? null : "stdout on the recording run differs from the elided .out";
  const a = normalise(ownDataFolder(judged)).split("\n");
  const b = normalise(ownDataFolder(recorded)).split("\n");
  if (a.join("\n") === b.join("\n")) return null;
  let line = 0;
  while (line < a.length && line < b.length && a[line] === b[line]) line++;
  return `stdout differs from the judged run's at line ${line + 1}`;
}

/** `ran`'s standard output with its binary's default data folder, `resolve(dirname(exe), ".nvsdata")`,
 * replaced by `<data folder>`, in either slash form. On Windows the match ignores case, as the file
 * system does, so a drive letter printed in either case is found. A run with no `exe` is unchanged. */
export function ownDataFolder(ran: Ran): string {
  if (!ran.exe) return ran.stdout;
  const folder = resolve(ROOT, dirname(ran.exe), ".nvsdata");
  const pattern = folder
    .split(/[\\/]/)
    .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("[\\\\/]");
  return ran.stdout.replace(new RegExp(pattern, process.platform === "win32" ? "gi" : "g"), "<data folder>");
}

/** One program run for a person to read: a `== <path>` line, what it printed, then its exit status and
 * time. An attack's time is shown against its limit. Nothing is judged here and nothing is remembered. */
export async function showProgram(nvs: string, path: string, kind: What | "bench"): Promise<string> {
  const limit = kind === "hostile" ? hostileLimitMs(read(path)) : EXAMPLE_TIMEOUT_MS;
  const out = await spawnProof(kind === "bench" ? [nvs, "run", path] : proofArgv(nvs, path, read(path)), path, limit);
  const lines = [`== ${path}`];
  const printed = (out.stdout + out.stderr).replace(/\r\n/g, "\n").trimEnd();
  if (printed) lines.push(printed);
  const ms = Math.round(out.ms);
  const status = out.timedOut ? "killed, out of time" : `exit ${out.code}`;
  lines.push(kind === "hostile" ? `${status} · ${ms} ms of ${limit} ms` : `${status} · ${ms} ms`);
  return `${lines.join("\n")}\n\n`;
}

/** One suite's verdict lines over `files`, out of a pass that ran them. Returns whether any failed. */
export function suiteLines(out: string[], what: What, files: string[], pass: Pass, opts: RunOptions & { quiet: boolean }): boolean {
  if (files.length === 0) {
    out.push(`proofs ${what}: 0 ok, 0 skipped, 0 known-gap, 0 failed (no ${what} on disk yet -- nothing to run)`);
    return false;
  }
  const rows = files.map((f) => [f, pass.results.get(`${what}:${f}`)!] as const);
  const ok = rows.filter(([, r]) => r.verdict === "ok").length;
  const cached = rows.filter(([, r]) => r.cached).length;
  const skipped = rows.filter(([, r]) => r.verdict === "skip");
  let gaps = rows.filter(([, r]) => r.verdict === "known");
  const bad = rows.filter(([, r]) => r.verdict === "fail");
  if (opts.strict) {
    bad.push(...gaps);
    gaps = [];
  }
  const byPath = (a: readonly [string, Result], b: readonly [string, Result]) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0);
  for (const [f, r] of bad.sort(byPath)) out.push(`  FAIL  ${f}: ${r.why}`);
  for (const [f, r] of gaps.sort(byPath)) out.push(`  gap   ${f}: ${r.why}`);
  if (!opts.quiet) for (const [f, r] of skipped.sort(byPath)) out.push(`  skip  ${f}: ${r.why}`);
  out.push(
    `proofs ${what}: ${ok} ok, ${skipped.length} skipped, ${gaps.length} known-gap, ${bad.length} failed ` +
      `(${files.length} files, ${cached} green and not reached by the change, ${pass.width} at a time, ${pass.seconds.toFixed(1)}s)`,
  );
  return bad.length > 0;
}
