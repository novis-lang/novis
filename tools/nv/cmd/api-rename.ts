// `bun nv api-rename --goal <slug> [--check] [--root <dir>]`: renames `Core` and package members across the
// tree from the `renames` block of `docs/goals/<slug>.md`, the map `rule:core-api/verb-lexicon`'s renames
// are written in. The map lives in the goal file, so it is deleted with the goal and nothing in the tree
// keeps a list of old names.
//
// The block is a fenced code block whose info string is `renames`, one pair per line:
// `Shop\Reply::json Shop\Reply::sendJson` for a static member, `Shop\Feed->retry Shop\Feed->setRetryDelay`
// for an instance member. `#` starts a comment line. (The examples here are made up, so a sweep never
// rewrites this file's own text.)
//
// Without `--check` it rewrites, in every text file of the tree (`TEXT_EXTENSIONS`):
//
//   * every name written in full, `Shop\Reply::json` or `Shop\Feed->retry`, with a leading `\` or not, and
//     with the separators doubled as a Rust or TypeScript string writes them (`Shop\\Reply::json`). An
//     instance member's feature id, `Shop\Feed::retry`, is a full name too;
//   * in a `.nvs`, `.nvst` or `.lspt` file, `Reply::json` where the file has `use Shop\Reply;`, and
//     `R::json` where it has `use Shop\Reply as R;`;
//   * in a prose file (`.md`, `.mdx`, `.astro`), the short form a page writes, `Reply::json` and
//     `Feed->retry`, since a page never declares a class of that name.
//
// It never rewrites `->retry(`: an instance call needs its receiver's type, so it lists each one as
// `file:line` for a session to fix by reading. In a project, `nvs check` names the ones left as unknown
// members.
//
// `--class <Old\Class>`, given once or more, keeps only the pairs whose old class is one of those, so one
// stage of a goal sweeps its own classes and leaves the rest of the block for a later stage.
//
// With `--check` it rewrites nothing, and exits 1 naming every old name still written in either form, outside
// the goal file and `website/src/data/changes.json`. A `.rs` or `.ts` file is held to the full form alone,
// since `Reply::json` there may be a Rust path. It exits 2 on a bad argument or a goal with no block.
//
// What it spends: one read of each text file, about a second over the tree.

import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { extname, join, relative } from "node:path";
import { NOT_INPUTS, ROOT } from "../lib/paths.ts";

export const summary = "rename Core and package members across the tree from a goal's renames block: nv api-rename --goal <slug> [--class <Old\\Class>]... [--check] [--root <dir>]";

/** One `Old New` pair of the block. `instance` is true for `->`. */
export interface Rename {
  oldClass: string;
  oldMember: string;
  newClass: string;
  newMember: string;
  instance: boolean;
}

/** The files the sweep reads. */
export const TEXT_EXTENSIONS = new Set([
  ".nvs", ".nvst", ".lspt", ".md", ".mdx", ".astro", ".rs", ".ts", ".json", ".toml", ".out",
]);
const NOVIS = new Set([".nvs", ".nvst", ".lspt"]);
const PROSE = new Set([".md", ".mdx", ".astro"]);

/** Directories never swept, beside `NOT_INPUTS`: build output, generated pages and the loop's own state. */
const SKIPPED = new Set(["dist", ".astro", "public", ".looper"]);

/**
 * The files `--check` ignores, repo-relative, beside the goal file itself: the changelog entries, the
 * reference data a build writes, and the record of what a release shipped, which is history.
 */
const ALLOWED = [
  "website/src/data/changes.json",
  "website/src/data/core.json",
  "website/src/data/core-changelog.json",
  "data/proofs/released.json",
];

const PAIR = /^(\\?[A-Z][\w\\]*)(::|->)(\w+)$/;

/** The pairs of the `renames` block of a goal file's text. Throws on a malformed line or no block. */
export function readRenames(text: string): Rename[] {
  const block = /^```renames[ \t]*\r?\n([\s\S]*?)^```/m.exec(text);
  if (!block) throw new Error("the goal file has no ```renames block");
  const out: Rename[] = [];
  for (const raw of block[1]!.split(/\r?\n/)) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) continue;
    const parts = line.split(/\s+/);
    const old = parts.length === 2 ? PAIR.exec(parts[0]!) : null;
    const neu = parts.length === 2 ? PAIR.exec(parts[1]!) : null;
    if (!old || !neu || old[2] !== neu[2]) throw new Error(`not an \`Old New\` pair of one kind: ${line}`);
    out.push({
      oldClass: old[1]!.replace(/^\\/, ""),
      oldMember: old[3]!,
      newClass: neu[1]!.replace(/^\\/, ""),
      newMember: neu[3]!,
      instance: old[2] === "->",
    });
  }
  return out;
}

/** The pairs of `renames` whose old class is one of `classes`. Throws naming a class no pair has. */
export function keepClasses(renames: Rename[], classes: string[]): Rename[] {
  const wanted = classes.map((c) => c.replace(/^\\/, ""));
  for (const c of wanted) {
    if (!renames.some((r) => r.oldClass === c)) throw new Error(`no pair of the block renames a member of ${c}`);
  }
  return renames.filter((r) => wanted.includes(r.oldClass));
}

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const sep = (r: Rename) => (r.instance ? "->" : "::");
const short = (cls: string) => cls.slice(cls.lastIndexOf("\\") + 1);

/** `Core\Response` matched with each `\` written once or twice, after no word character or `\`. */
function fullClass(cls: string): string {
  return `(?<![\\w\\\\])(\\\\{0,2})${cls.split("\\").map(escape).join("(\\\\{1,2})")}`;
}

/**
 * The full form of `r`'s old name, with a group for each separator so a rewrite keeps the spelling. `via`
 * is the separator matched: an instance member's feature id is written `Shop\Feed::retry`, so its rename
 * is also matched with `::`.
 */
function fullPattern(r: Rename, via: string = sep(r)): RegExp {
  return new RegExp(`${fullClass(r.oldClass)}${escape(via)}${escape(r.oldMember)}(?!\\w)`, "g");
}

/** The separators each full form of `r` is written with: `->` and `::` for an instance member. */
const fullSeps = (r: Rename) => (r.instance ? ["->", "::"] : ["::"]);

/** `r`'s new full name, written with the separators `groups` matched and with `via`. */
function fullReplacement(r: Rename, groups: string[], via: string = sep(r)): string {
  const lead = groups[0] ?? "";
  const middle = groups[1] ?? "\\";
  return `${lead}${r.newClass.split("\\").join(middle)}${via}${r.newMember}`;
}

/** `Short::old` or `Short->old`, the class written by the name `alias`. */
function shortPattern(alias: string, r: Rename): RegExp {
  return new RegExp(`(?<![\\w\\\\$>:])${escape(alias)}${escape(sep(r))}${escape(r.oldMember)}(?!\\w)`, "g");
}

/** The name each `use` line of a Novis file brings in for `cls`: its last segment, or its alias. */
export function importedNames(text: string, cls: string): string[] {
  const out: string[] = [];
  const re = new RegExp(`^\\s*use\\s+\\\\?${escape(cls)}(?:\\s+as\\s+(\\w+))?\\s*;`, "gim");
  for (const m of text.matchAll(re)) out.push(m[1] ?? short(cls));
  return out;
}

/** One place a name is written. */
export interface Hit {
  line: number;
  text: string;
  rename: Rename;
}

const lineOf = (text: string, index: number) => text.slice(0, index).split("\n").length;

/** `text` with every name of `renames` it may rewrite rewritten, and the count. `ext` is the file's. */
export function rewrite(text: string, ext: string, renames: Rename[]): { text: string; count: number } {
  let count = 0;
  let out = text;
  for (const r of renames) {
    for (const via of fullSeps(r)) {
      out = out.replace(fullPattern(r, via), (...m) => {
        count++;
        return fullReplacement(r, m.slice(1, 1 + r.oldClass.split("\\").length) as string[], via);
      });
    }
    const aliases = NOVIS.has(ext) && !r.instance ? importedNames(out, r.oldClass) : PROSE.has(ext) ? [short(r.oldClass)] : [];
    for (const alias of new Set(aliases)) {
      out = out.replace(shortPattern(alias, r), () => {
        count++;
        return `${alias}${sep(r)}${r.newMember}`;
      });
    }
  }
  return { text: out, count };
}

/**
 * Each `->old(` or `->old<T>(` call of an instance rename in `text`, which only a reader of the receiver
 * can rename.
 */
export function instanceCalls(text: string, renames: Rename[]): Hit[] {
  const out: Hit[] = [];
  for (const r of renames) {
    if (!r.instance) continue;
    const re = new RegExp(`(?<!(?<![\\w$\\\\])[A-Z]\\w*)\\??->\\s*${escape(r.oldMember)}\\s*(?:<[^<>()]*>\\s*)?\\(`, "g");
    for (const m of text.matchAll(re)) out.push({ line: lineOf(text, m.index!), text: m[0], rename: r });
  }
  return out.sort((a, b) => a.line - b.line);
}

/** Every old name `text` still writes, in the forms `--check` holds a file of extension `ext` to. */
export function oldNames(text: string, ext: string, renames: Rename[]): Hit[] {
  const out: Hit[] = [];
  const code = ext === ".rs" || ext === ".ts";
  for (const r of renames) {
    const forms = fullSeps(r).map((via) => fullPattern(r, via));
    if (!code) forms.push(shortPattern(short(r.oldClass), r));
    if (NOVIS.has(ext)) for (const alias of importedNames(text, r.oldClass)) forms.push(shortPattern(alias, r));
    const seen = new Set<number>();
    for (const re of forms) {
      for (const m of text.matchAll(re)) {
        if (seen.has(m.index!)) continue;
        seen.add(m.index!);
        out.push({ line: lineOf(text, m.index!), text: m[0], rename: r });
      }
    }
  }
  return out.sort((a, b) => a.line - b.line);
}

/** Every text file under `root`, repo-relative with forward slashes. */
export function textFiles(root: string): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(dir).sort()) {
      if (NOT_INPUTS.has(name) || SKIPPED.has(name)) continue;
      const abs = join(dir, name);
      const st = statSync(abs);
      if (st.isDirectory()) walk(abs);
      else if (TEXT_EXTENSIONS.has(extname(name))) out.push(relative(root, abs).split("\\").join("/"));
    }
  };
  walk(root);
  return out;
}

const describe = (r: Rename) => `${r.oldClass}${sep(r)}${r.oldMember} → ${r.newMember}`;

export async function run(args: string[]): Promise<number> {
  let goal: string | undefined;
  let check = false;
  let root = ROOT;
  const classes: string[] = [];
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === "--check") check = true;
    else if (a === "--goal" && args[i + 1] !== undefined) goal = args[++i];
    else if (a === "--class" && args[i + 1] !== undefined) classes.push(args[++i]!);
    else if (a === "--root" && args[i + 1] !== undefined) root = args[++i]!;
    else {
      console.error(`api-rename: unknown argument ${a}\n${summary}`);
      return 2;
    }
  }
  if (!goal) {
    console.error(`api-rename: --goal <slug> is required\n${summary}`);
    return 2;
  }
  const goalFile = `docs/goals/${goal}.md`;
  let renames: Rename[];
  try {
    renames = readRenames(readFileSync(join(root, goalFile), "utf8"));
    if (classes.length > 0) renames = keepClasses(renames, classes);
  } catch (e) {
    console.error(`api-rename: ${goalFile}: ${(e as Error).message}`);
    return 2;
  }
  const files = textFiles(root).filter((f) => f !== goalFile && !ALLOWED.includes(f));
  if (check) {
    let left = 0;
    for (const f of files) {
      for (const h of oldNames(readFileSync(join(root, f), "utf8"), extname(f), renames)) {
        console.log(`${f}:${h.line}: \`${h.text}\` is an old name (${describe(h.rename)})`);
        left++;
      }
    }
    console.log(left === 0 ? `api-rename: no old name of ${goal} is left` : `api-rename: ${left} old name(s) of ${goal} are still written`);
    return left === 0 ? 0 : 1;
  }
  let rewritten = 0;
  let changed = 0;
  const calls: string[] = [];
  for (const f of files) {
    const abs = join(root, f);
    const text = readFileSync(abs, "utf8");
    const { text: out, count } = rewrite(text, extname(f), renames);
    if (count > 0) {
      writeFileSync(abs, out);
      rewritten += count;
      changed++;
    }
    for (const h of instanceCalls(out, renames)) calls.push(`${f}:${h.line}: \`${h.text.trim()}\` may be ${describe(h.rename)}`);
  }
  console.log(`api-rename: rewrote ${rewritten} name(s) in ${changed} file(s)`);
  if (calls.length > 0) {
    console.log(`api-rename: ${calls.length} instance call(s) to read and fix by hand:`);
    for (const c of calls) console.log(c);
  }
  return 0;
}
