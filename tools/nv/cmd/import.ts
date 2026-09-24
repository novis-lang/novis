// `bun nv import --check`: reads every legacy home into records and writes nothing into the tree. The
// records are staged under `.cache/nv-import/`, where `nv check`'s own schema, foreign-key and
// invariant checks run over them, and the renderers render from them to compare with every rendered
// file on disk. Each line it prints is led by the legacy file it is about. It exits 1 when there is
// any, and while a record type has no importer, since the legacy homes are then not all read.

import { rmSync } from "node:fs";
import { join } from "node:path";
import { Index } from "../lib/index.ts";
import { CACHE, ROOT } from "../lib/paths.ts";
import { apply, type Output } from "../lib/render.ts";
import { pathOf, write } from "../lib/store.ts";
import { IMPORTERS } from "../import/index.ts";
import type { Imported, Unread } from "../import/lib.ts";
import { RENDERERS } from "../renderers/index.ts";
import { RECORDS } from "../schema/index.ts";

export const summary = "read the legacy homes into records: --check compares them, writing nothing";

export async function run(args: string[]): Promise<number> {
  if (args.length !== 1 || args[0] !== "--check") {
    console.error(`nv import: takes --check${args.includes("--write") ? "; --write is not built yet" : ""}`);
    return 2;
  }
  const records: Imported[] = [];
  const unread: Unread[] = [];
  let files = 0;
  for (const imp of IMPORTERS) {
    const got = imp.read(ROOT);
    records.push(...got.records);
    unread.push(...got.unread);
    files += got.files;
  }

  const stage = join(CACHE, "nv-import", crypto.randomUUID());
  const findings: string[] = [];
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

  for (const u of unread) console.log(`${u.path}: ${u.reason}`);
  for (const f of findings) console.log(f);
  for (const path of stale) console.log(`${path}: is not what the imported records render`);
  const covered = new Set(records.map((r) => r.type.name));
  const missing = RECORDS.filter((t) => !covered.has(t.name)).map((t) => t.name);
  if (missing.length > 0) console.log(`not imported yet: ${missing.join(", ")}`);
  console.log(
    `nv import --check: ${records.length} record(s) of ${covered.size} type(s) from ${files} file(s); ` +
      `${unread.length} unread, ${findings.length} finding(s), ${stale.length} of ${rendered} rendered file(s) stale`,
  );
  return unread.length + findings.length + stale.length + missing.length > 0 ? 1 : 0;
}
