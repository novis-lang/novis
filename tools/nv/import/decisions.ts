// The decision records' legacy homes: each `docs/decisions/NNNN.md`'s YAML block, its H1 and the bold
// field bullets under it, and the plain-language entry `docs/decisions.toml` holds for it. They become
// `data/decisions/NNNN.json`.
//
// A record's `changes:` block is not imported, because the rules' `because` lists say the same thing:
// the first decision a rule names created it and the rest amended it. What is imported instead is the
// check that the two agree, so a record whose block says otherwise is reported rather than lost.

import { parse as parseToml } from "smol-toml";
import { load } from "../lib/store.ts";
import { decision } from "../schema/decision.ts";
import { rule } from "../schema/rule.ts";
import { exists, extraKeys, list, text, type Importer, type ImportResult, type Unread } from "./lib.ts";

const DIR = "docs/decisions";
const SUMMARIES = "docs/decisions.toml";
const FIELDS = { "Scope": "scope", "Depends on": "dependsOn", "Validated by": "validatedBy" } as const;
/** A link to another record from inside `docs/decisions/`. */
const RECORD_LINK = /\[[^\]]*\]\((?:\.\/)?(\d{4})\.md(?:#[^)]*)?\)/g;

interface Changes {
  creates: string[];
  modifies: string[];
}

/** The `status` and `changes` of a record's YAML block, which is the only nesting it has. */
function yamlBlock(block: string, path: string, unread: Unread[]): { status: string; changes: Changes } {
  let status = "";
  const changes: Changes = { creates: [], modifies: [] };
  let list: string[] | null = null;
  for (const line of block.split("\n")) {
    let m: RegExpExecArray | null;
    if ((m = /^status: (\S+)$/.exec(line))) status = m[1]!;
    else if (line === "changes:") list = null;
    else if ((m = /^  (creates|modifies):( \[\])?$/.exec(line))) list = changes[m[1] as keyof Changes];
    else if ((m = /^    - (\S+)$/.exec(line)) && list) list.push(m[1]!);
    else if (line.trim() !== "") unread.push({ path, reason: `its YAML block has a line no field reads: ${line.trim()}` });
  }
  return { status, changes };
}

/**
 * The bold field bullets under the H1, each value with its line breaks kept. They end at the first
 * line that is neither a field, a field's indented continuation nor blank, which is where the body
 * the record leaves as prose begins.
 */
function fieldBullets(lines: string[], path: string, unread: Unread[]): Map<string, string> {
  const out = new Map<string, string>();
  let current: string | null = null;
  for (const line of lines) {
    const m = /^- \*\*([A-Z][a-z ]*):\*\* ?(.*)$/.exec(line);
    if (m) {
      current = m[1]!;
      if (!(current in FIELDS)) unread.push({ path, reason: `field **${current}:** is not one a record holds` });
      out.set(current, m[2]!);
    } else if (current !== null && line.startsWith("  ")) {
      out.set(current, `${out.get(current)}\n${line.slice(2)}`);
    } else if (line.trim() === "") {
      current = null;
    } else {
      break;
    }
  }
  return out;
}

interface Summary {
  adr: string;
  group: string;
  headline: string;
  body: string;
  pin?: boolean;
}

export const decisions: Importer = {
  name: "decisions",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 0 };
    const summaries = new Map<string, Summary>();
    if (exists(root, SUMMARIES)) {
      out.files++;
      const doc = parseToml(text(root, SUMMARIES)) as { entry?: Summary[] };
      out.unread.push(...extraKeys(SUMMARIES, "the file", doc, ["entry"]));
      for (const e of doc.entry ?? []) {
        out.unread.push(...extraKeys(SUMMARIES, `entry ${e.adr}`, e, ["adr", "group", "headline", "body", "pin", "digest"]));
        if (summaries.has(e.adr)) out.unread.push({ path: SUMMARIES, reason: `a second entry for ${e.adr}` });
        summaries.set(e.adr, e);
      }
    }
    const declared = new Map<string, Changes>();
    for (const name of list(root, DIR)) {
      const path = `${DIR}/${name}`;
      if (!/^\d{4}\.md$/.test(name)) {
        if (name.endsWith(".md")) out.unread.push({ path, reason: "is not named NNNN.md, so it is no decision record" });
        continue;
      }
      out.files++;
      const id = name.slice(0, 4);
      const src = text(root, path);
      const front = /^---\n([\s\S]*?)\n---\n/.exec(src);
      if (!front) {
        out.unread.push({ path, reason: "has no YAML block" });
        continue;
      }
      const { status, changes } = yamlBlock(front[1]!, path, out.unread);
      declared.set(id, changes);
      const lines = src.slice(front[0].length).split("\n");
      const h1 = /^# ADR (\d{4}) — (.+)$/.exec(lines[0] ?? "");
      if (!h1 || h1[1] !== id) {
        out.unread.push({ path, reason: `does not open on \`# ADR ${id} — <title>\`` });
        continue;
      }
      const fields = fieldBullets(lines.slice(1), path, out.unread);
      const value: Record<string, unknown> = { title: h1[2], status, scope: fields.get("Scope") ?? "", dependsOn: [] };
      const depends = fields.get("Depends on");
      if (depends !== undefined) {
        value.dependsOn = [...depends.matchAll(RECORD_LINK)].map((m) => m[1]!);
        const rest = depends.replace(RECORD_LINK, "").replace(/\band\b|[\s,;.]/g, "");
        if (rest !== "") value.dependsOnText = depends;
      }
      const validated = fields.get("Validated by");
      if (validated !== undefined) value.validatedBy = validated;
      const summary = summaries.get(id);
      if (summary) {
        value.summary = { group: summary.group, headline: summary.headline, body: summary.body.trimEnd() };
        if (summary.pin) (value.summary as Record<string, unknown>).pin = true;
        summaries.delete(id);
      }
      out.records.push({ type: decision, id, value, from: path });
    }
    for (const adr of summaries.keys()) out.unread.push({ path: SUMMARIES, reason: `entry ${adr} names no record` });
    out.unread.push(...changesAgree(root, declared));
    return out;
  },
};

/** Every record whose `changes:` block is not what the rule records' `because` lists derive. */
function changesAgree(root: string, declared: Map<string, Changes>): Unread[] {
  const derived = new Map<string, Changes>();
  const of = (id: string) => derived.get(id) ?? derived.set(id, { creates: [], modifies: [] }).get(id)!;
  for (const r of load(rule, root)) {
    const because = (r.value as { because?: string[] } | undefined)?.because ?? [];
    because.forEach((adr, i) => of(adr)[i === 0 ? "creates" : "modifies"].push(r.id));
  }
  const out: Unread[] = [];
  const same = (a: string[], b: string[]) => a.length === b.length && [...a].sort().join() === [...b].sort().join();
  for (const [id, changes] of declared) {
    const d = derived.get(id) ?? { creates: [], modifies: [] };
    for (const k of ["creates", "modifies"] as const) {
      if (!same(changes[k], d[k])) {
        out.push({ path: `${DIR}/${id}.md`, reason: `\`changes.${k}\` is not what the rules' \`because\` lists derive` });
      }
    }
  }
  return out;
}
