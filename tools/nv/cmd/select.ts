// `bun nv select`: which atoms a change reaches, from what each atom was observed to use.
//
//     bun nv select                      the atoms the change since the recorded tree selects, and why
//     bun nv select --since REV          compare the working tree with REV instead
//     bun nv select --since A --until B  replay the change from commit A to commit B
//     bun nv select --paths A B ...      take these paths as the change instead of asking git
//     bun nv select --stats              counts only: atoms per kind and reason, keys per kind
//     bun nv select --json               the selection as one JSON document
//     bun nv select --explain ATOM       why one atom was selected or not: the key, the path, the item
//     bun nv select --seed               record every atom from nothing (builds covws; long)
//          [--kinds case,proof,test,nv] [--limit N] [--jobs N]
//
// `tools/nv/select/` is the engine; `select.ts` there holds the rule a selection follows, and `store.ts`
// the store in `.cache/select.sqlite`. An atom is `case:<path>`, `proof:<path>`, `test:<package> <kind>
// <target>` or `nv:<check id>`.

import { metadata } from "../keys/graph.ts";
import { type AtomKind, ATOM_KINDS, SelectStore } from "../select/store.ts";
import { computeChange, counts, describe, discover, explain, query } from "../select/select.ts";
import { seed } from "../select/seed.ts";

export const summary =
  "which atoms a change reaches, from what each was seen to use: nv select [--since REV [--until REV]] [--paths P...] [--stats] [--json] [--explain ATOM] [--seed [--kinds K,...] [--limit N] [--jobs N] [--resume]]";

const USAGE = `usage: bun nv select [--since REV [--until REV]] [--paths PATH ...] [--stats] [--json] [--explain ATOM]
       bun nv select --seed [--kinds case,proof,test,nv] [--limit N] [--jobs N]

  --since REV     compare the working tree with REV; the store's recorded tree by default
  --until REV     take commit REV as the changed side instead of the working tree
  --paths P ...   take these paths as the change instead of asking git
  --stats         print counts only: atoms selected per kind and reason, keys moved per kind
  --json          print the selection as JSON
  --explain ATOM  say why ATOM was selected or not
  --seed          build covws and record every atom from nothing
  --kinds K,...   with --seed: only these kinds (case, proof, test, nv)
  --limit N       with --seed: at most N atoms of each kind
  --jobs N        with --seed: how many runs and extractions at once
  --resume        with --seed: skip every atom that already has a footprint`;

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
  help: boolean;
}

export function parse(args: string[]): Opts {
  const o: Opts = { stats: false, json: false, seed: false, help: false };
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
    else throw new Error(`unknown argument ${a}`);
  }
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
    const graph = await metadata();
    const change = await computeChange(store, { ...(o.since ? { since: o.since } : {}), ...(o.until ? { until: o.until } : {}), ...(o.paths ? { paths: o.paths } : {}), graph });
    const sel = query(store, change, { discovered: await discover(graph) });
    if (o.explain) {
      for (const line of explain(store, sel, o.explain)) console.log(line);
      return 0;
    }
    const c = counts(sel);
    if (o.json) {
      const doc: Record<string, unknown> = { since: change.since, ...c, global: change.global, gone: sel.gone };
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
