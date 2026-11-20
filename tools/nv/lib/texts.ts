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
//
// **The user texts** (`rule:testing/feature-proofs`, Reviewed) are every text an end user reads that a
// tool can key, each with its key and a hash of its text, which `userTexts` lists:
//
// - an `about.md` or a `details.md` under `docs/examples/`: its path, and the file;
// - a help card: `help:<feature id>`, and `card`; a language feature, a package type and a feature with
//   no card have none;
// - a handwritten page under `DOCS`: its path, and the file; the generated and the legal pages are out;
// - a `.nvs` under `docs/examples/`, `tests/hostile/`, `benches/members/` or `website/snippets/`: its
//   path, and its comment lines without directive lines and without a `// hostile: comment-payload`
//   block (`commentText`), so a code edit owes no review; a file with no comment is no text;
// - `website/src/data/changes.json`: its path, and every entry's `text` and `instead`.
//
// A text is reviewed when its hash is its stamp's in `data/proofs/reviewed.json`, or its baseline hash
// in `data/proofs/review-baseline.json`, which lists the texts that existed when the review began and
// only ever shrinks: a stamp deletes the text's key from it. What it costs: about 1 MB in the
// repository for the two files, and nothing at run time.

import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { comparePaths } from "./py.ts";
import { ROOT } from "./paths.ts";
import { write } from "./store.ts";
import { reviewBaseline, reviewed } from "../schema/proofs.ts";
import { MARKER_ROOTS, markersAt } from "../proofs/markers.ts";
import { ABOUT, aboutFile, benchFile, BENCHES, DETAILS, EXAMPLES, examplesDir, HOSTILE, hostileDir, type Entry, type Meta } from "../proofs/roster.ts";

/** The handwritten pages' directory. */
export const DOCS = "website/src/content/docs";
/** The page directories a renderer writes, under `DOCS`. */
export const GENERATED = ["reference/core/"];
/** Pages whose wording is fixed by law, which neither `prose` nor the review judges. */
export const LEGAL = ["impressum.mdx", "datenschutz.mdx"];
/** The directory of the snippets the pages show. */
export const SNIPPETS = "website/snippets";
/** The unreleased change entries, a user text of their own (`lib/changes.ts` reads them). */
const CHANGES_FILE = "website/src/data/changes.json";
export const REVIEWED = "data/proofs/reviewed.json";
export const REVIEW_BASELINE = "data/proofs/review-baseline.json";

/** A comment line a tool reads, not prose. */
export const COMMENT_DIRECTIVE_RE = /^\/\/\s*(?:bench|hostile|covers|proof|requires):/;
/** The line above an attack's payload, a comment block no reader reads as prose. */
export const COMMENT_PAYLOAD_RE = /^\/\/\s*hostile:\s*comment-payload\s*$/;

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
 * A language feature's is empty, because `bun nv render` writes its section from its own files. The
 * roster's `card`, when it read one, is the same text, so a caller without `meta` gets it too. */
export function card(root: string, entry: Entry, meta: Meta): string {
  if (entry.card !== undefined) return entry.card;
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

/** One text an end user reads: its key, the hash of its text, the feature it belongs to or "", and the
 * text a reader is shown. */
export interface UserText {
  key: string;
  hash: string;
  feature: string;
  text: string;
}

const textHash = (text: string) => createHash("sha256").update(text).digest("hex").slice(0, 16);

/** A program's prose: its `//` and `///` comment lines, without directive lines and without the comment
 * block under `// hostile: comment-payload`, one per line. `""` when it has none. */
export function commentText(source: string): string {
  const out: string[] = [];
  let payload = false;
  for (const line of source.replace(/\r\n?/g, "\n").split("\n")) {
    const text = line.trim();
    if (COMMENT_PAYLOAD_RE.test(text)) {
      payload = true;
      continue;
    }
    if (payload && text.startsWith("//")) continue;
    payload = false;
    if (text.startsWith("//") && !COMMENT_DIRECTIVE_RE.test(text)) out.push(text);
  }
  return out.join("\n");
}

const userText = (key: string, text: string, feature = ""): UserText | null => (text.trim() === "" ? null : { key, hash: textHash(text), feature, text });

/** A program's text, or null when it has no comment. */
const programText = (root: string, path: string, feature = "") => userText(path, commentText(readAt(root, path)), feature);

/** The help card's text of a feature, or null for a language feature and one with no card. A package
 * type's help is the comment above its declaration in a source file, which is not a published text. */
function helpText(root: string, entry: Entry, meta: Meta): UserText | null {
  if (entry.kind === "lang" || entry.kind === "package") return null;
  const text = card(root, entry, meta);
  return text === "null" ? null : userText(helpKey(entry.id), text, entry.id);
}

/** A program's feature: the feature whose example or attack directory holds it, or whose bench it is or
 * is a sibling of (`.scale.nvs`, `.twin.nvs`). */
function programOwners(entries: Entry[]): (path: string) => string {
  const dirs = new Map<string, string>();
  const benches = new Map<string, string>();
  for (const e of entries) {
    dirs.set(examplesDir(e), e.id);
    dirs.set(hostileDir(e), e.id);
    benches.set(benchFile(e).slice(0, -".nvs".length), e.id);
  }
  return (path) => {
    if (path.startsWith(`${BENCHES}/`)) return benches.get(path.replace(/(\.scale|\.twin)?\.nvs$/, "")) ?? "";
    for (let dir = dirname(path); dir.includes("/"); dir = dirname(dir)) {
      const id = dirs.get(dir);
      if (id !== undefined) return id;
    }
    return "";
  };
}

/** The texts of one feature that its Reviewed proof is owed for: its `about.md`, its `details.md`, its
 * help card, and the comments of its examples, its attacks and its bench. `meta` is `{}` where the
 * roster's `card` is all there is. */
export function featureTexts(root: string, entry: Entry, meta: Meta = {}): UserText[] {
  const dir = examplesDir(entry);
  const programs = [
    ...filesUnder(root, dir).filter((f) => f.endsWith(".nvs")),
    ...filesUnder(root, hostileDir(entry)).filter((f) => f.endsWith(".nvs")),
    ...[benchFile(entry), ...[".scale.nvs", ".twin.nvs"].map((s) => benchFile(entry).replace(/\.nvs$/, s))].filter((f) => existsSync(join(root, f))),
  ];
  return [
    userText(`${dir}/${ABOUT}`, readAt(root, `${dir}/${ABOUT}`), entry.id),
    userText(`${dir}/${DETAILS}`, readAt(root, `${dir}/${DETAILS}`), entry.id),
    helpText(root, entry, meta),
    ...programs.map((p) => programText(root, p, entry.id)),
  ].filter((t): t is UserText => t !== null);
}

/** What `userTexts` reads the roster from. */
export interface TextWorld {
  entries: Entry[];
  meta: Meta;
}

/** Every user text, in key order. */
export function userTexts(root: string, world: TextWorld): UserText[] {
  const owner = programOwners(world.entries);
  const out: UserText[] = [];
  for (const f of filesUnder(root, EXAMPLES)) {
    if (f.endsWith(`/${ABOUT}`) || f.endsWith(`/${DETAILS}`)) {
      const t = userText(f, readAt(root, f), owner(`${dirname(f)}/x.nvs`));
      if (t) out.push(t);
    }
  }
  for (const e of world.entries) {
    const t = helpText(root, e, world.meta);
    if (t) out.push(t);
  }
  for (const f of filesUnder(root, DOCS)) {
    const key = f.slice(DOCS.length + 1);
    if (!/\.mdx?$/.test(f) || GENERATED.some((g) => key.startsWith(g)) || LEGAL.includes(key)) continue;
    const t = userText(f, readAt(root, f));
    if (t) out.push(t);
  }
  for (const base of [EXAMPLES, HOSTILE, BENCHES, SNIPPETS]) {
    for (const f of filesUnder(root, base)) {
      if (!f.endsWith(".nvs")) continue;
      const t = programText(root, f, base === SNIPPETS ? "" : owner(f));
      if (t) out.push(t);
    }
  }
  const changes = changesText(root);
  if (changes) out.push(changes);
  return out.sort((a, b) => comparePaths(a.key, b.key));
}

/** `changes.json`'s text: each entry's `text` and `instead`, one per line. */
function changesText(root: string): UserText | null {
  const raw = readAt(root, CHANGES_FILE);
  if (!raw) return null;
  let entries: { text?: unknown; instead?: unknown }[] = [];
  try {
    const doc = JSON.parse(raw) as { entries?: unknown };
    if (Array.isArray(doc.entries)) entries = doc.entries;
  } catch {
    return userText(CHANGES_FILE, raw);
  }
  const lines = entries.flatMap((e) => [e.text, e.instead].filter((x): x is string => typeof x === "string"));
  return userText(CHANGES_FILE, lines.join("\n"));
}

/** The review stamps and the baseline: key -> the hash the text was reviewed at, or had at the start. */
export interface ReviewState {
  reviewed: Record<string, string>;
  baseline: Record<string, string>;
  /** Whether the baseline file exists, since `--review --baseline` writes it once. */
  hasBaseline: boolean;
}

const readMap = (root: string, path: string): Record<string, string> | null => {
  const raw = readAt(root, path);
  return raw ? (JSON.parse(raw) as Record<string, string>) : null;
};

/** Writes a stamp file through the record store, so its layout is the one `bun nv check` holds it to. */
const writeMap = (root: string, path: string, map: Record<string, string>) => {
  const type = path === REVIEWED ? reviewed : reviewBaseline;
  write(type, type.name, map, root);
};

export function readReview(root: string = ROOT): ReviewState {
  const baseline = readMap(root, REVIEW_BASELINE);
  return { reviewed: readMap(root, REVIEWED) ?? {}, baseline: baseline ?? {}, hasBaseline: baseline !== null };
}

/** Whether a text was reviewed at its hash now, or is on the baseline at that hash. */
export const isReviewed = (state: ReviewState, t: UserText) => state.reviewed[t.key] === t.hash || state.baseline[t.key] === t.hash;

/** Every text that is not reviewed, in key order. */
export function unreviewed(root: string, world: TextWorld, state: ReviewState = readReview(root)): UserText[] {
  return userTexts(root, world).filter((t) => !isReviewed(state, t));
}

/** Records each named text's hash now, and takes its key off the baseline. Returns what stopped it. */
export function stampReview(root: string, world: TextWorld, keys: string[]): string[] {
  const texts = new Map(userTexts(root, world).map((t) => [t.key, t]));
  const problems = keys.filter((k) => !texts.has(k)).map((k) => `${k}: no user text has this key`);
  if (problems.length) return problems;
  const state = readReview(root);
  for (const k of keys) {
    state.reviewed[k] = texts.get(k)!.hash;
    delete state.baseline[k];
  }
  writeMap(root, REVIEWED, state.reviewed);
  if (state.hasBaseline) writeMap(root, REVIEW_BASELINE, state.baseline);
  return [];
}

/** Writes the baseline once: every user text that has no stamp, at its hash now. Returns what stopped
 * it and how many texts it wrote. */
export function writeReviewBaseline(root: string, world: TextWorld): { problems: string[]; texts: number } {
  const state = readReview(root);
  if (state.hasBaseline) return { problems: [`${REVIEW_BASELINE} exists; the baseline is written once and only shrinks`], texts: 0 };
  const map = Object.fromEntries(userTexts(root, world).filter((t) => state.reviewed[t.key] !== t.hash).map((t) => [t.key, t.hash]));
  writeMap(root, REVIEW_BASELINE, map);
  return { problems: [], texts: Object.keys(map).length };
}

/** The three questions a reader answers for each text. */
export const REVIEW_QUESTIONS = [
  "Quote each sentence you had to read twice.",
  "Name each word or term you did not know.",
  "In your own words, in two or three sentences, say what the feature does, or what the page teaches.",
];

/** What a reader is given: the prose rules, the three questions, and each text with its key. `shown`
 * gives a help card's text as `nvs agent show` prints it. */
export function reviewBrief(texts: UserText[], rules: string, shown: (t: UserText) => string = (t) => t.text): string {
  const out = [
    "You read these texts as a programmer who has never seen this project. Read no file: answer from this brief alone.",
    "",
    "The rules the texts are written to:",
    "",
    rules.trim(),
    "",
    "For each text, under its key, answer:",
    "",
    ...REVIEW_QUESTIONS.map((q, i) => `${i + 1}. ${q}`),
    "",
  ];
  for (const t of texts) out.push(`=== ${t.key}${t.feature ? ` (${t.feature})` : ""}`, "", (t.key.startsWith("help:") ? shown(t) : t.text).trim(), "");
  return out.join("\n");
}
