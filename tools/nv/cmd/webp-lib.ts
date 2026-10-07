// `bun nv webp-lib`: libwebp compiled with wasi-sdk into the wasm static library the image component
// links, committed under `extensions/image/libwebp/` beside the digests it was built from.
// `rule:packaging/a-prebuilt-wasm-library-is-rebuilt-in-ci` owns the policy.
//
//     bun nv webp-lib                     rebuild the library from the pinned source and record its digest
//     bun nv webp-lib --check             exit 1 when the committed library's bytes differ from its record
//     bun nv webp-lib --verify            rebuild, and exit 1 when the bytes differ from the committed file (CI)
//     bun nv webp-lib --source <tarball>  build from a local copy of the source instead of downloading it
//
// `SOURCE.json` pins three things: the libwebp source tarball and the wasi-sdk release, each by URL
// and sha256, and the library's own sha256. A tarball whose digest differs from its pin is refused
// before a byte of it is extracted, so the library is always a function of reviewed source and a
// known compiler. Updating libwebp is an edit to the source pin followed by a rebuild.
//
// The build is the same on every host: the Linux wasi-sdk release, run directly on Linux and under
// WSL (`wsl.exe`) on Windows, so the bytes a contributor commits are the bytes CI's Linux leg
// rebuilds. It compiles every C file of `src/dec`, `src/enc`, `src/dsp`, `src/utils` and `sharpyuv`
// with no debug information, from a relative path, in a sorted order, and archives them with
// `llvm-ar`'s deterministic mode, so nothing about the machine or the time reaches the archive. No
// SIMD path is compiled: libwebp has none for wasm, and its x86 and Arm files compile empty.
//
// Downloads, the extracted trees and the objects go under `.agent-tmp/webp-lib/`, which the tool
// deletes when it is done. libwebp's `COPYING` and `PATENTS` are copied beside the library, because
// the third-party notice reproduces them.

import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs } from "../lib/py.ts";

export const summary = "libwebp's wasm static library, rebuilt from its pinned source or checked against its digest: nv webp-lib [--check|--verify]";

const USAGE = "usage: nv webp-lib [-h] [--check] [--verify] [--source SOURCE]";

/** Where the library, its record and libwebp's licence files are committed. */
export const LIB_DIR = join(ROOT, "extensions", "image", "libwebp");
const SCRATCH = join(ROOT, ".agent-tmp", "webp-lib");

/** The source folders whose C files make the library, relative to the extracted source. */
const SOURCE_DIRS = ["src/dec", "src/enc", "src/dsp", "src/utils", "sharpyuv"];
const LICENCE_FILES = ["COPYING", "PATENTS"];

const BUILD_TIMEOUT_MS = 20 * 60_000;
const OK = "libwebp: the committed library matches its recorded digest";

interface Pin {
  url: string;
  sha256: string;
}

/** `SOURCE.json`: what the library was built from, and the digest of what it is. */
export interface LibRecord {
  library: string;
  sha256: string;
  source: Pin & { version: string; dir: string };
  toolchain: Pin & { version: string; dir: string };
  cflags: string[];
}

export function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

export function readRecord(dir: string): LibRecord {
  return JSON.parse(readFileSync(join(dir, "SOURCE.json"), "utf8")) as LibRecord;
}

/** Whether the library in `dir` has the digest its record names: an empty string when it does,
 * and otherwise the sentence that says why not. */
export function checkLibrary(dir: string): string {
  if (!existsSync(join(dir, "SOURCE.json"))) return `no SOURCE.json in ${dir}; run \`bun nv webp-lib\``;
  const record = readRecord(dir);
  const path = join(dir, record.library);
  if (!existsSync(path)) return `no ${record.library} in ${dir}; run \`bun nv webp-lib\``;
  const actual = sha256(readFileSync(path));
  if (actual !== record.sha256) {
    return `${record.library} has sha256 ${actual}, and SOURCE.json records ${record.sha256}. ` +
      "Rebuild it with `bun nv webp-lib`; never edit the digest by hand.";
  }
  return "";
}

/** A tarball whose digest differs from the pin is refused, before anything reads it further. */
export class PinMismatch extends Error {}

export function verifyPin(what: string, bytes: Uint8Array, pin: Pin): void {
  const actual = sha256(bytes);
  if (actual !== pin.sha256) {
    throw new PinMismatch(`refused: the ${what} tarball has sha256 ${actual}, and SOURCE.json pins ${pin.sha256}`);
  }
}

/** The tarball at `pin.url`, downloaded into `dir` and checked against the pin. */
async function fetchPinned(what: string, pin: Pin, dir: string): Promise<string> {
  const path = join(dir, pin.url.slice(pin.url.lastIndexOf("/") + 1));
  const response = await fetch(pin.url);
  if (!response.ok) throw new Error(`${pin.url} answered ${response.status}`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  verifyPin(what, bytes, pin);
  writeFileSync(path, bytes);
  return path;
}

/** `path` as the build's shell sees it: itself on Linux, under `/mnt/<drive>/` from WSL. */
function shellPath(path: string): string {
  if (process.platform !== "win32") return path;
  const m = /^([A-Za-z]):[\\/](.*)$/.exec(path);
  if (!m) throw new Error(`${path} is not on a drive WSL mounts`);
  return `/mnt/${m[1]!.toLowerCase()}/${m[2]!.replaceAll("\\", "/")}`;
}

/** The shell script that extracts both tarballs into `work` and leaves `work/libwebp.a`. */
export function buildScript(record: LibRecord, work: string, source: string, sdk: string): string {
  const q = (s: string) => `'${s.replaceAll("'", "'\\''")}'`;
  const bin = `${q(work)}/sdk/${record.toolchain.dir}/bin`;
  return [
    "set -euo pipefail",
    `cd ${q(work)}`,
    "mkdir -p sdk src obj",
    `tar -xzf ${q(sdk)} -C sdk`,
    `tar -xzf ${q(source)} -C src`,
    `cd src/${record.source.dir}`,
    `files=$(ls ${SOURCE_DIRS.map((d) => `${d}/*.c`).join(" ")} | LC_ALL=C sort)`,
    'dup=$(for f in $files; do basename "$f"; done | sort | uniq -d)',
    'if [ -n "$dup" ]; then echo "two sources share an object name: $dup" >&2; exit 1; fi',
    `for f in $files; do ${bin}/clang ${record.cflags.join(" ")} -c "$f" -o ../../obj/$(basename "\${f%.c}").o; done`,
    "cd ../../obj",
    `${bin}/llvm-ar rcsD ../libwebp.a $(ls *.o | LC_ALL=C sort)`,
  ].join("\n");
}

async function build(sourceArg: string | undefined): Promise<Uint8Array> {
  if (process.platform !== "linux" && process.platform !== "win32") {
    throw new Error("the library is built on Linux, or under WSL on Windows; this host is neither");
  }
  const record = readRecord(LIB_DIR);
  rmSync(SCRATCH, { recursive: true, force: true });
  const dl = join(SCRATCH, "dl");
  mkdirSync(dl, { recursive: true });
  let source: string;
  if (sourceArg) {
    verifyPin("libwebp source", readFileSync(sourceArg), record.source);
    source = join(dl, "source.tar.gz");
    copyFileSync(sourceArg, source);
  } else source = await fetchPinned("libwebp source", record.source, dl);
  const sdk = await fetchPinned("wasi-sdk", record.toolchain, dl);
  const script = buildScript(record, shellPath(SCRATCH), shellPath(source), shellPath(sdk));
  const argv = process.platform === "win32" ? ["wsl.exe", "-e", "bash", "-c", script] : ["bash", "-c", script];
  const r = await runProc(argv, { timeoutMs: BUILD_TIMEOUT_MS });
  if (r.code !== 0) throw new Error(`the build exited ${r.code}${r.timedOut ? " (timed out)" : ""}\n${`${r.stdout}${r.stderr}`.trim()}`);
  const extracted = join(SCRATCH, "src", record.source.dir);
  for (const name of LICENCE_FILES) copyFileSync(join(extracted, name), join(LIB_DIR, name));
  return readFileSync(join(SCRATCH, "libwebp.a"));
}

function help(): string {
  return [
    USAGE,
    "",
    "Rebuild libwebp's wasm static library under extensions/image/libwebp/ from the source and",
    "the wasi-sdk release SOURCE.json pins, and record its digest.",
    "",
    "options:",
    "  --check          exit 1 when the committed library differs from its recorded digest",
    "  --verify         rebuild, and exit 1 when the bytes differ from the committed library",
    "  --source SOURCE  build from this source tarball; it must match the pin",
  ].join("\n");
}

export async function run(args: string[]): Promise<number> {
  let flags: Set<string>;
  let values: Map<string, string>;
  try {
    ({ flags, values } = parseArgs(args, { flags: ["--check", "--verify"], valued: ["--source"] }));
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv webp-lib: error: ${e.message}`);
    return 2;
  }
  if (flags.has("--help")) {
    console.log(help());
    return 0;
  }
  if (flags.has("--check")) {
    const problem = checkLibrary(LIB_DIR);
    if (problem) {
      console.error(`error: ${problem}`);
      return 1;
    }
    console.log(OK);
    return 0;
  }
  let bytes: Uint8Array;
  try {
    bytes = await build(values.get("--source"));
  } catch (e) {
    console.error(`error: ${e instanceof Error ? e.message : String(e)}`);
    return 1;
  } finally {
    rmSync(SCRATCH, { recursive: true, force: true });
  }
  const record = readRecord(LIB_DIR);
  const digest = sha256(bytes);
  if (flags.has("--verify")) {
    const committed = join(LIB_DIR, record.library);
    const same = existsSync(committed) && sha256(readFileSync(committed)) === digest && digest === record.sha256;
    if (!same) {
      console.error(`error: the rebuilt library has sha256 ${digest}, and the committed one differs from it or from SOURCE.json`);
      return 1;
    }
    console.log("libwebp: the rebuilt library is byte for byte the committed one");
    return 0;
  }
  writeFileSync(join(LIB_DIR, record.library), bytes);
  writeFileSync(join(LIB_DIR, "SOURCE.json"), `${JSON.stringify({ ...record, sha256: digest }, null, 2)}\n`);
  console.log(`libwebp: built ${record.library}, ${bytes.length} bytes, sha256 ${digest}`);
  return 0;
}
