// A handoff's Markdown as its record. `bun nv session --wrap` writes the wrap file's `## handoff` section
// to a scratch file and reads it here into `data/goals/<slug>.handoff.json`, or the side goal's.
//
// The Markdown is `# Handoff`, then `## State`, `## Next group` and an optional `## Backlog`. A
// `## Next group` is a lead paragraph, a checklist and what follows it.

import { readFileSync } from "node:fs";
import { join } from "node:path";

/** A file, or a part of one, that did not become a record as it stands, and why. */
export interface Unread {
  path: string;
  reason: string;
}

/** `lines` as Markdown paragraphs, each unwrapped to one line, or "" for none. */
function unwrap(lines: string[]): string {
  return lines
    .join("\n")
    .split(/\n\s*\n/)
    .map((p) => p.split("\n").map((l) => l.trim()).filter((l) => l !== "").join(" "))
    .filter((p) => p !== "")
    .join("\n\n");
}

/**
 * The list that opens `lines`, its items opening on `- ` and each unwrapped to one line, and the
 * lines after it: the list ends at the first line that neither opens nor continues an item.
 */
function listItems(lines: string[]): { items: string[]; rest: string[] } {
  const items: string[] = [];
  let open = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    if (line.startsWith("- ")) {
      items.push(line.slice(2).trim());
      open = true;
    } else if (line.trim() === "") {
      open = false;
    } else if (open && /^\s/.test(line)) {
      items[items.length - 1] += ` ${line.trim()}`;
    } else {
      return { items, rest: lines.slice(i) };
    }
  }
  return { items, rest: [] };
}

/** Where the first `.` that ends a sentence outside backticks is in `s`, or -1. */
function sentenceEnd(s: string): number {
  let code = false;
  for (let i = 0; i < s.length; i++) {
    if (s[i] === "`") code = !code;
    else if (!code && s[i] === "." && (i + 1 === s.length || s[i + 1] === " ")) return i;
  }
  return -1;
}

/**
 * The group's lead line, `**Stage N: title** — one file set: `a`, `b`. More.` The file set is the
 * backticked paths up to the first full stop, and `note` what follows; a file set that is more than
 * a list of paths stays whole in `note`, with `files` empty. A lead that opens on no bold name is
 * all `note`.
 */
function leadOf(lead: string): { stage: number | null; title?: string; files: string[]; note?: string } {
  const m = /^\*\*(?:Stage (\d+): )?(.+?)\*\*\s*(.*)$/s.exec(lead);
  if (!m) return { stage: null, files: [], ...(lead === "" ? {} : { note: lead }) };
  const out = { stage: m[1] === undefined ? null : Number(m[1]), title: m[2]!.replace(/\.$/, "") };
  const rest = m[3]!.replace(/^—\s*/, "");
  const set = /^[Oo]ne file set: (.*)$/.exec(rest);
  if (set) {
    const end = sentenceEnd(set[1]!);
    const clause = end < 0 ? set[1]! : set[1]!.slice(0, end);
    const paths = clause.split(/,\s+(?:and\s+)?|\s+and\s+/);
    if (paths.every((p) => /^`[^`]+`$/.test(p))) {
      const note = end < 0 ? "" : set[1]!.slice(end + 1).trim();
      return { ...out, files: paths.map((p) => p.slice(1, -1)), ...(note === "" ? {} : { note }) };
    }
  }
  return { ...out, files: [], ...(rest === "" ? {} : { note: rest }) };
}

/**
 * The handoff record the repo-relative `path` holds, or null with the reason in `unread`.
 *
 * A state that opens on `**Goal N — ` names the goal by its position. A record carries no position,
 * since `data/chain.json` is where one is computed from, so the state names the goal by `slug` instead.
 */
export function handoffValue(root: string, path: string, slug: string, unread: Unread[]): Record<string, unknown> | null {
  const parts = readFileSync(join(root, path), "utf8").replace(/\r\n?/g, "\n").split(/^## /m);
  const head = parts.shift()!.trim();
  if (head !== "# Handoff") unread.push({ path, reason: "it does not open on `# Handoff` alone" });
  const sections = new Map<string, string[]>();
  for (const p of parts) {
    const [heading, ...body] = p.split("\n");
    sections.set(heading!.trim(), body);
  }
  const extra = [...sections.keys()].filter((k) => !["State", "Next group", "Backlog"].includes(k));
  if (extra.length > 0) unread.push({ path, reason: `its sections ${extra.join(", ")} have no field` });
  const state = sections.get("State");
  const next = sections.get("Next group");
  if (state === undefined || next === undefined) {
    unread.push({ path, reason: "it has no `## State` or no `## Next group`" });
    return null;
  }

  const first = next.findIndex((l) => l.startsWith("- "));
  const lead = leadOf(unwrap(first < 0 ? next : next.slice(0, first)));
  const list = listItems(first < 0 ? [] : next.slice(first));
  const checklist = list.items.map((i) => {
    const m = /^\[( |x)\] (.*)$/.exec(i);
    return m ? { done: m[1] === "x", text: m[2]! } : null;
  });
  const after = unwrap(list.rest);
  const backlog = listItems(sections.get("Backlog") ?? []);
  if (checklist.includes(null) || backlog.rest.some((l) => l.trim() !== "")) {
    unread.push({ path, reason: "a next-group item has no `[ ]` box, or the backlog is not a list" });
    return null;
  }
  const group = { ...lead, items: checklist, ...(after === "" ? {} : { after }) };
  const said = state.join("\n").trim().replace(/^\*\*Goal \d+ —/, `**Goal \`${slug}\` —`);
  return { state: said, next: group, backlog: backlog.items };
}
