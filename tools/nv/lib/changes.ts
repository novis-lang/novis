// What changed since the last release, as a user reads it: `website/src/data/changes.json`, the snapshot it
// is judged against, and what a release does with both (`rule:testing/every-change-reaches-the-website`).
//
// **The entries.** Each is one change a user can see: its `kind`, the roster ids it names, one to three
// sentences of `text` in the voice of AGENTS.md § *Text an end user reads*, and `instead`, what to write
// now, on a `removed` entry and on one with `breaking: true`. `highlight` leads its section, `security`
// puts it under *Security fixes*.
//
// **The snapshot**, `data/proofs/released.json`, is each feature's behaviour hash (`Hashes.behaviour`,
// its tests alone) at the last release. `changesProblems` compares the roster now with it: a feature not
// in it owes a `new` entry, one no longer on the roster a `removed` entry, and one whose hash moved a
// `changed` or `fixed` entry, or an `unseen` record at the hash it has now. `unseen` is for a move a user
// cannot see, such as a renamed test, and never for one they can.
//
// **A release** (`releaseChanges`) adds each entry to the changelog of every feature it names, empties
// the entries and `unseen`, and writes a new snapshot. `bun nv release --notes` renders the entries first
// (`notesSections`), so the release body and `CHANGELOG.md` say the same.
//
// `cmd/site.ts` and `cmd/release.ts` both read this, and `site.ts` holds the prose check, so it is passed
// in. What it costs: nothing at run time. The check hashes every feature's tests once per run.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { Entry } from "../proofs/roster.ts";

export const CHANGES = "website/src/data/changes.json";
export const RELEASED = "data/proofs/released.json";
export const CORE_CHANGELOG = "website/src/data/core-changelog.json";

export const KINDS = ["new", "changed", "fixed", "faster", "removed"] as const;
export type Kind = (typeof KINDS)[number];

export interface ChangeEntry {
  kind: Kind;
  features: string[];
  text: string;
  breaking?: boolean;
  instead?: string;
  highlight?: boolean;
  security?: boolean;
}

export interface Unseen {
  hash: string;
  why: string;
}

export interface Changes {
  entries: ChangeEntry[];
  unseen: Record<string, Unseen>;
}

export interface Released {
  since: string;
  features: Record<string, string>;
}

const ENTRY_KEYS = new Set(["kind", "features", "text", "breaking", "instead", "highlight", "security"]);

const readJson = (root: string, path: string): unknown => {
  const at = join(root, path);
  return existsSync(at) ? JSON.parse(readFileSync(at, "utf8")) : null;
};

const writeJson = (root: string, path: string, value: unknown) => {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), `${JSON.stringify(value, null, 2)}\n`);
};

/** `changes.json`, or an empty one when the file is not there. */
export function readChanges(root: string): Changes {
  const raw = readJson(root, CHANGES) as Partial<Changes> | null;
  return { entries: Array.isArray(raw?.entries) ? raw.entries : [], unseen: raw?.unseen && typeof raw.unseen === "object" ? raw.unseen : {} };
}

export function writeChanges(root: string, changes: Changes): void {
  writeJson(root, CHANGES, changes);
}

/** The snapshot, or null when none was taken. */
export function readReleased(root: string): Released | null {
  return readJson(root, RELEASED) as Released | null;
}

/** The snapshot of `entries` now, named `since`. */
export function snapshot(entries: Entry[], behaviour: (e: Entry) => string, since: string): Released {
  const features: Record<string, string> = {};
  for (const e of [...entries].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))) features[e.id] = behaviour(e);
  return { since, features };
}

export function writeReleased(root: string, released: Released): void {
  writeJson(root, RELEASED, released);
}

/** The sentences of a text, counted with its code spans taken out, so `Core\Str::length.` is one. */
export function sentences(text: string): number {
  const plain = text.replace(/`[^`]*`/g, "x").trim();
  return (plain.match(/[.!?](?=\s|$)/g) ?? []).length;
}

/** Whether an entry may name a feature whose behaviour did not move. */
const mayNameUnmoved = (e: ChangeEntry) => e.kind === "faster" || e.highlight === true || e.security === true;

/** The kinds that clear a feature of each move. */
const CLEARS: Record<"new" | "removed" | "moved", Kind[]> = { new: ["new"], removed: ["removed"], moved: ["changed", "fixed"] };

/**
 * Every problem with `changes` against the roster now and the snapshot: a moved feature with no entry,
 * an entry that breaks its shape, and an `unseen` record that does not hold. `prose` is the countable
 * bounds of `--check prose`, which gives `line: problem` per problem.
 */
export function changesProblems(
  entries: Entry[],
  behaviour: (e: Entry) => string,
  released: Released | null,
  changes: Changes,
  prose: (text: string) => string[],
): string[] {
  if (released === null) return [`${RELEASED} is missing: take the snapshot with \`bun nv site --changes --snapshot <since>\``];
  const problems: string[] = [];
  const now = new Map(entries.map((e) => [e.id, behaviour(e)]));
  const then = new Map(Object.entries(released.features));

  // How each feature moved since the snapshot.
  const moves = new Map<string, "new" | "removed" | "moved">();
  for (const [id, hash] of now) {
    const was = then.get(id);
    if (was === undefined) moves.set(id, "new");
    else if (was !== hash) moves.set(id, "moved");
  }
  for (const id of then.keys()) if (!now.has(id)) moves.set(id, "removed");

  changes.entries.forEach((entry, i) => {
    const at = `${CHANGES} entry ${i + 1}`;
    for (const key of Object.keys(entry)) if (!ENTRY_KEYS.has(key)) problems.push(`${at}: has \`${key}\`, which an entry does not have`);
    if (!(KINDS as readonly string[]).includes(entry.kind)) problems.push(`${at}: kind \`${entry.kind}\` is not one of ${KINDS.join(", ")}`);
    if (!Array.isArray(entry.features) || entry.features.length === 0) problems.push(`${at}: names no feature`);
    if (typeof entry.text !== "string" || entry.text.trim() === "") problems.push(`${at}: has no text`);
    else {
      const n = sentences(entry.text);
      if (n < 1 || n > 3) problems.push(`${at}: text is ${n} sentence(s), and an entry is one to three`);
      for (const p of prose(entry.text)) problems.push(`${at}: text: ${p.slice(p.indexOf(":") + 1).trim()}`);
    }
    const owesInstead = entry.kind === "removed" || entry.breaking === true;
    if (owesInstead && (typeof entry.instead !== "string" || entry.instead.trim() === "")) {
      problems.push(`${at}: a ${entry.kind === "removed" ? "removed" : "breaking"} entry says what to write instead in \`instead\``);
    }
    if (typeof entry.instead === "string") for (const p of prose(entry.instead)) problems.push(`${at}: instead: ${p.slice(p.indexOf(":") + 1).trim()}`);
    for (const id of Array.isArray(entry.features) ? entry.features : []) {
      if (!now.has(id) && !then.has(id)) {
        problems.push(`${at}: names ${id}, which neither the roster nor the snapshot has`);
        continue;
      }
      if (!moves.has(id) && !mayNameUnmoved(entry)) problems.push(`${at}: names ${id}, whose behaviour has not moved since ${released.since}`);
    }
  });

  for (const [id, record] of Object.entries(changes.unseen)) {
    const hash = now.get(id);
    if (hash === undefined) problems.push(`${CHANGES} unseen: ${id} is not on the roster`);
    else if (record.hash !== hash) problems.push(`${CHANGES} unseen: ${id} names hash ${record.hash}, and its behaviour hash is now ${hash}`);
    if (typeof record.why !== "string" || record.why.trim() === "") problems.push(`${CHANGES} unseen: ${id} says no \`why\``);
  }

  const named = (id: string, kinds: Kind[]) => changes.entries.some((e) => kinds.includes(e.kind) && Array.isArray(e.features) && e.features.includes(id));
  for (const [id, move] of [...moves].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
    if (named(id, CLEARS[move])) continue;
    if (move === "moved" && changes.unseen[id]?.hash === now.get(id)) continue;
    if (move === "new") problems.push(`new: ${id} is not in the snapshot of ${released.since}, and no \`new\` entry names it`);
    else if (move === "removed") problems.push(`removed: ${id} is no longer on the roster, and no \`removed\` entry names it`);
    else problems.push(`changed: ${id}'s tests moved since ${released.since}, and no \`changed\` or \`fixed\` entry or \`unseen\` record names it`);
  }
  return problems;
}

/** The release notes' sections the entries go to, in their order. *Internal changes* is the commit count. */
export const NOTE_SECTIONS = ["Security fixes", "Breaking changes", "New", "Faster", "Fixes"] as const;

/** The section an entry goes to: its flags first, then its kind. */
export function sectionOf(e: ChangeEntry): (typeof NOTE_SECTIONS)[number] {
  if (e.security) return "Security fixes";
  if (e.kind === "removed" || e.breaking) return "Breaking changes";
  if (e.kind === "new" || e.kind === "changed") return "New";
  if (e.kind === "faster") return "Faster";
  return "Fixes";
}

/** The entries as the notes' first sections: one list item each, its text and then its `instead`, a
 * highlight first in its section, and an empty section left out. */
export function notesSections(entries: ChangeEntry[]): string[] {
  const lines: string[] = [];
  for (const title of NOTE_SECTIONS) {
    const rows = entries.filter((e) => sectionOf(e) === title);
    if (rows.length === 0) continue;
    const ordered = [...rows.filter((e) => e.highlight), ...rows.filter((e) => !e.highlight)];
    lines.push(`### ${title}`, "", ...ordered.map((e) => `- ${[e.text, e.instead].filter(Boolean).join(" ")}`), "");
  }
  return lines;
}

/** The key a feature's changelog has in `core-changelog.json`, the one its reference page reads: a
 * `Core` member's page id, `Str.length`, and every other feature's roster id. */
export function changelogKey(id: string, entry: Entry | undefined): string {
  if (entry?.kind !== "member") return id;
  const member = id.slice(id.lastIndexOf("::") + 2);
  return `${entry.group.replace(/^Core\\/, "").replace(/\\/g, ".")}.${member}`;
}

/**
 * What a release does with the entries: adds `{ version, change }` to the changelog of every feature
 * each entry names, empties the entries and `unseen`, and writes the snapshot of the roster now, named
 * `v<version>`. Returns how many entries it moved.
 */
export function releaseChanges(root: string, version: string, entries: Entry[], behaviour: (e: Entry) => string): number {
  const bare = version.replace(/^v/, "");
  const changes = readChanges(root);
  const byId = new Map(entries.map((e) => [e.id, e]));
  const changelog = (readJson(root, CORE_CHANGELOG) ?? {}) as Record<string, unknown>;
  for (const entry of changes.entries) {
    for (const id of entry.features) {
      const key = changelogKey(id, byId.get(id));
      const list = Array.isArray(changelog[key]) ? (changelog[key] as unknown[]) : [];
      list.push({ version: bare, change: entry.text });
      changelog[key] = list;
    }
  }
  writeJson(root, CORE_CHANGELOG, changelog);
  writeChanges(root, { entries: [], unseen: {} });
  writeReleased(root, snapshot(entries, behaviour, `v${bare}`));
  return changes.entries.length;
}
