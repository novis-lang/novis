// `bun nv select`: which atoms a change reaches, from what each atom was observed to use.
//
//     bun nv select                      the atoms the change since the recorded tree selects, and why
//     bun nv select --since REV          compare the working tree with REV instead
//     bun nv select --since A --until B  replay the change from commit A to commit B
//     bun nv select --paths A B ...      take these paths as the change instead of asking git
//     bun nv select --stats              counts only: atoms per kind and reason, keys per kind, and
//                                        the atoms marked diverged
//     bun nv select --json               the selection as one JSON document
//     bun nv select --explain ATOM       why one atom was selected or not: the key, the path, the item
//     bun nv select --explain CHECK      what a plan check is made of, and why each of its atoms was
//                                        selected or not; CHECK is its id or its name
//     bun nv select --seed               record every atom from nothing (builds covws; long)
//          [--kinds case,proof,test,nv] [--limit N] [--jobs N]
//     bun nv select --full [--jobs N]    run and record every atom that is not heavy, move the tree, and
//                                        report the selection misses; exits 1 on one (`select/full.ts`)
//     bun nv select --mutate FILE        apply each batch of FILE's edits, run every atom on a copy of the
//          [--batch N] [--jobs N]        store, and require every red atom to have been selected
//                                        (`select/mutate.ts`); exits 1 on a miss or a failed batch
//
// `tools/nv/select/` is the engine; `select.ts` there holds the rule a selection follows, and `store.ts`
// the store in `.cache/select.sqlite`. An atom is `case:<path>`, `proof:<path>`, `test:<package> <kind>
// <target>`, `nvtest:<file>`, `step:<name>`, or a plan check's own `check:`, `nv:` or `heavy:<check id>`.
// A plan check is a group of atoms (`select/checks.ts`); what it is made of does not depend on what
// changed, so `--explain CHECK` prints that first, whether or not the store has a tree to compare with.

import type { Check } from "../driver/accept.ts";
import { metadata } from "../keys/graph.ts";
import { currentPlan } from "../lib/chain.ts";
import { describeGroup, grouped, planContext } from "../select/checks.ts";
import { type AtomKind, ATOM_KINDS, SelectStore } from "../select/store.ts";
import { computeChange, counts, describe, discover, explain, query } from "../select/select.ts";
import { seed } from "../select/seed.ts";
import { describeFull, fullRun } from "../select/full.ts";
import { describeBatch, mutateBatch, readMutations } from "../select/mutate.ts";
import { abs } from "../lib/paths.ts";

export const summary =
  "which atoms a change reaches, from what each was seen to use: nv select [--since REV [--until REV]] [--paths P...] [--stats] [--json] [--explain ATOM] [--seed [--kinds K,...] [--limit N] [--jobs N] [--resume]] [--full] [--mutate FILE [--batch N]]";

const USAGE = `usage: bun nv select [--since REV [--until REV]] [--paths PATH ...] [--stats] [--json] [--explain ATOM]
       bun nv select --seed [--kinds case,proof,test,nv] [--limit N] [--jobs N]
       bun nv select --full [--jobs N]
       bun nv select --mutate FILE [--batch N] [--jobs N]

  --since REV     compare the working tree with REV; the store's recorded tree by default
  --until REV     take commit REV as the changed side instead of the working tree
  --paths P ...   take these paths as the change instead of asking git
  --stats         print counts only: atoms selected per kind and reason, keys moved per kind
  --json          print the selection as JSON
  --explain ATOM  say why ATOM was selected or not; a plan check's id or name says what it is made of
  --seed          build covws and record every atom from nothing
  --kinds K,...   with --seed: only these kinds (case, proof, test, nv)
  --limit N       with --seed: at most N atoms of each kind
  --jobs N        with --seed: how many runs and extractions at once
  --resume        with --seed: skip every atom that already has a footprint
  --full          run and record every atom that is not heavy, move the recorded tree, and report each
                  selection miss: an atom red now, green before, that the selection did not pick
  --mutate FILE   apply each batch of edits FILE lists, run every atom that is not heavy on a copy of
                  the store, and require every red atom to have been selected; the edits are reverted
  --batch N       with --mutate: only batch N, counted from 1`;

interface Opts {
  since?: string;
  until?: string;
  paths?: string[];
  stats: boolean;
  json: boolean;
  explain?: string;
  seed: boolean;
  resume?: boolean;
  kinds?: AtomKind[];
  limit?: number;
  jobs?: number;
  full: boolean;
  mutate?: string;
  batch?: number;
  help: boolean;
}

export function parse(args: string[]): Opts {
  const o: Opts = { stats: false, json: false, seed: false, full: false, help: false };
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    const value = () => {
      const v = args[++i];
      if (v === undefined || v.startsWith("--")) throw new Error(`${a} needs a value`);
      return v;
    };
    if (a === "-h" || a === "--help") o.help = true;
    else if (a === "--since") o.since = value();
    else if (a === "--until") o.until = value();
    else if (a === "--paths") {
      o.paths = [];
      while (i + 1 < args.length && !args[i + 1]!.startsWith("--")) o.paths.push(args[++i]!);
      if (o.paths.length === 0) throw new Error("--paths needs at least one path");
    } else if (a === "--stats") o.stats = true;
    else if (a === "--json") o.json = true;
    else if (a === "--explain") o.explain = value();
    else if (a === "--seed") o.seed = true;
    else if (a === "--resume") o.resume = true;
    else if (a === "--kinds") {
      const kinds = value().split(",") as AtomKind[];
      const bad = kinds.find((k) => !ATOM_KINDS.includes(k));
      if (bad) throw new Error(`--kinds: no atom kind ${bad}`);
      o.kinds = kinds;
    } else if (a === "--limit") o.limit = Number.parseInt(value(), 10);
    else if (a === "--jobs") o.jobs = Number.parseInt(value(), 10);
    else if (a === "--full") o.full = true;
    else if (a === "--mutate") o.mutate = value();
    else if (a === "--batch") {
      o.batch = Number.parseInt(value(), 10);
      if (!(o.batch >= 1)) throw new Error("--batch needs a number from 1");
    } else throw new Error(`unknown argument ${a}`);
  }
  if (o.batch !== undefined && o.mutate === undefined) throw new Error("--batch goes with --mutate");
  if ([o.seed, o.full, o.mutate !== undefined].filter(Boolean).length > 1) throw new Error("--seed, --full and --mutate each run on their own");
  return o;
}

export async function run(args: string[]): Promise<number> {
  let o: Opts;
  try {
    o = parse(args);
  } catch (e) {
    console.error(`${USAGE}\nnv select: error: ${(e as Error).message}`);
    return 2;
  }
  if (o.help) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  if (o.mutate !== undefined) return mutate(o.mutate, o.batch, o.jobs);
  const store = new SelectStore();
  try {
    if (o.seed) {
      const r = await seed(store, { ...(o.kinds ? { kinds: o.kinds } : {}), ...(o.limit ? { limit: o.limit } : {}), ...(o.jobs ? { jobs: o.jobs } : {}), ...(o.resume ? { resume: true } : {}) });
      const s = store.stats();
      if (o.json) console.log(JSON.stringify({ seed: r, store: s }, null, 2));
      else {
        console.log(`select: seeded at ${r.base.slice(0, 12)} in ${r.seconds}s`);
        for (const [kind, k] of Object.entries(r.kinds)) console.log(`  ${kind.padEnd(6)} ${k.run} run, ${k.red} red, ${k.processes} processes, ${k.executed} functions ran, ${k.unmapped} unmapped`);
        console.log(`  store: ${s.entries} footprint entries over ${s.keys} keys, ${(s.bytes / 1e6).toFixed(1)} MB`);
      }
      return 0;
    }
    if (o.full) {
      const r = await fullRun(store, { ...(o.jobs ? { jobs: o.jobs } : {}) });
      if (o.json) console.log(JSON.stringify({ since: r.since, selected: r.selection.selected.size, kinds: r.kinds, owed: r.owed, misses: r.misses, seconds: r.seconds, buildFailed: r.buildFailed ?? null }, null, 2));
      else for (const line of describeFull(r)) console.log(line);
      return r.buildFailed || r.misses.length > 0 ? 1 : 0;
    }
    const graph = await metadata();
    const plan = o.explain ? currentPlan() : null;
    const check = plan ? (plan.goal.checks as Check[]).find((c) => c.id === o.explain || c.name === o.explain) : undefined;
    let atoms: string[] = [];
    if (check) {
      const g = grouped(check, planContext(store, graph));
      for (const line of describeGroup(check, g, graph)) console.log(line);
      atoms = g.atoms;
      if (store.base() === null && !o.since) {
        console.log("  the store has no recorded tree, so every atom of it runs");
        return 0;
      }
    }
    const change = await computeChange(store, { ...(o.since ? { since: o.since } : {}), ...(o.until ? { until: o.until } : {}), ...(o.paths ? { paths: o.paths } : {}), graph });
    const found = discover(graph);
    const sel = query(store, change, { discovered: found.atoms, complete: found.complete });
    if (check) {
      for (const a of atoms.slice(0, 10)) console.log(`  ${explain(store, sel, a)[0]}`);
      if (atoms.length > 10) console.log(`  ... and ${atoms.length - 10} more atom(s)`);
      return 0;
    }
    if (o.explain) {
      for (const line of explain(store, sel, o.explain)) console.log(line);
      return 0;
    }
    const c = counts(sel);
    const diverged = [...store.divergences()].sort(([a], [b]) => (a < b ? -1 : 1));
    if (o.json) {
      const doc: Record<string, unknown> = { since: change.since, ...c, global: change.global, gone: sel.gone, diverged: Object.fromEntries(diverged) };
      if (!o.stats) {
        doc.atoms = [...sel.selected.values()].map((s) => ({ id: s.id, why: s.why, keys: s.keys.slice(0, 5).map((k) => ({ key: k.key, from: describe(k.origin) })) }));
        doc.changes = change.changes;
      }
      console.log(JSON.stringify(doc, null, 2));
      return 0;
    }
    const total = Object.values(c.selected).reduce((a, b) => a + b, 0);
    const known = Object.values(c.known).reduce((a, b) => a + b, 0);
    console.log(`select: ${c.changes} path(s) changed since ${change.since.slice(0, 12)}, ${c.itemChanges} item change(s) in ${c.rustFiles} Rust file(s)${change.global ? `; ${change.global} is global` : ""}`);
    console.log(`select: ${total} of ${known} atom(s) selected`);
    for (const [kind, n] of Object.entries(c.selected)) console.log(`  ${kind.padEnd(6)} ${n} of ${c.known[kind] ?? 0}`);
    for (const [why, byKind] of Object.entries(c.byWhy)) console.log(`  because ${why}: ${Object.entries(byKind).map(([k, n]) => `${n} ${k}`).join(", ")}`);
    if (diverged.length > 0) {
      console.log(`select: ${diverged.length} atom(s) marked diverged: the recording run ended differently from the judged run, so each runs every time`);
      for (const [id, why] of diverged) console.log(`  diverged  ${id}: ${why}`);
    }
    if (o.stats) return 0;
    for (const s of sel.selected.values()) {
      const first = s.keys[0];
      console.log(`${s.id}  ${s.why}${first ? `  ${first.key} <- ${describe(first.origin)}` : ""}`);
    }
    return 0;
  } finally {
    store.close();
  }
}

/** `--mutate FILE [--batch N]`: each batch, or batch N alone, through `mutateBatch`; 1 when a batch
 * failed or missed. */
async function mutate(file: string, only: number | undefined, jobs: number | undefined): Promise<number> {
  const doc = readMutations(abs(file));
  if (only !== undefined && only > doc.batches.length) {
    console.error(`nv select: error: ${file} has ${doc.batches.length} batch(es), and no batch ${only}`);
    return 2;
  }
  let bad = 0;
  for (let i = 0; i < doc.batches.length; i++) {
    if (only !== undefined && i + 1 !== only) continue;
    console.log(`select: batch ${i + 1} of ${doc.batches.length}`);
    const rep = await mutateBatch(i + 1, doc.batches[i]!, {
      run: (store) => fullRun(store, { ...(jobs ? { jobs } : {}), say: (l) => console.error(`  ${l}`) }),
      explain,
    });
    for (const line of describeBatch(rep)) console.log(line);
    if (rep.failed || rep.misses.length > 0) bad++;
  }
  console.log(bad === 0 ? "select: every red atom of every batch was selected" : `select: ${bad} batch(es) failed or missed`);
  return bad === 0 ? 0 : 1;
}
