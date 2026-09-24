// `bun nv import --check`: reads every legacy home into records and writes nothing into the tree. The
// records are staged under `.cache/nv-import/`, where `nv check`'s own schema, foreign-key and
// invariant checks run over them, and the renderers render from them to compare with every rendered
// file on disk. Each line it prints is led by the legacy file it is about. It exits 1 when there is
// any, and while a record type has no importer, since the legacy homes are then not all read.
//
// `bun nv import --write` builds and checks the same records, and refuses, writing nothing, on
// anything `--check` would fail on except a stale rendered file. Otherwise it writes every record
// into `data/`, deletes a record file of a known type that the import no longer builds, and runs the
// renderers over the tree, which is what rewrites the generated files and the prose files' front
// matter.
//
// `tools/nv/import/known.json` declares where the import and the legacy tools disagree: a path, and
// why, for each file. A file it could not read wholly is declared there or it fails both modes; a
// declared path that no longer exists is a finding, so the list cannot outlive its files.
//
// `bun nv import --citations` lists every citation of a gap by its position in a tracked text file,
// with the gap it names today, for the cutover to rewrite into a slug. It always exits 0.

import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tracked } from "../lib/git.ts";
import { Index } from "../lib/index.ts";
import { CACHE, ROOT } from "../lib/paths.ts";
import { apply, type Output } from "../lib/render.ts";
import { idAt, pathOf, recordFiles, remove, write } from "../lib/store.ts";
import { citations, readGaps } from "../import/gaps.ts";
import { IMPORTERS } from "../import/index.ts";
import type { Imported, Unread } from "../import/lib.ts";
import { RENDERERS } from "../renderers/index.ts";
import { RECORDS } from "../schema/index.ts";

export const summary = "read the legacy homes into records: --check compares them, --write writes data/; --citations lists gaps cited by position";

const KNOWN = "tools/nv/import/known.json";

async function listCitations(): Promise<number> {
  const { positions } = readGaps(ROOT);
  let count = 0;
  let resolved = 0;
  for (const path of await tracked()) {
    let src: string;
    try {
      src = readFileSync(join(ROOT, path), "utf8");
    } catch {
      continue;
    }
    if (src.includes("\0") || !/known[- ]gap/i.test(src)) continue;
    for (const c of citations(ROOT, path, src.replace(/\r\n?/g, "\n"), positions)) {
      count++;
      if (c.gap) resolved++;
      const what = c.module === null ? "a module its wording does not name" : c.module;
      console.log(`${c.path}:${c.line}: known gap ${c.num} of ${what}: ${c.gap ?? "no gap today"}`);
    }
  }
  console.log(`nv import --citations: ${count} citation(s) of a gap by position, ${resolved} naming a gap today`);
  return 0;
}

interface Build {
  records: Imported[];
  unread: Unread[];
  declared: Unread[];
  files: number;
  findings: string[];
  stale: string[];
  rendered: number;
  missing: string[];
}

/** Every record the importers build, and everything `--check` says about them. */
async function build(): Promise<Build> {
  const records: Imported[] = [];
  const all: Unread[] = [];
  let files = 0;
  for (const imp of IMPORTERS) {
    const got = imp.read(ROOT);
    records.push(...got.records);
    all.push(...got.unread);
    files += got.files;
  }

  const findings: string[] = [];
  const known = JSON.parse(readFileSync(join(ROOT, KNOWN), "utf8")) as Record<string, string>;
  for (const path of Object.keys(known)) {
    if (!existsSync(join(ROOT, path))) findings.push(`${KNOWN}: declares ${path}, which does not exist`);
  }
  const unread = all.filter((u) => !(u.path in known));
  const declared = all.filter((u) => u.path in known);

  const stage = join(CACHE, "nv-import", crypto.randomUUID());
  let stale: string[] = [];
  let rendered = 0;
  try {
    const from = new Map<string, string>();
    for (const r of records) {
      const path = pathOf(r.type, r.id);
      if (from.has(path)) {
        findings.push(`${r.from}: ${path} is built a second time, first from ${from.get(path)}`);
        continue;
      }
      from.set(path, r.from);
      try {
        write(r.type, r.id, r.value, stage);
      } catch (e) {
        for (const line of (e as Error).message.split("\n")) findings.push(`${r.from}: ${path}: ${line}`);
      }
    }
    const index = new Index({ root: stage, types: RECORDS, prose: [], file: ":memory:" });
    try {
      index.refresh();
      for (const f of index.check()) findings.push(`${from.get(f.path) ?? f.path}: ${f.path}: ${f.message}`);
    } finally {
      index.close();
    }
    const outputs: Output[] = [];
    for (const r of RENDERERS) outputs.push(...(await r.render(stage)));
    rendered = outputs.length;
    stale = apply(outputs, { check: true, root: ROOT }).stale;
  } finally {
    rmSync(stage, { recursive: true, force: true });
  }

  const covered = new Set(records.map((r) => r.type.name));
  const missing = RECORDS.filter((t) => !covered.has(t.name)).map((t) => t.name);
  return { records, unread, declared, files, findings, stale, rendered, missing };
}

function report(b: Build, mode: string): void {
  for (const u of b.unread) console.log(`${u.path}: ${u.reason}`);
  for (const u of b.declared) console.log(`${u.path}: ${u.reason} (declared in ${KNOWN})`);
  for (const f of b.findings) console.log(f);
  if (b.missing.length > 0) console.log(`not imported yet: ${b.missing.join(", ")}`);
  const types = new Set(b.records.map((r) => r.type.name)).size;
  console.log(
    `nv import ${mode}: ${b.records.length} record(s) of ${types} type(s) from ${b.files} file(s); ` +
      `${b.unread.length} unread, ${b.declared.length} declared, ${b.findings.length} finding(s)`,
  );
}

export async function run(args: string[]): Promise<number> {
  if (args.length === 1 && args[0] === "--citations") return listCitations();
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    console.error("nv import: takes --check, --write or --citations");
    return 2;
  }
  const b = await build();
  const refused = b.unread.length + b.findings.length + b.missing.length > 0;

  if (mode === "--check") {
    report(b, mode);
    for (const path of b.stale) console.log(`${path}: is not what the imported records render`);
    console.log(`nv import --check: ${b.stale.length} of ${b.rendered} rendered file(s) stale`);
    return refused || b.stale.length > 0 ? 1 : 0;
  }

  report(b, mode);
  if (refused) {
    console.log("nv import --write: wrote nothing; every line above is read or declared first");
    return 1;
  }
  const built = new Set<string>();
  let changed = 0;
  for (const r of b.records) {
    const got = write(r.type, r.id, r.value, ROOT);
    built.add(got.path);
    if (got.changed) changed++;
  }
  let removed = 0;
  for (const path of recordFiles(ROOT)) {
    if (built.has(path)) continue;
    for (const type of RECORDS) {
      const id = idAt(type, path);
      if (id !== null && remove(type, id, ROOT)) removed++;
    }
  }
  const outputs: Output[] = [];
  for (const r of RENDERERS) outputs.push(...(await r.render(ROOT)));
  const { stale } = apply(outputs, { check: false, root: ROOT });
  for (const path of stale) console.log(`${path}: rewritten`);
  console.log(
    `nv import --write: ${changed} of ${built.size} record file(s) written, ${removed} removed, ` +
      `${stale.length} of ${outputs.length} rendered file(s) rewritten`,
  );
  return 0;
}
