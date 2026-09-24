// `bun nv impact --probe [<probe>...]`: holds every unit's key to `data/impact-probes.json`. Each probe
// is one synthetic edit, applied to an in-memory copy of the tree with `Tree.edited`, and two lists of
// patterns over `<role>: <name>` (`tools/nv/keys/checks.ts` says what each role is). Every unit a
// `rerun` pattern matches must move, and every other unit a `keep` pattern matches must not. A
// pattern that matches no unit is an error, so a renamed check cannot empty a probe without saying so.
//
// A unit keyed on everything moves on every edit. A probe counts it as `wide` rather than failing on
// it, because a wide key is never an unsafe one; `tools/data/impact-wide.txt` is the list of test
// binaries that are wide, and the reason each is. `--show <probe>` prints every unit the edit moves.
//
// Exits 0 when every probe holds, 1 when one does not, and 2 on a bad argument or a probe file the
// schema or this command refuses.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { DATA } from "../lib/paths.ts";
import { type Unit, isWide, loadRecords, units } from "../keys/checks.ts";
import { metadata } from "../keys/graph.ts";
import { keyOf } from "../keys/key.ts";
import { Tree } from "../keys/tree.ts";
import { impactProbes } from "../schema/impact-probes.ts";

export const summary = "hold every key to its probes: nv impact --probe [<probe>...] | --show <probe>";

const PROBES = join(DATA, "impact-probes.json");

export interface Edit {
  path: string;
  /** Anchors found one after another; the text goes in right after the last. None: the file's end. */
  after?: string[];
  insert?: string;
  /** A new file's text. */
  create?: string;
}

export interface Probe {
  name: string;
  what: string;
  edit: Edit;
  rerun: string[];
  keep: string[];
}

/** The file the edit writes, and its text after the edit. Throws on an anchor the file lacks. */
export function applyEdit(tree: Tree, e: Edit): Record<string, string> {
  if (e.create !== undefined) {
    if (tree.has(e.path)) throw new Error(`${e.path} exists, and a probe creates it`);
    return { [e.path]: e.create };
  }
  if (e.insert === undefined) throw new Error(`${e.path}: an edit needs \`insert\` or \`create\``);
  const text = tree.text(e.path);
  let at = text.length;
  if (e.after && e.after.length > 0) {
    at = 0;
    for (const anchor of e.after) {
      const found = text.indexOf(anchor, at);
      if (found < 0) throw new Error(`${e.path}: anchor ${JSON.stringify(anchor)} not found`);
      at = found + anchor.length;
    }
  }
  return { [e.path]: text.slice(0, at) + e.insert + text.slice(at) };
}

export const label = (u: Unit) => `${u.role}: ${u.name}`;

export interface Verdict {
  failures: string[];
  rerun: number;
  kept: number;
  wide: number;
  moved: string[];
}

/** Judges one probe from each unit's key before and after, and whether its key after is wide. */
export function judge(p: Probe, us: Unit[], moved: Set<Unit>, wide: Set<Unit>): Verdict {
  const rerun = p.rerun.map((r) => new RegExp(r));
  const keep = p.keep.map((k) => new RegExp(k));
  const failures: string[] = [];
  const hits = (res: RegExp[]) => res.map(() => 0);
  const rerunHits = hits(rerun);
  const keepHits = hits(keep);
  let nRerun = 0;
  let nKept = 0;
  let nWide = 0;
  for (const u of us) {
    const l = label(u);
    let isRerun = false;
    rerun.forEach((re, i) => {
      if (re.test(l)) {
        rerunHits[i]!++;
        isRerun = true;
      }
    });
    if (isRerun) {
      nRerun++;
      if (!moved.has(u)) failures.push(`does not re-run: ${l}`);
      continue;
    }
    let isKeep = false;
    keep.forEach((re, i) => {
      if (re.test(l)) {
        keepHits[i]!++;
        isKeep = true;
      }
    });
    if (!isKeep) continue;
    if (wide.has(u)) nWide++;
    else if (moved.has(u)) failures.push(`re-runs: ${l}`);
    else nKept++;
  }
  rerunHits.forEach((n, i) => n === 0 && failures.push(`rerun pattern matches no unit: ${p.rerun[i]}`));
  keepHits.forEach((n, i) => n === 0 && failures.push(`keep pattern matches no unit: ${p.keep[i]}`));
  const movedLabels = us.filter((u) => moved.has(u)).map(label);
  return { failures, rerun: nRerun, kept: nKept, wide: nWide, moved: movedLabels };
}

function loadProbes(): Probe[] | string {
  let doc: unknown;
  try {
    doc = JSON.parse(readFileSync(PROBES, "utf8"));
  } catch (e) {
    return `${PROBES}: ${(e as Error).message}`;
  }
  const issues = impactProbes.schema.validate(doc);
  if (issues.length > 0) return issues.map((i) => `data/impact-probes.json${i.at}: ${i.message}`).join("\n");
  const probes = (doc as { probes: Probe[] }).probes;
  for (const p of probes) if (p.rerun.length === 0) return `data/impact-probes.json: probe ${p.name} has an empty \`rerun\`, and a probe must name what re-runs`;
  return probes;
}

export async function run(args: string[]): Promise<number> {
  const show = args[0] === "--show" ? args[1] : undefined;
  if (!(args[0] === "--probe" || (show !== undefined && args.length === 2))) {
    console.error("nv impact: --probe [<probe>...] | --show <probe>");
    return 2;
  }
  const all = loadProbes();
  if (typeof all === "string") {
    console.error(`nv impact: ${all}`);
    return 2;
  }
  const names = show !== undefined ? [show] : args.slice(1);
  for (const n of names) {
    if (!all.some((p) => p.name === n)) {
      console.error(`nv impact: no probe ${n}; the probes are ${all.map((p) => p.name).join(", ")}`);
      return 2;
    }
  }
  const probes = names.length > 0 ? all.filter((p) => names.includes(p.name)) : all;

  const graph = await metadata();
  if (!graph) {
    console.error("nv impact: `cargo metadata` failed, and every key needs the graph");
    return 2;
  }
  const tree = await Tree.read();
  const us = units(loadRecords(graph));
  const before = new Map(us.map((u) => [u, keyOf(u.name, u.parts(tree))]));

  let failed = 0;
  for (const p of probes) {
    let after: Tree;
    try {
      after = tree.edited(applyEdit(tree, p.edit));
    } catch (e) {
      console.log(`${p.name}: FAILS\n  the edit: ${(e as Error).message}`);
      failed++;
      continue;
    }
    const moved = new Set<Unit>();
    const wide = new Set<Unit>();
    for (const u of us) {
      const parts = u.parts(after);
      if (keyOf(u.name, parts) !== before.get(u)) moved.add(u);
      if (isWide(parts)) wide.add(u);
    }
    const v = judge(p, us, moved, wide);
    if (show !== undefined) {
      console.log(`${p.name}: ${v.moved.length} unit(s) move`);
      for (const l of v.moved) console.log(`  ${wide.has(us.find((u) => label(u) === l)!) ? "wide " : ""}${l}`);
      return 0;
    }
    if (v.failures.length === 0) {
      console.log(`${p.name}: holds -- ${v.rerun} re-run, ${v.kept} kept, ${v.wide} wide`);
    } else {
      failed++;
      console.log(`${p.name}: FAILS`);
      for (const f of v.failures.slice(0, 12)) console.log(`  ${f}`);
      if (v.failures.length > 12) console.log(`  ... and ${v.failures.length - 12} more`);
    }
  }
  tree.save();
  if (failed === 0) {
    console.log("impact probes: every probe holds");
    return 0;
  }
  console.log(`impact probes: ${failed} of ${probes.length} fail`);
  return 1;
}
