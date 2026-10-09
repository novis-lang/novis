// What the sweep after a session runs besides the acceptance sweep: the two goal-end gates and the disk.
// `bun nv sweep` (`cmd/sweep.ts`) calls them, by hand, from a hook or in-process from the driver's turn.
//
// **The goal-end gates** run on a sweep that would reach the goal, and a goal is not reached while either
// is red. The rustdoc gate is `bun nv verify --doc`: a broken intra-doc link stops no build, so a goal in
// progress may carry one, and what has to be clean is the tree a goal leaves behind. The owner gate is
// `bun nv owners --closes <slug>`: a gap record still naming the goal is a tag, not a build, and the
// floor's own check of it would only fire one goal late, once the goal has walked. Each writes its standing verdict to its file under `.loop/`, `{ when, failed, session }`,
// which `nv orient` prints to the next session; a red gate holds the goal open without ending the run.
//
// **The disk.** A run refuses to start below `--min-free-gb`, since a run that fills the disk dies inside a
// session with the tree half edited. `sweepDisk` is `nv disk --clean`'s sweep, run in-process after every
// session's acceptance sweep: `nv disk --clean` itself refuses while `.loop/running` exists, and the sweep
// after a session runs when nothing is building.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { clean, freeGb, freedLines, human, total } from "../cmd/disk.ts";
import { ROOT } from "../lib/paths.ts";
import { failLine, run } from "../lib/proc.ts";

export const DOC_GATE = ".loop/doc-gate.json";
export const OWNER_GATE = ".loop/owner-gate.json";

/** What a gate or a service step printed, one line to the caller. */
type Say = (line: string) => void;


/** Writes a gate's standing verdict: `failed` is "" when it is green. */
function writeGate(path: string, failed: string, session: string, root: string): void {
  const file = join(root, path);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, `${JSON.stringify({ when: Date.now() / 1000, failed, session }, null, 1)}\n`);
}

/** `bun nv verify --doc`. Returns the finding, or "" when green. */
export async function docGate(session: string, say: Say, root: string = ROOT): Promise<string> {
  say("rustdoc gate: every link in a doc comment, resolved (the acceptance list is green)");
  const r = await run(["bun", "nv", "verify", "--doc"], { cwd: root, timeoutMs: 900_000 });
  const found = r.code === 0 ? "" : failLine(`${r.stdout}\n${r.stderr}`, r.code, "bun nv verify --doc");
  writeGate(DOC_GATE, found, found ? session : "", root);
  return found;
}

/** `nv owners --closes <slug>`. Returns its finding, or "" when green. */
export async function ownerGate(slug: string, session: string, say: Say, root: string = ROOT): Promise<string> {
  say(`owner gate: no gap names goal \`${slug}\` (the acceptance list is green)`);
  const r = await run(["bun", "nv", "owners", "--closes", slug], { cwd: root, timeoutMs: 300_000 });
  const lines = `${r.stdout}\n${r.stderr}`.replace(/\r\n/g, "\n").split("\n").map((l) => l.trim()).filter(Boolean);
  const found = r.code === 0 ? "" : (lines.at(-1) ?? `\`bun nv owners --closes ${slug}\` exited ${r.code}`);
  writeGate(OWNER_GATE, found, found ? session : "", root);
  return found;
}



/** `nv disk --clean`'s sweep. Says each part, and returns the one-line summary for the caller's ledger. */
export async function sweepDisk(keepRuns: number, say: Say, root: string = ROOT): Promise<string> {
  const before = freeGb(root);
  let freed: Record<string, number | null>;
  try {
    freed = await clean({ keepRuns });
  } catch (e) {
    const failed = `disk: the sweep failed -- ${(e as Error).message}`;
    say(failed);
    return failed;
  }
  let summary = `disk: freed ${human(total(freed))}, ${before.toFixed(1)}G -> ${freeGb(root).toFixed(1)}G free`;
  for (const line of freedLines(freed)) say(`  ${line}`);
  if (freed["target/deps"] === null) summary += "; deps/ left alone, cargo could not name the live set";
  return summary;
}
