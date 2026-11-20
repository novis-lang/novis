// What a feature's texts are, and the hashes that say when one of them is stale.
//
// `cmd/site.ts` and `proofs/collect.ts` both need this, and `site.ts` already imports `collect.ts`, so it
// lives here rather than in either. Every function takes `root`, so a test runs it on a scratch tree.
//
// **A feature's tests** are the tests whose `covers:` marker names it, as `markersAt` finds them under
// `MARKER_ROOTS`. A `.nvst` case is its whole file. A Rust `#[test]` and a package's `#[Test]` method
// run from the marker's line to the closing brace at the indent of their `fn` or `function` line, so an
// edit to another test in the same file moves nothing. A case credited only because it calls
// `Class::member(` is not here: it usually tests something else.
//
// **The hashes** (`rule:testing/every-change-reaches-the-website`), each over what decides one verdict:
//
// - `page`: the card, `about.md`, every file of the example directory and the tests. A handwritten
//   page's stamp.
// - `text`: the same without the tests. A page's search questions, and the baseline's restamp.
// - `about`: the page hash without `about.md`, so rewriting it never makes it stale itself.
// - `help`: the page hash without the card, for the same reason.
// - `behaviour`: the tests alone. `--check changes`.
//
// What it costs: one walk of the marker trees per `Hashes`, which `bun nv site --check stale` makes
// once per run, as `bun nv proofs` does. Nothing at run time.

import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { comparePaths } from "./py.ts";
import { ROOT } from "./paths.ts";
import { MARKER_ROOTS, markersAt } from "../proofs/markers.ts";
import { ABOUT, aboutFile, examplesDir, type Entry, type Meta } from "../proofs/roster.ts";

/** A file's text with every line ending read as `\n`, or `""` when it cannot be read. */
export const readAt = (root: string, path: string) => {
  try {
    return readFileSync(join(root, path), "utf8").replace(/\r\n?/g, "\n");
  } catch {
    return "";
  }
};

/** Every file under a repo-relative directory, repo-relative and sorted. */
export function filesUnder(root: string, dir: string): string[] {
  const out: string[] = [];
  const walk = (d: string) => {
    if (!existsSync(join(root, d))) return;
    for (const name of readdirSync(join(root, d))) {
      const p = `${d}/${name}`;
      if (statSync(join(root, p)).isDirectory()) walk(p);
      else out.push(p);
    }
  };
  walk(dir);
  return out.sort(comparePaths);
}

/** The section of a reference chapter a language or tool feature is, from its `# ` heading to the next. */
export function section(root: string, anchor: string): string {
  const at = anchor.lastIndexOf(":");
  const lines = readAt(root, anchor.slice(0, at)).split("\n");
  const out: string[] = [];
  let fence = "";
  for (const line of lines.slice(Number(anchor.slice(at + 1)) - 1)) {
    const marker = line.trim().slice(0, 3);
    if (fence) {
      if (marker === fence) fence = "";
    } else if (marker === "```" || marker === "~~~") {
      fence = marker;
    } else if (out.length > 0 && line.startsWith("# ")) {
      break;
    }
    out.push(line);
  }
  return out.join("\n");
}

const named = (items: unknown[] | undefined, key: string, name: string) =>
  (items ?? []).find((x) => (typeof x === "object" && x !== null ? String((x as Record<string, unknown>)[key]) : String(x)) === name);

/** What the binary's help says of a feature: its card in `nvs meta --json`, or its chapter section.
 * A language feature's is empty, because `bun nv render` writes its section from its own files. */
export function card(root: string, entry: Entry, meta: Meta): string {
  let found: unknown;
  switch (entry.kind) {
    case "member": {
      const member = entry.id.slice(entry.id.lastIndexOf("::") + 2);
      found = meta.classes?.find((c) => c.name === entry.group)?.members?.find((m) => m.name === member);
      break;
    }
    case "exception":
      found = named(meta.exceptions, "name", entry.id);
      break;
    case "enum":
      found = named(meta.enums, "name", entry.id);
      break;
    case "interface":
      found = named(meta.interfaces, "name", entry.id);
      break;
    case "directive":
      found = named(meta.directives, "key", entry.id.slice("directive:".length));
      break;
    case "lang":
      // Its chapter section is generated from the files the hashes already read.
      return "";
    default:
      return entry.anchor ? section(root, entry.anchor) : "";
  }
  return JSON.stringify(found ?? null);
}

/** One test of a feature: the label it is credited under, and its text. */
export interface Test {
  label: string;
  text: string;
}

/** Every feature id a `covers:` marker names, with its tests in label order. */
export type TestIndex = Map<string, Test[]>;

/** Every file under `base` ending in `ext`, repo-relative, outside build output. */
function walk(root: string, base: string, ext: string): string[] {
  const out: string[] = [];
  const visit = (d: string) => {
    let names: string[];
    try {
      names = readdirSync(join(root, d));
    } catch {
      return;
    }
    for (const name of names) {
      if (name === "target" || name === "node_modules" || name.startsWith(".")) continue;
      const p = `${d}/${name}`;
      if (statSync(join(root, p)).isDirectory()) visit(p);
      else if (name.endsWith(ext)) out.push(p);
    }
  };
  visit(base);
  return out;
}

/** The text of the test under a marker: the whole file for a case, else from the marker's line to the
 * closing brace at the indent of its `fn` or `function` line, or to the end of the file. */
function testText(text: string, at: number, fnAt: number): string {
  if (fnAt < 0) return text;
  const start = text.lastIndexOf("\n", at) + 1;
  const fnLine = text.slice(text.lastIndexOf("\n", fnAt) + 1);
  const indent = /^[ \t]*/.exec(fnLine)![0];
  const close = new RegExp(`\\n${indent}\\}[ \\t]*(?:\\n|$)`, "g");
  close.lastIndex = fnAt;
  const m = close.exec(text);
  return m ? text.slice(start, m.index + m[0].trimEnd().length) : text.slice(start);
}

/** Every feature's tests under `MARKER_ROOTS`, read once. */
export function testIndex(root: string = ROOT): TestIndex {
  const out: TestIndex = new Map();
  for (const [base, ext] of MARKER_ROOTS) {
    for (const path of walk(root, base, ext)) {
      const text = readAt(root, path);
      for (const m of markersAt(path, text)) {
        const body = testText(text, m.at, m.fnAt);
        for (const name of m.names) {
          if (!out.has(name)) out.set(name, []);
          out.get(name)!.push({ label: m.label, text: body });
        }
      }
    }
  }
  for (const tests of out.values()) tests.sort((a, b) => comparePaths(a.label, b.label) || (a.text < b.text ? -1 : a.text > b.text ? 1 : 0));
  return out;
}

/** The key a feature's help card is stamped under in the lock. */
export const helpKey = (id: string) => `help:${id}`;

/** Whether a lock key names an `about.md` or a help card rather than a page. */
export const isTextKey = (key: string) => key.startsWith("help:") || (key.startsWith("docs/examples/") && key.endsWith(`/${ABOUT}`));

/** What one feature's hashes are made of. */
interface Parts {
  card: string;
  about: string;
  files: string[];
  tests: string[];
}

/** The hashes of a feature, each read once per feature. One per run: the files it reads are not
 * expected to change while it lives. */
export class Hashes {
  private parts = new Map<string, Parts>();

  constructor(
    readonly root: string,
    readonly meta: Meta,
    readonly tests: TestIndex = testIndex(root),
  ) {}

  private of(entry: Entry): Parts {
    let p = this.parts.get(entry.id);
    if (p) return p;
    const dir = examplesDir(entry);
    p = {
      card: card(this.root, entry, this.meta),
      about: readAt(this.root, `${dir}/${ABOUT}`),
      files: filesUnder(this.root, dir).filter((f) => !f.endsWith(`/${ABOUT}`)).map((f) => `\0${f}\0${readAt(this.root, f)}`),
      tests: (this.tests.get(entry.id) ?? []).map((t) => `\0test\0${t.label}\0${t.text}`),
    };
    this.parts.set(entry.id, p);
    return p;
  }

  private hash(entry: Entry, use: { card: boolean; about: boolean; tests: boolean }): string {
    const p = this.of(entry);
    const h = createHash("sha256");
    h.update(use.card ? p.card : "");
    h.update(use.about ? `\0${p.about}` : "\0");
    for (const f of p.files) h.update(f);
    if (use.tests) for (const t of p.tests) h.update(t);
    return h.digest("hex").slice(0, 16);
  }

  /** A handwritten page's stamp: the card, `about.md`, the example directory and the tests. */
  page(entry: Entry): string {
    return this.hash(entry, { card: true, about: true, tests: true });
  }

  /** The same without the tests: a page's search questions. */
  text(entry: Entry): string {
    return this.hash(entry, { card: true, about: true, tests: false });
  }

  /** An `about.md`'s stamp: the page hash without the `about.md` itself. */
  about(entry: Entry): string {
    return this.hash(entry, { card: true, about: false, tests: true });
  }

  /** A help card's stamp: the page hash without the card itself. */
  help(entry: Entry): string {
    return this.hash(entry, { card: false, about: true, tests: true });
  }

  /** The tests alone: what says a feature's behaviour moved. */
  behaviour(entry: Entry): string {
    const h = createHash("sha256");
    for (const t of this.of(entry).tests) h.update(t);
    return h.digest("hex").slice(0, 16);
  }

  /** The stamped texts a feature has, each with its lock key and its hash now: its `about.md` when
   * the file exists, and its help card unless it is a language feature, whose reference section is
   * written from the files the hashes already read. */
  textsOf(entry: Entry): { key: string; hash: string }[] {
    const out: { key: string; hash: string }[] = [];
    if (this.of(entry).about !== "") out.push({ key: aboutFile(entry), hash: this.about(entry) });
    if (entry.kind !== "lang") out.push({ key: helpKey(entry.id), hash: this.help(entry) });
    return out;
  }
}
