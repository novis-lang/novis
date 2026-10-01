// `bun nv try`: runs Novis snippets against PHP, several at a time, in the shape a `.nvst` case
// already has.
//
//     bun nv try .agent-tmp/promo.nvst .agent-tmp/div.nvst     one call, as many as you have questions
//     bun nv try --keep .agent-tmp/promo.nvst                  leave the generated .nvs/.php behind
//     bun nv try --bundle examples/hello.nvs --expect "Hello, World!"
//
// Each argument is a file in the `.nvst` shape -- `--TEST--`, `--FILE--`, and optionally `--ORACLE--`
// -- or, when it holds no markers at all, a bare `<?nvs` snippet. For each one this runs the Novis
// binary; where there is an `--ORACLE--` it runs PHP over that too and says whether the two agree, with
// the first line they differ on. `conventions.md` § *A `.nvst` test case* is the format's home.
//
// A hand-written `php -r` twin is a translation, made by the agent that wrote the Novis at the moment
// it most wants the answer to be yes. Here the twin is a section of the same file, so an experiment
// that comes out right is already a differential case: give it a `--TEST--` line and move it under
// `tests/differential/`. The snippets run several at a time, as wide as `tools/nv/lib/machine.ts` says
// this box may go unless `NVS_TRY_JOBS` or `NVS_JOBS` says otherwise, and print back in the order they
// were asked for.
//
// Nothing here judges. A snippet that fails to compile prints its diagnostic, a twin that diverges
// prints both outputs, and the exit status is 0, because "Novis and PHP disagree" is the finding.
//
// `--bundle` swaps the twin: each argument is a `.nvs` entry point, built with `nvs build --compile`,
// and the executable is run beside `nvs run` over the same source. That pair is
// `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s own verification --
// a bundled executable runs identically to `nvs run`. `--expect LINE` is the one place this command
// judges, because an acceptance check needs a verdict: every `LINE` must appear in the bundle's
// output, and the exit status is 1 when one does not or when the bundle and `nvs run` differ.


import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import { basename, join, normalize, parse } from "node:path";
import { Tree } from "../driver/proctree.ts";
import { covwsNvs } from "../lib/covws.ts";
import { jobs as machineJobs } from "../lib/machine.ts";
import { ROOT } from "../lib/paths.ts";
import { ArgError, parseArgs, pyRepr } from "../lib/py.ts";

export const summary = "run .nvst snippets beside their PHP twin, or a bundle beside nvs run: nv try [--bundle] FILE...";

const USAGE = "usage: nv try [-h] [--keep] [--php PHP] [--bundle] [--expect LINE]\n              FILE [FILE ...]";

const TMP = join(ROOT, ".agent-tmp");
let binary = "";

/** The `nvs` the snippets run on: `NVS_BIN` when it is set, and otherwise the pipeline's `covws` build. */
function nvsBinary(): string {
  return (binary ||= process.env.NVS_BIN || covwsNvs());
}

// A `--SECTION--` line in a `.nvst` file. `crates/nvs-test/src/case.rs` owns the full roster; only
// `TEST`, `FILE`, `ORACLE` and `ORACLE-DIVERGES` mean anything here, and an unknown one is ignored.
const SECTION = /^--([A-Z][A-Z0-9-]*)--\s*$/gm;

// How long one program may run. A hung snippet is reported beside the others, never waited on.
const TIMEOUT_S = 30;

function help(): string {
  return [
    USAGE,
    "",
    "Run Novis snippets against PHP, several at a time, in the shape a `.nvst` case already has.",
    "",
    "positional arguments:",
    "  FILE           `.nvst`-shaped snippets, or bare `<?nvs` ones; as many as you have questions",
    "",
    "options:",
    "  -h, --help     show this help message and exit",
    "  --keep         leave the generated .nvs/.php under .agent-tmp/ instead of removing them",
    "  --php PHP      the PHP executable (default `php`)",
    "  --bundle       treat each FILE as a `.nvs` entry point: build it with `nvs build --compile` and",
    "                 run the result beside `nvs run`",
    "  --expect LINE  with --bundle: a line the bundle's output must contain; a miss is a non-zero",
    "                 exit. Repeatable",
  ].join("\n");
}

/** `{SECTION: body}` for a `.nvst` file, or `{FILE: text}` for a bare snippet. */
function sections(text: string): Map<string, string> {
  const marks = [...text.matchAll(SECTION)];
  if (marks.length === 0) return new Map([["FILE", text]]);
  const found = new Map<string, string>();
  marks.forEach((m, i) => {
    const end = i + 1 < marks.length ? marks[i + 1]!.index! : text.length;
    found.set(m[1]!, text.slice(m.index! + m[0].length, end));
  });
  return found;
}

/** The first `n` characters, counted the way Python counts them. */
const head = (s: string, n: number) => [...s].slice(0, n).join("");

/**
 * Runs `argv` in the repository root and returns everything it printed, stdout then stderr, with
 * every line ending read as `\n`.
 */
async function exec(argv: string[]): Promise<[string, number]> {
  let child;
  try {
    child = Bun.spawn(argv, { cwd: ROOT, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  } catch {
    return [`<${argv[0]} not found on PATH>`, 127];
  }
  const tree = new Tree(child);
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    tree.kill();
  }, TIMEOUT_S * 1000);
  try {
    const [stdout, stderr, code] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    if (timedOut) return [`<no answer in ${TIMEOUT_S}s>`, 124];
    return [(stdout + stderr).replace(/\r\n?/g, "\n"), code];
  } finally {
    clearTimeout(timer);
    tree.close();
  }
}

/** The output's lines, each behind a `    | ` gutter. */
const gutter = (text: string) => text.replace(/\n+$/, "").split("\n").map((line) => `    | ${line}`);

/** The first line two outputs disagree on, labelled `left` and `right`. */
function firstDifference(a: string, b: string, left = "nvs", right = "php"): string {
  const la = a.split("\n");
  const lb = b.split("\n");
  for (let n = 0; n < Math.min(la.length, lb.length); n++) {
    if (la[n] !== lb[n]) {
      return `    first difference at line ${n + 1}\n` +
        `      ${left}: ${pyRepr(head(la[n]!, 90))}\n` +
        `      ${right}: ${pyRepr(head(lb[n]!, 90))}`;
    }
  }
  if (la.length !== lb.length) {
    const [longer, who] = la.length > lb.length ? [la, left] : [lb, right];
    const at = Math.min(la.length, lb.length);
    return `    both agree for ${at} line(s); ${who} then has ${Math.abs(la.length - lb.length)} more, ` +
      `starting ${pyRepr(head(longer[at]!, 90))}`;
  }
  return "    the outputs differ in trailing whitespace only";
}

function remove(path: string): void {
  try {
    unlinkSync(path);
  } catch {
    // Already gone, or held open by something else: either way nothing is left to report.
  }
}

/** One snippet: true when Novis and its twin agree, or when there is no twin to disagree with. */
async function one(path: string, keep: boolean, php: string, stem: string): Promise<[boolean, string[]]> {
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch (e) {
    return [true, [`===== ${normalize(path)}  -- cannot read: ${(e as Error).message}`]];
  }
  const parts = sections(text);
  const title = (parts.get("TEST") ?? "").trim().split("\n")[0]!;
  const out = [`===== ${basename(path)}` + (title ? `  -- ${head(title, 90)}` : "")];

  const body = parts.get("FILE");
  if (body === undefined || !body.trim()) {
    out.push("  no `--FILE--` section and no bare snippet -- nothing to run");
    return [true, out];
  }

  mkdirSync(TMP, { recursive: true });
  const nvsFile = join(TMP, `try-${stem}.nvs`);
  writeFileSync(nvsFile, body.replace(/^\n+/, ""));
  const [nvsOut, nvsCode] = await exec([nvsBinary(),"run", nvsFile]);
  out.push(`  nvs  exit ${nvsCode}`, ...gutter(nvsOut));

  const diverges = parts.get("ORACLE-DIVERGES");
  if (diverges !== undefined) {
    out.push(`  oracle: deliberately diverges -- ${head(diverges.trim().split("\n")[0]!, 90)}`);
    return [true, out];
  }
  const twin = parts.get("ORACLE");
  if (twin === undefined) {
    out.push("  no `--ORACLE--`: this ran Novis only. A twin here is what makes the answer");
    out.push("  evidence rather than an opinion -- and makes the file a differential case.");
    return [true, out];
  }

  const phpFile = join(TMP, `try-${stem}.php`);
  writeFileSync(phpFile, twin.replace(/^\n+/, ""));
  const [phpOut, phpCode] = await exec([php, phpFile]);
  out.push(`  php  exit ${phpCode}`, ...gutter(phpOut));
  if (!keep) {
    remove(nvsFile);
    remove(phpFile);
  }

  const agree = nvsOut === phpOut;
  out.push(agree ? "  MATCH" : "  DIFFER");
  if (!agree) out.push(firstDifference(nvsOut, phpOut));
  return [agree, out];
}

/**
 * One entry point, built with `nvs build --compile` and run beside `nvs run`. True when the bundle
 * built, ran, agreed with `nvs run` line for line, and printed every `expect` line.
 */
async function bundled(path: string, keep: boolean, expect: string[], stem: string): Promise<[boolean, string[]]> {
  const out = [`===== ${basename(path)}  -- bundled`];
  if (!existsSync(path)) return [false, [...out, `  cannot read ${normalize(path)}`]];

  mkdirSync(TMP, { recursive: true });
  const exe = join(TMP, process.platform === "win32" ? `bundle-${stem}.exe` : `bundle-${stem}`);
  const [buildOut, buildCode] = await exec([nvsBinary(),"build", "--compile", path, "-o", exe]);
  out.push(`  build  exit ${buildCode}`, ...gutter(buildOut));
  if (buildCode !== 0 || !existsSync(exe)) return [false, out];

  const [bundleOut, bundleCode] = await exec([exe]);
  out.push(`  bundle exit ${bundleCode}`, ...gutter(bundleOut));
  const [runOut, runCode] = await exec([nvsBinary(),"run", path]);
  out.push(`  nvs run exit ${runCode}`);
  if (!keep) remove(exe);

  let ok = true;
  if (bundleOut === runOut && bundleCode === runCode) {
    out.push("  MATCH -- the bundle runs identically to `nvs run` (`rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host` Verification)");
  } else {
    ok = false;
    out.push("  DIFFER -- the bundle and `nvs run` do not agree");
    out.push(firstDifference(bundleOut, runOut, "bundle", "nvs run"));
  }
  for (const line of expect) {
    if (!bundleOut.includes(line)) {
      ok = false;
      out.push(`  MISSING -- the bundle never printed ${pyRepr(line)}`);
    }
  }
  return [ok, out];
}


export async function run(args: string[]): Promise<number> {
  let parsed;
  try {
    parsed = parseArgs(args, {
      flags: ["--keep", "--bundle"],
      valued: ["--php"],
      repeated: ["--expect"],
      order: ["--keep", "--php", "--bundle", "--expect"],
      positionals: true,
    });
    if (parsed.flags.has("--help")) {
      console.log(help());
      return 0;
    }
    if (parsed.positionals.length === 0) throw new ArgError("the following arguments are required: FILE");
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv try: error: ${e.message}`);
    return 2;
  }
  const keep = parsed.flags.has("--keep");
  const bundle = parsed.flags.has("--bundle");
  const php = parsed.values.get("--php") ?? "php";
  const expect = parsed.lists.get("--expect") ?? [];
  if (expect.length > 0 && !bundle) {
    console.error(`${USAGE}\nnv try: error: --expect is only meaningful with --bundle`);
    return 2;
  }

  if (!existsSync(nvsBinary())) {
    console.log(`nv try: ${nvsBinary()} is not built.`);
    console.log("        `bun nv verify` builds it, so a snippet run right after a green");
    console.log("        verification needs nothing.");
    return 2;
  }

  const paths = parsed.positionals;
  // One `.agent-tmp` name per snippet, unique even when two arguments share a stem.
  const stems: string[] = [];
  const seen = new Set<string>();
  paths.forEach((p, i) => {
    const stem = parse(p).name;
    stems.push(seen.has(stem) ? `${stem}-${i}` : stem);
    seen.add(stems[stems.length - 1]!);
  });

  // A snippet is a process pair with nothing shared, so the width is the machine's to decide.
  const jobs = machineJobs("local", { ceiling: paths.length, envs: ["NVS_TRY_JOBS"] });
  const work = (i: number) => (bundle ? bundled(paths[i]!, keep, expect, stems[i]!) : one(paths[i]!, keep, php, stems[i]!));
  const results: Promise<[boolean, string[]]>[] = new Array(paths.length);
  let next = 0;
  const workers = Array.from({ length: jobs }, async () => {
    while (next < paths.length) {
      const i = next++;
      results[i] = work(i);
      await results[i];
    }
  });
  await Promise.all(workers);

  let agreed = 0;
  for (let i = 0; i < paths.length; i++) {
    const [agree, block] = await results[i]!;
    if (i) console.log();
    console.log(block.join("\n"));
    if (agree) agreed++;
  }
  console.log();
  const twins = paths.length - agreed;
  const noun = bundle ? "entry point" : "snippet";
  console.log(
    `-- try: ${paths.length} ${noun}(s) in one call` +
      (jobs > 1 ? `, ${jobs} at a time` : "") +
      (twins ? `, ${twins} disagreeing with its twin` : ""),
  );
  return bundle && twins ? 1 : 0;
}
