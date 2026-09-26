// The keys a footprint is made of, and how a footprint log's lines become keys.
//
// A key names one thing an atom was seen to use. Each kind is spelled `<kind>:<value>`:
//
// | key | what it is |
// |---|---|
// | `fn:<file>#<item>` | a Rust item that ran, mapped from coverage (`items.ts` names items) |
// | `fn:<file>#*` | some code of that file ran that no item could be found for, so any change to the file selects the atom |
// | `class:<core\name>` | a `Core` class looked up in the registry, lowercased; `class:*` is the whole roster |
// | `card:<core\name>` | a `Core` class's card printed or served, lowercased; `card:*` is every card |
// | `file:<path>` | a repo file read whole |
// | `dir:<path>` | a directory listed: its names, not what they hold; `dir:*` is every name of the tree |
// | `exists:<path>` | a path tested for existence |
// | `tree:<path>` | a file or a directory a test asked `nvs_repo` for: everything beneath it; `tree:.` is the whole tree |
// | `named:<name>` | every file called `<name>`, anywhere |
// | `mod:<path>` | a TypeScript module a `bun nv` process loaded |
// | `tests:<package>` | computed: held by every test binary of the package, moved by a test added to it |
// | `*` | something ran that could not be attributed at all: any change selects the atom |
//
// Paths are repo-relative with `/`. A path outside the repository, or under a directory that is never
// an input (`target`, `.agent-tmp`, `.git`, ...), is no key: nothing a change to the tree can move.
// Class and card names are lowercased because the compiler compares them without regard to ASCII case.

import { isAbsolute, posix, resolve } from "node:path";
import { NOT_INPUTS, ROOT } from "../lib/paths.ts";
import type { Reads } from "../lib/reads.ts";

export const WILD = "*";

export const fnKey = (file: string, id: string) => `fn:${file}#${id}`;
export const fileWild = (file: string) => `fn:${file}#*`;
/** The prefix every item key of `file` starts with. */
export const itemPrefix = (file: string) => `fn:${file}#`;
export const classKey = (name: string) => `class:${norm(name)}`;
export const cardKey = (name: string) => `card:${norm(name)}`;
/** Held by every test binary of `pkg`, and moved by a test added anywhere in it. */
export const testsKey = (pkg: string) => `tests:${pkg}`;
export const ALL_CLASSES = "class:*";
export const ALL_CARDS = "card:*";
export const WHOLE_TREE = "tree:.";

/** A class or card name as a key holds it: lowercased, without a leading `\`. */
export function norm(name: string): string {
  return name === WILD ? WILD : name.replace(/^\\+/, "").toLowerCase();
}

/** The kind of a key: the text before its first `:`, or `*` for the wild key. */
export function kindOf(key: string): string {
  if (key === WILD) return WILD;
  const at = key.indexOf(":");
  return at < 0 ? key : key.slice(0, at);
}

/** The file a `fn:` key names, or null for another kind. */
export function fileOfFn(key: string): string | null {
  if (!key.startsWith("fn:")) return null;
  const hash = key.indexOf("#");
  return hash < 0 ? null : key.slice(3, hash);
}

/**
 * `path` as a key spells it, or null when it is no input: repo-relative, `/`-separated, `..` resolved.
 * An absolute path is compared with the root without regard to case on Windows, where the file system
 * does not regard it either.
 */
export function repoPath(path: string, root: string = ROOT): string | null {
  let p = path.replace(/\\/g, "/");
  if (p.startsWith("//?/")) p = p.slice(4);
  const r = root.replace(/\\/g, "/").replace(/\/+$/, "");
  let rel: string;
  if (isAbsolute(p) || /^[A-Za-z]:\//.test(p)) {
    p = posix.normalize(resolve(p).replace(/\\/g, "/"));
    const win = process.platform === "win32";
    const head = p.slice(0, r.length);
    const same = win ? head.toLowerCase() === r.toLowerCase() : head === r;
    if (!same) return null;
    if (p.length === r.length) return ".";
    if (p[r.length] !== "/") return null;
    rel = p.slice(r.length + 1);
  } else {
    rel = posix.normalize(p);
    if (rel.startsWith("../") || rel === "..") return null;
  }
  rel = rel.replace(/\/+$/, "");
  if (rel === "" || rel === ".") return ".";
  if (NOT_INPUTS.has(rel.split("/")[0]!)) return null;
  return rel;
}

/** What one footprint log says, as keys. A line that cannot be read makes the whole log wild: a log is
 * written by this repository's own code, so a line it cannot read means the reader is out of date. */
export function logKeys(text: string, root: string = ROOT): Set<string> {
  const keys = new Set<string>();
  for (const raw of text.split("\n")) {
    const line = raw.replace(/\r$/, "");
    if (line === "") continue;
    const tab = line.indexOf("\t");
    if (tab < 0) {
      keys.add(WILD);
      continue;
    }
    const kind = line.slice(0, tab);
    const value = line.slice(tab + 1);
    switch (kind) {
      case "class":
        keys.add(classKey(value));
        break;
      case "card":
        keys.add(cardKey(value));
        break;
      case "named":
        keys.add(`named:${value}`);
        break;
      case "file":
      case "dir":
      case "exists":
      case "tree": {
        const p = repoPath(value, root);
        if (p === null) break;
        if (p === "." && kind !== "tree") keys.add(`${kind}:.`);
        else keys.add(`${kind}:${p}`);
        break;
      }
      default:
        keys.add(WILD);
    }
  }
  return keys;
}

/** Held by an atom that listed every file name of the tree; moved by any path that came or went. */
export const ALL_NAMES = "dir:*";

/** The subcommands of `git` that read nothing of the working tree: history, refs and configuration. */
const GIT_HISTORY = new Set(["rev-parse", "log", "show", "cat-file", "merge-base", "config", "for-each-ref", "rev-list", "ls-tree", "describe", "symbolic-ref", "worktree", "branch", "tag", "hash-object", "var", "version"]);

/** What a program a `bun nv` process started read of the tree, as keys. `bun` and `nvs` record their own
 * reads in their own logs, and most tools read no file of the tree. `git` does, and keeps no log:
 * `ls-files` lists every name (`dir:*`), and `grep`, `status`, `diff` or a subcommand not known here
 * read what they like (`tree:.`). */
export function spawnKeys(argv: string[]): string[] {
  const exe = (argv[0] ?? "").replace(/\\/g, "/").split("/").pop()!.toLowerCase().replace(/\.exe$/, "");
  if (exe !== "git") return [];
  let i = 1;
  while (i < argv.length && argv[i]!.startsWith("-")) {
    // A git pointed at a directory outside the tree reads nothing of it.
    if (argv[i] === "-C" && repoPath(argv[i + 1] ?? "") === null) return [];
    i += argv[i] === "-C" || argv[i] === "-c" ? 2 : 1;
  }
  const sub = argv[i] ?? "";
  if (GIT_HISTORY.has(sub)) return [];
  if (sub === "ls-files") return [ALL_NAMES];
  return [WHOLE_TREE];
}

/** What a `bun nv` process read (`lib/reads.ts`), as keys, with what the programs it started read
 * (`spawnKeys`). */
export function readsKeys(reads: Reads, modules: string[] = []): Set<string> {
  const keys = new Set<string>();
  for (const f of reads.files) keys.add(`file:${f}`);
  for (const f of reads.exists) keys.add(`exists:${f}`);
  for (const f of reads.dirs) keys.add(`dir:${f}`);
  for (const argv of reads.spawns ?? []) for (const k of spawnKeys(argv)) keys.add(k);
  for (const m of modules) {
    const p = repoPath(m);
    if (p !== null) keys.add(`mod:${p}`);
  }
  return keys;
}

/** The keys a changed path moves, other than its Rust items: the file itself, every directory it sits
 * in as a `tree:`, its name, and, for a path that came or went, the test for it and its directory's
 * listing. A `.ts` file is also a module. */
export function pathKeys(path: string, cameOrWent: boolean): string[] {
  const keys = [`file:${path}`, `named:${posix.basename(path)}`, WHOLE_TREE];
  const parts = path.split("/");
  for (let i = 1; i <= parts.length; i++) keys.push(`tree:${parts.slice(0, i).join("/")}`);
  if (cameOrWent) {
    keys.push(`exists:${path}`, ALL_NAMES);
    const dir = parts.length > 1 ? parts.slice(0, -1).join("/") : ".";
    keys.push(`dir:${dir}`);
    // A path under a directory that was tested for existence may have made it come or go.
    for (let i = 1; i < parts.length; i++) keys.push(`exists:${parts.slice(0, i).join("/")}`);
  }
  if (path.endsWith(".ts")) keys.push(`mod:${path}`);
  return keys;
}
