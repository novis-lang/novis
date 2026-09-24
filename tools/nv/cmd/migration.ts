// `bun nv migration`: docs/spec/02-php-migration.md audited against the PHP inventory and the Core
// member list.
//
//     bun nv migration             audit; exit 1 on a structural error
//     bun nv migration --report    audit, then list every unclassified name
//     bun nv migration --min 85    also exit 1 below 85% classified
//     bun nv migration --seed      emit rows derived from 01-core-library.md's Replaces column, for
//                                  pasting into the table
//
// Every PHP name owes a recorded outcome, and this is what records that it has one. Three inputs, each
// with one job:
//
// - `tools/data/php-builtins.txt`   the inventory: what PHP has. Generated against the oracle build and
//                                   never edited by hand, so a name PHP has announced and this build
//                                   does not carry is in `AHEAD_OF_THE_BUILD`.
// - `docs/spec/01-core-library.md`  what Novis has. Authoritative for every signature.
// - `docs/spec/02-php-migration.md` the mapping: one row per PHP name, one outcome each.
//
// A structural error fails. An `open` row does not until the migration file's header says `Complete:
// yes`, so the count of what is left stays visible on every run without the table having to be filled
// in one pass. `--min N` is the floor a goal names: it compares the rounded percentage the `open:` line
// prints, so a run that prints "85% covered" passes `--min 85`.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

export const summary = "02-php-migration.md against the PHP inventory: nv migration [--report | --min N | --seed]";

const INVENTORY = "tools/data/php-builtins.txt";
const CORE_LIB = "docs/spec/01-core-library.md";
const MIGRATION = "docs/spec/02-php-migration.md";

const OUTCOMES = new Set(["member", "language", "dropped", "open"]);
/** The outcomes that owe a replacement cell. */
const DECIDED = new Set(["member", "language", "dropped"]);

// Extensions the oracle build does not load, so their functions are absent from the inventory and
// cannot be audited. Each is a Tier 1 extension or a subsystem whose replacement is `Core`'s own, and
// one comes off this list when the inventory is regenerated against a build that carries it.
const UNAUDITED = [
  "mbstring", "curl", "openssl", "sodium", "sockets", "intl", "gd", "exif",
  "posix", "pcntl", "gettext", "ftp", "ldap", "soap",
  "bz2", "xsl", "tidy", "shmop", "sysvsem", "imap", "snmp", "dba", "enchant",
];

// Names a future PHP announces that the oracle build does not have yet, each with the release it waits
// for. A row for one is answered ahead of the build rather than a typo. These names are not in the
// denominator, so a row here only stops being an error.
const AHEAD_OF_THE_BUILD: Record<string, string> = {
  // `rule:php-migration/a-deprecation-is-a-refusal`: already `Core\Math::clamp`, and generic over any
  // naturally ordered type rather than over int|float.
  clamp: "PHP 8.6",
};

const CLASS_REF = /Core\\[A-Za-z\\]+/g;
const BACKTICKED = /`([^`]+)`/g;
const PHP_NAME = /[a-z_][a-z0-9_]*/g;
const MEMBER_REF = /(Core\\[A-Za-z\\]+)::([A-Za-z][A-Za-z0-9]*)/g;
const ROW = /^\|\s*`([^`]+)`\s*\|\s*([a-z]+)\s*\|(.*)\|\s*$/;

/** Python's `str.splitlines`: every line boundary it knows, and no empty line after the last one. */
function splitlines(text: string): string[] {
  const lines = text.split(/\r\n|[\n\r\v\f\x1c-\x1e\x85\p{Zl}\p{Zp}]/u);
  if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}

/** `repr` of a string the way Python writes it, since a message quotes a cell that way. */
function pyRepr(s: string): string {
  const q = s.includes("'") && !s.includes('"') ? '"' : "'";
  const body = s.replace(/\\/g, "\\\\").replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t");
  return q + (q === "'" ? body.replace(/'/g, "\\'") : body) + q;
}

/** Python's `round` to a whole number: a tie goes to the even neighbour. */
function roundHalfEven(x: number): number {
  const floor = Math.floor(x);
  const diff = x - floor;
  if (diff > 0.5) return floor + 1;
  if (diff < 0.5) return floor;
  return floor % 2 === 0 ? floor : floor + 1;
}

/** Python's `int` over a command-line word, or null where it would raise. */
function pyInt(word: string): number | null {
  const m = /^\s*([+-]?)(\d+(?:_\d+)*)\s*$/.exec(word);
  return m ? Number(m[1]! + m[2]!.replaceAll("_", "")) : null;
}

function read(path: string): string {
  return readFileSync(join(ROOT, path), "utf8").replace(/^﻿/, "");
}

/**
 * The file with fenced code blocks removed. A fence is three backticks, so leaving one in flips the
 * pairing of every inline code span after it and mis-parses the rest of the file.
 */
function prose(path: string): string {
  const kept: string[] = [];
  let fenced = false;
  for (const line of splitlines(read(path))) {
    if (line.trimStart().startsWith("```")) {
      fenced = !fenced;
      continue;
    }
    if (!fenced) kept.push(line);
  }
  return kept.join("\n");
}

/** The oracle build's internal functions and types, in file order. */
function phpInventory(): { functions: string[]; types: string[] } {
  const functions: string[] = [];
  const types: string[] = [];
  let bucket: string[] | null = null;
  for (const raw of splitlines(read(INVENTORY))) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    if (line === "[functions]") bucket = functions;
    else if (line === "[types]") bucket = types;
    else if (bucket !== null) bucket.push(line);
  }
  return { functions, types };
}

/**
 * Every `Core\X` class the spec names, and every backticked bare word in it. The word set is loose on
 * purpose: the spec states some members in prose, so demanding a table row per member would fail on
 * the file's own valid shapes. What this catches is a member renamed in the spec while the migration
 * table still names the old one.
 */
function coreSurface(): { classes: Set<string>; words: Set<string> } {
  const classes = new Set<string>();
  const words = new Set<string>();
  for (const [, span = ""] of prose(CORE_LIB).matchAll(BACKTICKED)) {
    for (const [cls = ""] of span.matchAll(CLASS_REF)) classes.add(cls);
    for (const [, cls = "", member = ""] of span.matchAll(MEMBER_REF)) {
      classes.add(cls);
      words.add(member);
    }
    const bare = span.trim();
    if (/^[A-Za-z][A-Za-z0-9]*$/.test(bare)) words.add(bare);
    const head = /^([A-Za-z][A-Za-z0-9]*)\s*\(/.exec(bare);
    if (head) words.add(head[1]!);
    const arrow = /^\$[\p{L}\p{N}_]+->([A-Za-z][A-Za-z0-9]*)/u.exec(bare);
    if (arrow) words.add(arrow[1]!);
  }
  return { classes, words };
}

/** Rows keyed by PHP name, the names that have a second row, and whether the file claims completeness. */
function migrationRows(): { rows: Map<string, [string, string]>; duplicates: string[]; complete: boolean } {
  const rows = new Map<string, [string, string]>();
  const duplicates: string[] = [];
  let complete = false;
  for (const line of splitlines(read(MIGRATION))) {
    if (line.startsWith("**Complete:**")) complete = line.toLowerCase().includes("yes");
    const m = ROW.exec(line.trimEnd());
    if (!m) continue;
    const [, name = "", outcome = "", note = ""] = m;
    if (rows.has(name)) duplicates.push(name);
    rows.set(name, [outcome, note.trim()]);
  }
  return { rows, duplicates, complete };
}

/** Rows derived from the Replaces column, for a person to check and paste. */
function seed(): void {
  const { functions } = phpInventory();
  const known = new Set(functions);
  let section = "";
  const derived = new Map<string, string>();
  for (const line of splitlines(prose(CORE_LIB))) {
    if (line.startsWith("#")) {
      const found = /`(Core\\[A-Za-z\\]+)`/.exec(line);
      if (found) section = found[1]!;
      continue;
    }
    const spans = [...line.matchAll(BACKTICKED)].map((m) => m[1]!);
    if (spans.length === 0) continue;
    // The target is whatever the line is about: a table row's first cell, a bullet's leading class, or
    // failing both the section this line sits in.
    let target = section;
    const lead = spans[0]!.trim();
    if (line.startsWith("|")) {
      const first = line.split("|")[1]!.trim();
      const head = /`([^`]+)`/.exec(first);
      if (head) {
        const name = head[1]!.trim();
        if (/^(Core\\[A-Za-z\\]+)(?:::|$)/.test(name)) {
          target = name.split("(")[0]!.replace(/^`+|`+$/g, "");
        } else if (name.includes("::")) {
          target = "Core\\" + name;
        } else if (/^[A-Za-z][A-Za-z0-9]*(\s*\/\s*[A-Za-z][A-Za-z0-9]*)?$/.test(name)) {
          target = section ? `${section}::${name.split("/")[0]!.trim()}` : name;
        }
      }
    } else if (line.startsWith("- ") && lead.startsWith("Core\\")) {
      target = lead;
    }
    for (const span of spans) {
      for (const [candidate = ""] of span.matchAll(PHP_NAME)) {
        if (known.has(candidate) && !derived.has(candidate)) derived.set(candidate, target);
      }
    }
  }
  for (const name of [...derived.keys()].sort()) {
    const target = derived.get(name)!;
    console.log(`| \`${name}\` | member | ${target ? `\`${target}\`` : ""} |`);
  }
  console.error(`\n# ${derived.size} of ${functions.length} inventory functions derived`);
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  if (args.includes("--seed")) {
    seed();
    return 0;
  }

  const { functions, types } = phpInventory();
  const { classes, words } = coreSurface();
  const { rows, duplicates, complete } = migrationRows();

  const errors: string[] = [];
  for (const name of duplicates) errors.push(`duplicate row: ${name}`);
  const expected = `[${[...OUTCOMES].sort().map(pyRepr).join(", ")}]`;
  for (const [name, [outcome, note]] of rows) {
    if (!OUTCOMES.has(outcome)) errors.push(`${name}: unknown outcome ${pyRepr(outcome)}, expected one of ${expected}`);
    if (DECIDED.has(outcome) && !note) {
      errors.push(`${name}: outcome ${outcome} with an empty replacement cell`);
    }
    for (const [, cls = "", member = ""] of note.matchAll(MEMBER_REF)) {
      if (!classes.has(cls)) errors.push(`${name}: names ${cls}, which 01-core-library.md does not`);
      else if (!words.has(member)) errors.push(`${name}: names ${cls}::${member}, which 01-core-library.md does not`);
    }
  }

  const inventory = new Set(functions);
  const typeSet = new Set(types);
  const unknown = [...rows.keys()]
    .filter((n) => !inventory.has(n) && !typeSet.has(n) && !Object.hasOwn(AHEAD_OF_THE_BUILD, n))
    .sort();
  for (const name of unknown) {
    errors.push(`row for ${pyRepr(name)}, which the inventory does not list -- typo, or a stale row`);
  }

  // A name with no row and a name whose row says `open` are the same thing: undecided.
  const unclassified = functions.filter((f) => (rows.get(f)?.[0] ?? "open") === "open");
  const openRows = [...rows.values()].filter((r) => r[0] === "open").length;

  const covered = roundHalfEven(100 * (1 - unclassified.length / Math.max(functions.length, 1)));
  console.log(`inventory: ${functions.length} functions, ${types.length} types (${INVENTORY})`);
  console.log(`rows:      ${rows.size}  classified: ${rows.size - openRows}`);
  console.log(`open:      ${unclassified.length}  (${covered}% covered)`);
  console.log(`unaudited extensions (absent from this build): ${UNAUDITED.join(", ")}`);
  const ahead = [...rows.keys()].filter((n) => Object.hasOwn(AHEAD_OF_THE_BUILD, n)).sort();
  if (ahead.length > 0) {
    console.log(
      "ahead of the build (answered, not yet in the inventory): " +
        ahead.map((n) => `${n} (${AHEAD_OF_THE_BUILD[n]})`).join(", "),
    );
  }

  if (args.includes("--report") && unclassified.length > 0) {
    console.log("\nunclassified:");
    for (const name of unclassified) console.log(`  ${name}`);
  }

  if (errors.length > 0) {
    console.log(`\n${errors.length} structural error(s):`);
    for (const error of errors.slice(0, 80)) console.log(`  ${error}`);
    if (errors.length > 80) console.log(`  ... and ${errors.length - 80} more`);
    return 1;
  }

  if (complete && unclassified.length > 0) {
    console.log(`\nheader says Complete: yes, but ${unclassified.length} names are unclassified`);
    return 1;
  }

  const at = args.indexOf("--min");
  if (at >= 0) {
    if (at + 1 >= args.length) {
      console.log("\n--min needs a percentage, e.g. --min 85");
      return 2;
    }
    const floor = pyInt(args[at + 1]!);
    if (floor === null) {
      console.log(`\n--min takes a whole percentage, not ${pyRepr(args[at + 1]!)}`);
      return 2;
    }
    if (covered < floor) {
      console.log(
        `\n${covered}% covered, below the ${floor}% this run asserts ` +
          `-- ${unclassified.length} name(s) still unclassified, \`--report\` lists them`,
      );
      return 1;
    }
  }

  return 0;
}
