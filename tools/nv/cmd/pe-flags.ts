// `bun nv pe-flags --bin <path>`: checks that a Windows binary carries the protections and the stack
// `nvs.exe` ships with, and names each one it lacks. Exits 0 when all are there, 1 when one is missing
// or the file is not a PE32+ image, and 2 on a bad command line.
//
// It reads the PE headers itself, a few hundred bytes from the front of the file, so no machine or CI
// runner needs `dumpbin` or `llvm-readobj` for it. What it checks, and why each one matters:
//
//   * `DYNAMIC_BASE` (ASLR): the loader may place the image at a random address, so an attacker cannot
//     know where its code and data sit in memory.
//   * `HIGH_ENTROPY_VA`: that random address is drawn from the whole 64-bit space. Without it, ASLR on
//     a 64-bit image has few enough choices to guess.
//   * `NX_COMPAT` (DEP): data pages are never executable, so bytes an attacker writes into a buffer
//     cannot run as code. The JIT maps its own code pages executable on purpose, and only those.
//   * `SizeOfStackReserve` of 8 MiB: the main thread's stack is the size Linux and macOS give, set by
//     `crates/nvs-cli/build.rs` `main_thread_stack`. The default of 1 MiB aborts deep programs.
//   * a non-empty resource directory: the icon and the version information `build.rs` links, which
//     Explorer and the file's properties show.
//
// A linker change is what loses one of these silently: each is a linker default or a linker argument,
// and nothing a test runs would notice its absence. The Windows leg of `.github/workflows/build.yml`
// runs this on the binary it archives, before the archive step.

export const summary = "check that a Windows binary has ASLR, high-entropy addresses, NX, an 8 MiB stack and resources: nv pe-flags --bin <path>";

/** `IMAGE_DLLCHARACTERISTICS_*` bits this check requires, with the name it reports for each. */
export const REQUIRED_FLAGS: ReadonlyArray<readonly [name: string, bit: number]> = [
  ["HIGH_ENTROPY_VA", 0x0020],
  ["DYNAMIC_BASE", 0x0040],
  ["NX_COMPAT", 0x0100],
];

/** The main thread's stack reserve `crates/nvs-cli/build.rs` links with `/STACK`. */
export const STACK_RESERVE = 8 * 1024 * 1024;

const PE32_PLUS = 0x20b;
/** Offsets inside a PE32+ optional header. */
const DLL_CHARACTERISTICS = 70;
const STACK_RESERVE_AT = 72;
const RVA_COUNT = 108;
const DIRECTORIES = 112;
const RESOURCE_DIRECTORY = 2;
/** How much of the file the headers need: the DOS stub's pointer is read first, then this much past it. */
const OPTIONAL_HEADER_END = 24 + DIRECTORIES + 8 * (RESOURCE_DIRECTORY + 1);

/** What `inspect` found: a reason the file is not a PE32+ image, or the protections it lacks. */
export type Verdict = { refused: string } | { missing: string[] };

/** Reads the headers in `bytes`, which start at the beginning of the file and hold at least its headers. */
export function inspect(bytes: Uint8Array): Verdict {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (bytes.length < 64 || view.getUint16(0, true) !== 0x5a4d) return { refused: "no MZ header" };
  const pe = view.getUint32(0x3c, true);
  if (pe + 24 > bytes.length || view.getUint32(pe, true) !== 0x4550) return { refused: "no PE signature" };
  const optionalSize = view.getUint16(pe + 20, true);
  const optional = pe + 24;
  const magic = view.getUint16(optional, true);
  if (magic !== PE32_PLUS) return { refused: `optional header magic 0x${magic.toString(16)}, not PE32+ (0x20b)` };
  if (optionalSize < DIRECTORIES || optional + DIRECTORIES > bytes.length) return { refused: "optional header is truncated" };
  const missing: string[] = [];
  const flags = view.getUint16(optional + DLL_CHARACTERISTICS, true);
  for (const [name, bit] of REQUIRED_FLAGS) if ((flags & bit) === 0) missing.push(name);
  const stack = view.getBigUint64(optional + STACK_RESERVE_AT, true);
  if (stack !== BigInt(STACK_RESERVE)) missing.push(`SizeOfStackReserve of ${STACK_RESERVE} (found ${stack})`);
  const count = view.getUint32(optional + RVA_COUNT, true);
  const resource = optional + DIRECTORIES + 8 * RESOURCE_DIRECTORY;
  const hasResource =
    count > RESOURCE_DIRECTORY &&
    optionalSize >= DIRECTORIES + 8 * (RESOURCE_DIRECTORY + 1) &&
    resource + 8 <= bytes.length &&
    view.getUint32(resource, true) !== 0 &&
    view.getUint32(resource + 4, true) !== 0;
  if (!hasResource) missing.push("a resource directory (icon and version information)");
  return { missing };
}

/** Reads the front of `path`, enough for its headers wherever the DOS stub points. */
async function headers(path: string): Promise<Uint8Array> {
  const file = Bun.file(path);
  const head = new Uint8Array(await file.slice(0, 64).arrayBuffer());
  if (head.length < 64) return head;
  const pe = new DataView(head.buffer).getUint32(0x3c, true);
  return new Uint8Array(await file.slice(0, pe + OPTIONAL_HEADER_END).arrayBuffer());
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const at = args.indexOf("--bin");
  const path = at >= 0 ? args[at + 1] : undefined;
  if (path === undefined || args.length !== 2) {
    console.error("usage: bun nv pe-flags --bin <path>");
    return 2;
  }
  if (!(await Bun.file(path).exists())) {
    console.error(`nv pe-flags: ${path} does not exist`);
    return 1;
  }
  const verdict = inspect(await headers(path));
  if ("refused" in verdict) {
    console.error(`nv pe-flags: ${path} is not a PE32+ image: ${verdict.refused}`);
    return 1;
  }
  if (verdict.missing.length > 0) {
    for (const lack of verdict.missing) console.error(`nv pe-flags: ${path} lacks ${lack}`);
    return 1;
  }
  console.log(`nv pe-flags: ${path} has ASLR, high-entropy addresses, NX, an 8 MiB stack and resources`);
  return 0;
}
