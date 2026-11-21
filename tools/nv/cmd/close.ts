// `bun nv close --goal <slug>`: the goal-end check of a goal that is not the chain's full gate. It
// answers one question: is this goal's own work complete? It runs in about three minutes.
//
//     bun nv close --goal <slug>     every check below; exit 0 when all are green, 1 when one is red
//
// It runs `bun nv ci --check`'s checks that judge this tree's own files, taken from `ci.ts`'s
// `PRE_PUSH` by name so each command line has one home, and two checks of the goal's own:
//
//   - `bun nv owners --closes <slug>`: no gap or designed rule still names the goal;
//   - `bun nv proofs --gate --goal <slug>`: every feature the goal's work reaches has all its proofs.
//
// Three phases: the checks that read files run side by side, then `cargo build`, then the checks that
// read the binary or `target/` side by side. The website stale guard takes most of the time, about
// 160 seconds of the three minutes on the reference machine.
//
// What it leaves to the chain's full gate, which `looper verify`, the floor and CI run: the whole
// `cargo test`, the conformance cases and every other case tree, the LSP and tools tests, every proof
// program run on the optimized binary, the memory scans, and of `PRE_PUSH` the advisories, the
// attribution ledger, the separate workspaces' clippy and the whole roster's proofs gate. A goal's
// change selects most of the test suite through the binary's crates (each of the chain's last three
// goals selected 125 to 317 of 351 test binaries and every conformance case), so that part costs as
// much as the full gate and does not fit a goal's end.

import { pool, run as proc } from "../lib/proc.ts";
import { ROOT } from "../lib/paths.ts";
import { ArgError, parseArgs } from "../lib/py.ts";
import { PRE_PUSH, type PrePush } from "./ci.ts";

export const summary = "the goal-end check: ci --check's checks of this tree's files, the goal's owners and its proofs: nv close --goal <slug>";

const USAGE = "usage: nv close --goal SLUG";

const EXE = process.platform === "win32" ? "nvs.exe" : "nvs";
const DEBUG_NVS = `target/debug/${EXE}`;

/** The `PRE_PUSH` checks a goal's end runs before the build, after it, and the one between. */
export const BEFORE_BUILD = ["rustfmt", "lint tables", "rules", "rulebook pages", "rendered files", "repository layout"];
export const BUILD = "build";
export const AFTER_BUILD = ["rustdoc links", "chapter examples", "website render", "website stale guard"];
/** How many lines of a red check's output are printed. */
const SHOWN = 30;

/** The `PRE_PUSH` check named `name`. A name `PRE_PUSH` no longer has is an error, so a rename there is
 * not a check silently dropped here. */
export function prePush(name: string): PrePush {
  const c = PRE_PUSH.find((p) => p.name === name);
  if (!c) throw new Error(`nv close: ci.ts's PRE_PUSH has no check named ${JSON.stringify(name)}`);
  return c;
}

/** The checks of `slug`'s end, in their three phases. */
export function phases(slug: string): PrePush[][] {
  return [
    [...BEFORE_BUILD.map(prePush), { name: "owners", argv: ["bun", "nv", "owners", "--closes", slug] }],
    [prePush(BUILD)],
    [...AFTER_BUILD.map(prePush), { name: "goal proofs", argv: ["bun", "nv", "proofs", "--gate", "--goal", slug, "--nvs", DEBUG_NVS] }],
  ];
}

export async function run(args: string[]): Promise<number> {
  let slug: string;
  try {
    const { flags, values } = parseArgs(args, { flags: [], valued: ["--goal"] });
    if (flags.has("--help")) {
      console.log(`${USAGE}\n\nnv close: ${summary}`);
      return 0;
    }
    const goal = values.get("--goal");
    if (goal === undefined) throw new ArgError("the following arguments are required: --goal");
    slug = goal;
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv close: error: ${e.message}`);
    return 2;
  }
  const reds: string[] = [];
  const start = performance.now();
  for (const phase of phases(slug)) {
    await pool(phase, phase.length, async (c) => {
      const t = performance.now();
      const r = await proc(c.argv, { cwd: ROOT, timeoutMs: 900_000, env: c.env ?? {} });
      const took = ((performance.now() - t) / 1000).toFixed(0);
      if (r.code === 0 && !r.timedOut) {
        console.log(`close: ${c.name}: ok (${took} s)`);
        return;
      }
      console.log(`close: ${c.name}: red (${took} s)`);
      const lines = `${r.stdout}\n${r.stderr}${r.timedOut ? "\ntimed out" : ""}`.split("\n").filter((l) => l.trim() !== "");
      const env = Object.entries(c.env ?? {}).map(([k, v]) => `${k}=${v} `).join("");
      reds.push([`${c.name}: red (\`${env}${c.argv.join(" ")}\` exited ${r.code})`, ...lines.slice(-SHOWN).map((l) => `  ${l}`)].join("\n"));
    });
    // Nothing after a red build reads a current binary.
    if (phase.some((c) => c.name === BUILD) && reds.some((r) => r.startsWith(`${BUILD}: red`))) break;
  }
  const total = ((performance.now() - start) / 1000).toFixed(0);
  if (reds.length === 0) {
    console.log(`close: goal ${slug}: every goal-end check is green (${total} s)`);
    return 0;
  }
  console.log(`\nclose: goal ${slug}: ${reds.length} red check(s) (${total} s)\n\n${reds.join("\n\n")}`);
  return 1;
}
