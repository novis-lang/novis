// `bun nv api-rename --goal <slug> [--check] [--root <dir>]`: renames `Core` and package members across the
// tree from the `renames` block of `docs/goals/<slug>.md`, the map `rule:core-api/verb-lexicon`'s renames
// are written in. The map lives in the goal file, so it is deleted with the goal and nothing in the tree
// keeps a list of old names.
//
// The block is a fenced code block whose info string is `renames`, one pair per line:
// `Core\Response::json Core\Response::sendJson` for a static member, `Core\Sse->retry Core\Sse->setRetryDelay`
// for an instance member. `#` starts a comment line.
//
// Without `--check` it rewrites, in every text file of the tree (`TEXT_EXTENSIONS`):
//
//   * every name written in full, `Core\Response::json` or `Core\Sse->retry`, with a leading `\` or not, and
//     with the separators doubled as a Rust or TypeScript string writes them (`Core\\Response::json`);
//   * in a `.nvs`, `.nvst` or `.lspt` file, `Response::json` where the file has `use Core\Response;`, and
//     `R::json` where it has `use Core\Response as R;`;
//   * in a prose file (`.md`, `.mdx`, `.astro`), the short form a page writes, `Response::json` and
//     `Sse->retry`, since a page never declares a class of that name.
//
// It never rewrites `->retry(`: an instance call needs its receiver's type, so it lists each one as
// `file:line` for a session to fix by reading. In a project, `nvs check` names the ones left as unknown
// members.
//
// With `--check` it rewrites nothing, and exits 1 naming every old name still written in either form, outside
// the goal file and `website/src/data/changes.json`. A `.rs` or `.ts` file is held to the full form alone,
// since `Response::json` there may be a Rust path. It exits 2 on a bad argument or a goal with no block.
//
// What it spends: one read of each text file, about a second over the tree.

import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { extname, join, relative } from "node:path";
import { NOT_INPUTS, ROOT } from "../lib/paths.ts";

export const summary = "rename Core and package members across the tree from a goal's renames block: nv api-rename --goal <slug> [--check] [--root <dir>]";

/** One `Old New` pair of the block. `instance` is true for `->`. */
export interface Rename {
  oldClass: string;
  oldMember: string;
  newClass: string;
  newMember: string;
  instance: boolean;
}

/** The files the sweep reads. */
export const TEXT_EXTENSIONS = new Set([".nvs", ".nvst", ".lspt", ".md", ".mdx", ".astro", ".rs", ".ts"]);
const NOVIS = new Set([".nvs", ".nvst", ".lspt"]);
const PROSE = new Set([".md", ".mdx", ".astro"]);

/** Directories never swept, beside `NOT_INPUTS`: build output and generated pages. */
const SKIPPED = new Set(["dist", ".astro", "public"]);

/** The files `--check` ignores, repo-relative: the changelog entries, and the goal file itself. */
const ALLOWED = ["website/src/data/changes.json"];

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

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const sep = (r: Rename) => (r.instance ? "->" : "::");
const short = (cls: string) => cls.slice(cls.lastIndexOf("\\") + 1);

/** `Core\Response` matched with each `\` written once or twice, after no word character or `\`. */
function fullClass(cls: string): string {
  return `(?<![\\w\\\\])(\\\\{0,2})${cls.split("\\").map(escape).join("(\\\\{1,2})")}`;
}

/** The full form of `r`'s old name, with a group for each separator so a rewrite keeps the spelling. */
function fullPattern(r: Rename): RegExp {
  return new RegExp(`${fullClass(r.oldClass)}${escape(sep(r))}${escape(r.oldMember)}(?!\\w)`, "g");
}

/** `r`'s new full name, written with the separators `groups` matched. */
function fullReplacement(r: Rename, groups: string[]): string {
  const lead = groups[0] ?? "";
  const middle = groups[1] ?? "\\";
  return `${lead}${r.newClass.split("\\").join(middle)}${sep(r)}${r.newMember}`;
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
    out = out.replace(fullPattern(r), (...m) => {
      count++;
      return fullReplacement(r, m.slice(1, 1 + r.oldClass.split("\\").length) as string[]);
    });
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

/** Each `->old(` call of an instance rename in `text`, which only a reader of the receiver can rename. */
export function instanceCalls(text: string, renames: Rename[]): Hit[] {
  const out: Hit[] = [];
  for (const r of renames) {
    if (!r.instance) continue;
    const re = new RegExp(`(?<!(?<![\\w$\\\\])[A-Z]\\w*)\\??->\\s*${escape(r.oldMember)}\\s*\\(`, "g");
    for (const m of text.matchAll(re)) out.push({ line: lineOf(text, m.index!), text: m[0], rename: r });
  }
  return out.sort((a, b) => a.line - b.line);
}

/** Every old name `text` still writes, in the forms `--check` holds a file of extension `ext` to. */
export function oldNames(text: string, ext: string, renames: Rename[]): Hit[] {
  const out: Hit[] = [];
  const code = ext === ".rs" || ext === ".ts";
  for (const r of renames) {
    const forms = [fullPattern(r)];
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
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === "--check") check = true;
    else if (a === "--goal" && args[i + 1] !== undefined) goal = args[++i];
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
