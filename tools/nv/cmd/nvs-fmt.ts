// `bun nv nvs-fmt`: formats every `.nvs` file the corpus holds, so a change to a layout rule is one
// command over the whole tree. `tools/data/nvs-fmt-corpus.txt` says which files those are.
//
//     bun nv nvs-fmt [PATH...]           build `nvs`, format every held file (under PATH when given), and
//                                        name each expectation file the new layout can have moved
//     bun nv nvs-fmt --bless [PATH...]   the same, then bless each one it names
//     bun nv nvs-fmt --check [PATH...]   write nothing: name each held file the formatter would change,
//                                        and each `.nvs` file outside every tree
//     --nvs <path>                       format with this binary instead of building the covws `nvs`
//
// A file that does not parse is refused by `nvs fmt` and left as it is: the corpus has many that are
// broken on purpose. A layout change never changes what a program does (`crates/nvs-fmt/tests/
// corpus.rs`), but it moves lines and columns, and an expectation file beside a program can quote them:
// a `.out` or `.err` that names `file.nvs:12`, a `.tainted` range, a recorded `.lcov`. The run names each
// such file beside a program it rewrote, with the command that writes it again, and `--bless` runs those
// commands. A name in that list is a lead, not a verdict: the full gate is what proves the tree green.

import { readdirSync, readFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { buildCovws } from "../lib/covws.ts";
import { held, nvsFiles, outside, readCorpus } from "../lib/nvs-fmt-corpus.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough, run as runProc } from "../lib/proc.ts";
import { cargoLines } from "../lib/progress.ts";

export const summary = "format every .nvs file the formatter holds, and name the expectation files to bless again: nv nvs-fmt [--check | --bless] [--nvs PATH] [PATH...]";

/** Below Windows' 32 767-character command line, with room for the binary and the flags. */
const ARGV_CHARS = 24_000;
/** What `nvs fmt` prints on standard error for a file it refuses. */
const REFUSED_RE = /^error: (.+?): not formatted, because it does not parse/gm;
/** A position written into an expectation file: `file.nvs:12`, `12:5` or `line 12`. */
const PIN_RE = /\.nvs:\d+|\b\d+:\d+\b|\bline \d+\b/;
/** Expectation files that quote positions or text of the program whatever they say: a snippet's
 * `.tainted` ranges, a `.replay`'s starting text, which is the snippet before its steps type the rest,
 * and a recorded `.lcov`. */
const ALWAYS = [".tainted", ".replay", ".lcov"];
/** Expectation files that can quote a position, and do when `PIN_RE` finds one. */
const QUOTING = [".out", ".err"];
const SNIPPETS = "website/snippets/";

interface Options {
  mode: "write" | "check" | "bless";
  nvs: string | null;
  paths: string[];
}

function parse(args: string[]): Options | string {
  const o: Options = { mode: "write", nvs: null, paths: [] };
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--check" || a === "--bless") {
      if (o.mode !== "write") return "--check and --bless do not go together";
      o.mode = a === "--check" ? "check" : "bless";
    } else if (a === "--nvs") {
      const v = args[++i];
      if (v === undefined) return "--nvs needs a path";
      o.nvs = v;
    } else if (a.startsWith("-")) {
      return `unknown argument ${a}`;
    } else {
      o.paths.push(a.replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/$/, ""));
    }
  }
  return o;
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const o = parse(args);
  if (typeof o === "string") {
    console.error(`nv nvs-fmt: ${o}`);
    return 2;
  }
  const corpus = readCorpus();
  const all = await nvsFiles();
  const strays = all.filter((f) => outside(corpus, f));
  const wanted = (f: string) => o.paths.length === 0 || o.paths.some((p) => f === p || f.startsWith(`${p}/`));
  const files = all.filter((f) => held(corpus, f) && wanted(f));
  if (o.paths.length > 0 && files.length === 0) {
    console.error(`nv nvs-fmt: no file the corpus holds is under ${o.paths.join(", ")}`);
    return 2;
  }

  const nvs = o.nvs ?? (await build());
  if (nvs === null) return 1;

  const before = new Map(files.map((f) => [f, readFileSync(join(ROOT, f))]));
  const fmt = await format(nvs, files, o.mode === "check");
  if (fmt === null) return 1;

  for (const f of strays) console.log(`outside every tree: ${f}`);
  if (strays.length > 0) console.log(`nv nvs-fmt: add the tree of each file above to tools/data/nvs-fmt-corpus.txt, or a skip that says why`);

  if (o.mode === "check") {
    for (const f of fmt.named) console.log(`would change: ${f}`);
    console.log(`nv nvs-fmt: ${fmt.named.length} of ${files.length} file(s) would change; ${fmt.refused} do not parse and are left as they are`);
    return fmt.named.length > 0 || strays.length > 0 ? 1 : 0;
  }

  const rewritten = files.filter((f) => !readFileSync(join(ROOT, f)).equals(before.get(f)!));
  const byTree = new Map<string, number>();
  for (const f of rewritten) {
    const tree = corpus.trees.find((t) => f.startsWith(`${t}/`))!;
    byTree.set(tree, (byTree.get(tree) ?? 0) + 1);
  }
  for (const [tree, n] of [...byTree].sort((a, b) => b[1] - a[1])) console.log(`  ${String(n).padStart(5)}  ${tree}/`);
  console.log(`nv nvs-fmt: rewrote ${rewritten.length} of ${files.length} file(s); ${fmt.refused} do not parse and are left as they are`);

  const pins = pinned(rewritten);
  const site = pins.filter((p) => p.program.startsWith(SNIPPETS) && !p.byHand);
  const proofs = pins.filter((p) => !p.program.startsWith(SNIPPETS) && !p.byHand);
  const byHand = pins.filter((p) => p.byHand);
  const programs = (ps: Pin[]) => [...new Set(ps.map((p) => p.program))];
  if (pins.length > 0) {
    console.log(`\nnv nvs-fmt: ${pins.length} expectation file(s) beside a rewritten program can quote a position:`);
    for (const p of pins) console.log(`  ${p.sidecar}`);
    if (proofs.length > 0) console.log(`\n  bun nv proofs --bless ${programs(proofs).join(" ")}`);
    if (site.length > 0) console.log(`\n  bun nv site --bless ${programs(site).join(" ")}`);
    if (byHand.length > 0) console.log(`\n  by hand, as the test that reads it says: ${byHand.map((p) => p.sidecar).join(" ")}`);
  }
  const cites = citing(rewritten.filter((f) => lines(before.get(f)!.toString("utf8")) !== lines(readFileSync(join(ROOT, f), "utf8"))));
  if (cites.length > 0) {
    console.log(`\nnv nvs-fmt: ${cites.length} comment(s) in a program whose lines moved name a line number; check each against the program as it now is:`);
    for (const c of cites) console.log(`  ${c}`);
  }
  if (o.mode !== "bless") return strays.length > 0 ? 1 : 0;

  let code = 0;
  if (proofs.length > 0) code ||= await passthrough(["bun", "nv", "proofs", "--bless", ...programs(proofs)], { timeoutMs: 60 * 60 * 1000 });
  if (site.length > 0) code ||= await passthrough(["bun", "nv", "site", "--bless", ...programs(site)], { timeoutMs: 60 * 60 * 1000 });
  if (byHand.length > 0) code ||= 1;
  return code || (strays.length > 0 ? 1 : 0);
}

const lines = (text: string) => text.split("\n").length;
/** A comment that names a line: `// line 9 never ran`. */
const CITE_RE = /^\s*(\/\/|#|\*).*\b[Ll]ines? \d+/;

/** `file:line: text` for each comment in `programs` that names a line number. A program that reads its
 * own coverage or its own `location` names its lines in code too, so a blessed `.out` can be wrong
 * without failing, and only these comments point at it. */
function citing(programs: string[]): string[] {
  const out: string[] = [];
  for (const f of programs) {
    for (const [i, l] of readFileSync(join(ROOT, f), "utf8").split(/\r?\n/).entries()) {
      if (CITE_RE.test(l)) out.push(`${f}:${i + 1}: ${l.trim()}`);
    }
  }
  return out;
}

/** Builds the covws tree, the one debug build the pipeline runs, and returns its `nvs`, or null when the
 * build fails. */
async function build(): Promise<string | null> {
  console.log("nv nvs-fmt: building the covws `nvs`");
  try {
    return (await buildCovws({ onLine: cargoLines("nv nvs-fmt") })).nvs;
  } catch (e) {
    console.error(`nv nvs-fmt: ${(e as Error).message.trimEnd().split("\n").slice(-15).join("\n")}`);
    return null;
  }
}

/** Runs `nvs fmt` over `files` in batches that fit a command line. `named` is what `--check` printed,
 * and `refused` how many files do not parse. Null when `nvs` fails in a way a refusal does not explain. */
async function format(nvs: string, files: string[], check: boolean): Promise<{ named: string[]; refused: number } | null> {
  const named: string[] = [];
  let refused = 0;
  for (const batch of batches(files)) {
    const r = await runProc([nvs, "fmt", ...(check ? ["--check"] : []), ...batch], { cwd: ROOT });
    // 1 is `--check` finding a file to change, and 2 a file that does not parse.
    if (![0, 1, 2].includes(r.code)) {
      console.error(`nv nvs-fmt: \`nvs fmt\` exited ${r.code}:\n${r.stderr.trimEnd()}`);
      return null;
    }
    refused += [...r.stderr.matchAll(REFUSED_RE)].length;
    if (check) named.push(...r.stdout.split(/\r?\n/).map((l) => l.trim().replaceAll("\\", "/")).filter(Boolean));
  }
  return { named, refused };
}

function* batches(files: string[]): Generator<string[]> {
  let batch: string[] = [];
  let chars = 0;
  for (const f of files) {
    if (batch.length > 0 && chars + f.length + 1 > ARGV_CHARS) {
      yield batch;
      batch = [];
      chars = 0;
    }
    batch.push(f);
    chars += f.length + 1;
  }
  if (batch.length > 0) yield batch;
}

interface Pin {
  program: string;
  sidecar: string;
  /** No `bun nv` command writes it, so the test that reads it says how. */
  byHand: boolean;
}

/** Each expectation file beside a program in `programs` that can quote a position in it. */
function pinned(programs: string[]): Pin[] {
  const pins: Pin[] = [];
  for (const program of programs) {
    const dir = dirname(program);
    const stem = basename(program, ".nvs");
    for (const name of readdirSync(join(ROOT, dir))) {
      if (!name.startsWith(`${stem}.`) || name.endsWith(".nvs")) continue;
      const ext = name.slice(stem.length);
      const sidecar = `${dir}/${name}`;
      // `nv site --bless` writes a snippet's `.tainted`, and `nv proofs --bless` an example's `.out`.
      const blessed = ext === ".tainted" ? program.startsWith(SNIPPETS) : QUOTING.some((e) => ext.endsWith(e));
      if (ALWAYS.includes(ext) || (QUOTING.some((e) => ext.endsWith(e)) && PIN_RE.test(readFileSync(join(ROOT, sidecar), "utf8")))) {
        pins.push({ program, sidecar, byHand: !blessed });
      }
    }
  }
  return pins;
}
