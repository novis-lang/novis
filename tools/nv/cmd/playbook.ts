// `bun nv playbook`: prints a bullet by its selector, audits the playbook, and deletes every bullet
// whose `[until:]` condition holds with the goal manifest lines that named only a bullet that went.
// `tools/playbook.py`'s module doc is the trailer's syntax and the reasoning behind it.
//
//     bun nv playbook --show <selector>       one bullet, or a whole section, as `nv orient` prints it
//     bun nv playbook --check                 expiry, stale paths, selectors, section sizes; 1 on a gating finding
//     bun nv playbook --closes <slug>         1 while a carried-gaps § Owned row names that goal
//     bun nv playbook --retire [--dry-run]
//
// Four files declare what retires their blocks: every playbook fragment under `docs/agent/playbook/`,
// and `DECLARING`'s three append-mostly files. `expiryReport` evaluates each block's trailer against the
// tree, and `retire` deletes the ones that hold. A fragment file holds one bullet, so retiring it deletes
// the file and its record under `data/playbook/`. In the other files the block's lines go.
//
// Pruning the manifests is what keeps a retirement from halting the loop: a goal names its bullets by
// lead-in in its `[context] playbook`, `nv chain --check` refuses a selector that reaches nothing, and
// that check is on every goal's floor. So each selector that reached a bullet before the retirement and
// reaches nothing after it is dropped from every live goal's `.toml`, from `docs/agent/loop-goal.toml`,
// and from every live goal's record. A selector that still reaches another bullet stays, and so does one
// that already reached nothing, which is `nv chain --check`'s finding for a reader.

import { existsSync, readFileSync, statSync, unlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { parse as parseToml } from "smol-toml";
import { list } from "../import/lib.ts";
import { namedPaths, playbook as playbookImporter } from "../import/playbook.ts";
import { chainGoals } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { section as proseSection } from "../lib/prose.ts";
import { pyRepr } from "../lib/py.ts";
import { load, pathOf, remove as removeRecord, write as writeRecord } from "../lib/store.ts";
import { goal as goalType } from "../schema/goal.ts";
import { playbookBullet, playbookSection } from "../schema/playbook.ts";
import { allBullets, playbookBook, sliceBullets, type BookSection } from "./orient.ts";

export const summary = "the playbook: nv playbook --show <selector> | --check | --closes <slug> | --retire [--dry-run]";

const PLAYBOOK_DIR = "docs/agent/playbook";
const GOALS_DIR = "docs/agent/goals";
const GOAL_TOML = "docs/agent/loop-goal.toml";
/**
 * The index of gaps a goal owns. The file is deleted, and `--check` and `--closes` still read its
 * § *Owned* rows out of this path, so an index rebuilt here is gated from its first row.
 */
const CARRIED_GAPS = "docs/agent/carried-gaps.md";

/** The trailer every bullet ends with. `tools/playbook.py`'s module doc is its syntax's one home. */
export const EXPIRY = /\[until:\s*(test|exists|gone|rule|reviewed)\s+([^\]]+?)\s*\]\s*$/;
/** Anything that looks like a trailer and did not parse as one, so a typo is a finding. */
const EXPIRY_LIKE = /\[until:[^\]]*\]?\s*$/;
/** How old a `reviewed` date may be before the bullet is owed a re-read. */
const REVIEW_DAYS = 14;

/** The other append-mostly files, and which of their blocks must declare: by section heading and first line. */
const DECLARING: [string, (head: string, first: string) => boolean][] = [
  ["docs/agent/guard-name-debt.md", (_head, first) => first.startsWith("- [")],
  [CARRIED_GAPS, (head) => head === "Unowned"],
  ["docs/agent/carried-refusals.md", (_head, first) => /^9\d\d\. /.test(first)],
];

/** One `playbook = [ ... ]` entry on a line of its own, a trailing comma and comment allowed. */
const SELECTOR_LINE = /^\s*('[^']*'|"(?:[^"\\]|\\.)*")\s*,?\s*(?:#.*)?$/;
const LIST_OPEN = /^\s*playbook\s*=\s*\[\s*(?:#.*)?$/;

export interface Block {
  section: string;
  first: string;
  /** 0-based line indices, both inclusive. */
  start: number;
  end: number;
  body: string;
  lead: string;
}

/** The first `n` characters of `s`, counted as Python counts them. */
const head = (s: string, n: number) => [...s].slice(0, n).join("");

/**
 * Every `- ` bullet and `NNN. ` entry at column 0, with its line span and section. A block runs to the
 * next block at column 0, the next heading, or the end, and a blank line ends it only when what follows
 * is not indented continuation, since `carried-refusals.md`'s entries carry indented paragraphs.
 */
export function blocks(text: string): Block[] {
  const lines = text.split("\n");
  const starts = /^(- |\d+\. )/;
  const out: Omit<Block, "body" | "lead">[] = [];
  let cur: Omit<Block, "body" | "lead"> | null = null;
  let section = "";
  lines.forEach((line, i) => {
    if (/^#{1,6}\s/.test(line)) {
      if (cur) out.push(cur);
      cur = null;
      section = line.replace(/^#+\s*/, "").trim();
    } else if (starts.test(line)) {
      if (cur) out.push(cur);
      cur = { section, first: line, start: i, end: i };
    } else if (cur && !line.trim()) {
      const nxt = lines[i + 1] ?? "";
      if (nxt && !nxt.startsWith(" ") && !nxt.startsWith("\t") && !starts.test(nxt)) {
        out.push(cur);
        cur = null;
      }
    } else if (cur) {
      cur.end = i;
    }
  });
  if (cur) out.push(cur);
  return out.map((b) => ({
    ...b,
    body: lines.slice(b.start, b.end + 1).join("\n").trimEnd(),
    lead: head(b.first.replace(/^(- (?:\[.\] )?|\d+\. )/, ""), 70),
  }));
}

/** The `[until: kind arg]` a block ends with, or null when it declares nothing or wraps its trailer over a line. */
export function declaration(body: string): { kind: string; arg: string } | null {
  const m = EXPIRY.exec(body.trimEnd());
  return m && !m[2]!.includes("\n") ? { kind: m[1]!, arg: m[2]!.trim() } : null;
}

/** Whether the block ends with a trailer broken across a line, which `declaration` refuses. */
function wrapped(body: string): boolean {
  const m = EXPIRY.exec(body.trimEnd());
  return m !== null && m[2]!.includes("\n");
}

/** Every `fn <name>` in a `.rs` file git tracks under `root`. */
async function testNames(root: string): Promise<Set<string>> {
  const done = await runProc(["git", "grep", "-h", "-o", "-E", "\\bfn [A-Za-z_][A-Za-z0-9_]*", "--", "*.rs"], { cwd: root, timeoutMs: 60_000 });
  return new Set(done.stdout.split("\n").filter((l) => l.startsWith("fn ")).map((l) => l.slice(3).trim()));
}

/** Days from the calendar day `from` to the calendar day `to`, both read in local time. */
function daysBetween(from: Date, to: Date): number {
  const day = (d: Date) => Date.UTC(d.getFullYear(), d.getMonth(), d.getDate());
  return Math.round((day(to) - day(from)) / 86_400_000);
}

/**
 * Whether a declared condition holds on `today`, and a word on why: true when the block has expired,
 * false when it stands, and null when the declaration cannot be evaluated, which is a finding rather
 * than a guess either way.
 */
export function holds(root: string, kind: string, arg: string, today: Date, tests: ReadonlySet<string>): [boolean | null, string] {
  if (kind === "reviewed") {
    const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(arg);
    const when = m ? new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3])) : null;
    if (!when || when.getMonth() !== Number(m![2]) - 1 || when.getDate() !== Number(m![3])) return [null, `\`${arg}\` is not a YYYY-MM-DD date`];
    const age = daysBetween(when, today);
    if (age < 0) return [false, `dated ${-age} days ahead -- owed a re-read, since no one read it then`];
    if (age > REVIEW_DAYS) return [false, `reviewed ${age} days ago -- owed a re-read`];
    return [false, `reviewed ${age} days ago`];
  }
  if (kind === "test") {
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(arg)) return [null, `\`${arg}\` is not a test function name`];
    const there = tests.has(arg);
    return [there, `fn ${arg} ${there ? "exists" : "is not in the tree"}`];
  }
  if (kind === "rule") {
    const there = existsSync(join(root, "docs/rules", `${arg}.md`));
    return [there, `docs/rules/${arg}.md ${there ? "exists" : "does not exist"}`];
  }
  if (kind === "exists" || kind === "gone") {
    const colon = arg.indexOf(":");
    const path = (colon < 0 ? arg : arg.slice(0, colon)).trim().replace(/\\/g, "/");
    const needle = colon < 0 ? "" : arg.slice(colon + 1);
    if (!path || /[*?<>]/.test(path)) return [null, `\`${path}\` is not a repo path`];
    const full = join(root, path);
    const onDisk = existsSync(full);
    let present = onDisk;
    if (present && needle) {
      try {
        present = statSync(full).isFile() && readFileSync(full, "utf8").includes(needle);
      } catch {
        present = false;
      }
    }
    const what = present
      ? `${path}${needle ? ` holds ${pyRepr(needle)}` : ""}`
      : onDisk
        ? `${path} no longer holds ${pyRepr(needle)}`
        : `${path} does not exist`;
    return [kind === "exists" ? present : !present, what];
  }
  return [null, `unknown kind ${pyRepr(kind)}`];
}

export interface Expired extends Block {
  /** Repo-relative. */
  file: string;
  /** 1-based. */
  line: number;
  kind: string;
  arg: string;
  why: string;
}

export interface Finding {
  file: string;
  line: number;
  lead: string;
  why: string;
}

/** Every file whose blocks must declare what retires them, repo-relative, with which blocks must. */
function declaring(root: string): [string, (head: string, first: string) => boolean][] {
  const sections = playbookImporter
    .read(root)
    .records.filter((r) => r.type === playbookSection)
    .sort((a, b) => (a.value as { order: number }).order - (b.value as { order: number }).order);
  const fragments = sections.flatMap((s) =>
    list(root, `${PLAYBOOK_DIR}/${s.id}`)
      .filter((n) => n.endsWith(".md"))
      .map((n) => [`${PLAYBOOK_DIR}/${s.id}/${n}`, () => true] as [string, (head: string, first: string) => boolean]),
  );
  return [...fragments, ...DECLARING];
}

/**
 * An § *Owned* table's rows in `CARRIED_GAPS`'s shape: the 0-based line, the gap cell and the owner cell.
 * None when the text has no such section, which is what the deleted index gives.
 */
function ownedRows(text: string): { line: number; gap: string; owner: string }[] {
  const found = proseSection(text, "Owned");
  if (!found) return [];
  const lines = text.split("\n");
  const own = found.replace(/\n$/, "").split("\n");
  const offset = lines.indexOf(own[0]!);
  const rows: { line: number; gap: string; owner: string }[] = [];
  own.forEach((line, i) => {
    const cells = line.trim().replace(/^\|+|\|+$/g, "").split("|").map((c) => c.trim());
    if (line.startsWith("|") && cells.length >= 2 && cells[0] !== "Gap" && !/^-*$/.test(cells[0]!)) {
      rows.push({ line: offset + i, gap: cells[0]!, owner: cells[1]! });
    }
  });
  return rows;
}

/** The slug an § *Owned* row's owner cell names. */
const ownerSlug = (cell: string) => cell.trim().replace(/^`+|`+$/g, "");

/**
 * Over every declaring file: the blocks whose condition holds, the `reviewed` ones owed a re-read, the
 * ones that declare nothing or declare it wrongly, how many declare anything, and `CARRIED_GAPS`'s
 * § *Owned* rows whose owner goal has walked.
 */
export async function expiryReport(
  root: string = ROOT,
  today: Date = new Date(),
): Promise<{ expired: Expired[]; owed: Expired[]; bad: Finding[]; rows: Finding[]; declared: number }> {
  const expired: Expired[] = [];
  const owed: Expired[] = [];
  const bad: Finding[] = [];
  const rows: Finding[] = [];
  let declared = 0;
  const tests = await testNames(root);
  for (const [file, must] of declaring(root)) {
    const full = join(root, file);
    if (!existsSync(full)) continue;
    const text = readFileSync(full, "utf8").replace(/\r\n/g, "\n");
    if (file === CARRIED_GAPS) {
      const gone = new Set(chainGoals(root).filter((g) => g.retired).map((g) => g.slug));
      for (const r of ownedRows(text)) {
        const slug = ownerSlug(r.owner);
        if (gone.has(slug)) rows.push({ file, line: r.line + 1, lead: head(r.gap, 70), why: `owner goal \`${slug}\` is retired; close the gap or strike the owner` });
      }
    }
    for (const b of blocks(text)) {
      const where = { file, line: b.start + 1, lead: b.lead };
      const decl = declaration(b.body);
      if (decl !== null) declared++;
      if (decl === null) {
        if (wrapped(b.body)) {
          bad.push({ ...where, why: "the trailer is broken across two lines, so its argument holds a newline no path, needle, name or date can -- put the whole `[until: ...]` on one line, past the wrap column if need be" });
        } else if (EXPIRY_LIKE.test(b.body)) {
          bad.push({ ...where, why: "the trailer does not parse -- it is `[until: <kind> <arg>]`, kind one of test, exists, gone, rule, reviewed" });
        } else if (must(b.section, b.first)) {
          bad.push({ ...where, why: "no `[until: ...]` trailer" });
        }
        continue;
      }
      const [ok, why] = holds(root, decl.kind, decl.arg, today, tests);
      const entry: Expired = { ...b, ...where, kind: decl.kind, arg: decl.arg, why };
      if (ok === null) bad.push({ ...where, why });
      else if (ok) expired.push(entry);
      else if (decl.kind === "reviewed" && why.includes("owed")) owed.push(entry);
    }
  }
  return { expired, owed, bad, rows, declared };
}

/** `selectors` less every one that reached a bullet in `before` and reaches none in `after`, and those that went. */
function prune(selectors: string[], before: BookSection[], after: BookSection[]): { keep: string[]; went: string[] } {
  const keep: string[] = [];
  const went: string[] = [];
  for (const sel of selectors) {
    const [was] = sliceBullets(before, sel);
    const [, complaint] = sliceBullets(after, sel);
    if (was.length && complaint) went.push(sel);
    else keep.push(sel);
  }
  return { keep, went };
}

/**
 * A goal `.toml` with every `playbook` list entry that went dropped, or null when none did or when
 * dropping them leaves the file unreadable. A comment run left with nothing under it before the closing
 * `]` goes with its lines. Inline lists are not read: the goal files write each entry on a line of its own.
 */
function pruneToml(text: string, before: BookSection[], after: BookSection[], say: (line: string) => void, rel: string): { text: string; went: string[] } | null {
  const keep: string[] = [];
  const went: string[] = [];
  let inList = false;
  let commentsSinceKept = 0;
  let lastWent = false;
  for (const line of text.split("\n")) {
    if (!inList) {
      keep.push(line);
      inList = LIST_OPEN.test(line);
      commentsSinceKept = 0;
      lastWent = false;
      continue;
    }
    const stripped = line.trim();
    if (stripped === "]") {
      if (lastWent && commentsSinceKept) keep.splice(keep.length - commentsSinceKept, commentsSinceKept);
      keep.push(line);
      inList = false;
      continue;
    }
    const m = SELECTOR_LINE.exec(line);
    if (!m) {
      keep.push(line);
      if (stripped.startsWith("#")) commentsSinceKept++;
      continue;
    }
    const selector = (parseToml(`x = ${m[1]}`) as { x: string }).x;
    if (prune([selector], before, after).went.length) {
      went.push(selector);
      lastWent = true;
      continue;
    }
    keep.push(line);
    commentsSinceKept = 0;
    lastWent = false;
  }
  if (!went.length) return null;
  const out = keep.join("\n");
  try {
    parseToml(out);
  } catch (err) {
    say(`  keep    ${rel}  -- dropping ${went.length} selector(s) leaves it unreadable (${(err as Error).message.split("\n")[0]}); left for a hand`);
    return null;
  }
  return { text: out, went };
}

/** Drops each selector that went from every live goal's manifest; returns the files changed, repo-relative. */
function pruneManifests(root: string, before: BookSection[], after: BookSection[], dry: boolean, say: (line: string) => void): { files: string[]; dropped: number } {
  const files: string[] = [];
  let dropped = 0;
  const tomls = list(root, GOALS_DIR)
    .filter((n) => n.endsWith(".toml"))
    .map((n) => `${GOALS_DIR}/${n}`);
  if (existsSync(join(root, GOAL_TOML))) tomls.push(GOAL_TOML);
  for (const rel of tomls) {
    const got = pruneToml(readFileSync(join(root, rel), "utf8").replace(/\r\n/g, "\n"), before, after, say, rel);
    if (!got) continue;
    for (const sel of got.went) say(`  drop    ${rel}  ${JSON.stringify(sel)}  -- named only a bullet retired above`);
    dropped += got.went.length;
    if (!dry) writeFileSync(join(root, rel), got.text);
    files.push(rel);
  }
  for (const g of load(goalType, root)) {
    if (g.value.checks.length === 0) continue;
    const value = structuredClone(g.value);
    const lists = [value.context, ...value.stages.map((s) => s.context)].filter((c) => c?.playbook?.length);
    let went = 0;
    for (const c of lists) {
      const got = prune(c!.playbook!, before, after);
      went += got.went.length;
      c!.playbook = got.keep;
    }
    if (!went) continue;
    say(`  drop    ${g.path}  ${went} selector(s)  -- named only a bullet retired above`);
    if (!dry) writeRecord(goalType, g.id, value, root);
    files.push(g.path);
  }
  return { files, dropped };
}

/**
 * Deletes every block in `expired`, then every manifest selector that reached only a bullet that went,
 * and returns the files changed, repo-relative. A fragment file's bullet takes its record with it.
 */
export function retire(expired: Expired[], dry: boolean, root: string = ROOT, say: (line: string) => void = console.log): string[] {
  const byFile = new Map<string, Expired[]>();
  for (const e of expired) byFile.set(e.file, [...(byFile.get(e.file) ?? []), e]);
  const fragments = new Set([...byFile.keys()].filter((f) => f.startsWith(`${PLAYBOOK_DIR}/`)));
  const before = playbookBook(root);
  const after = playbookBook(root, fragments);
  const changed: string[] = [];
  for (const [file, entries] of byFile) {
    if (fragments.has(file)) {
      for (const e of entries) say(`  retire  ${file}  ${e.lead}  -- ${e.why}`);
      const id = file.slice(PLAYBOOK_DIR.length + 1, -".md".length);
      changed.push(file);
      if (existsSync(join(root, pathOf(playbookBullet, id)))) changed.push(pathOf(playbookBullet, id));
      if (!dry) {
        unlinkSync(join(root, file));
        removeRecord(playbookBullet, id, root);
      }
      continue;
    }
    const lines = readFileSync(join(root, file), "utf8").replace(/\r\n/g, "\n").split("\n");
    for (const e of [...entries].sort((a, b) => b.start - a.start)) {
      say(`  retire  ${file}:${e.line}  ${e.lead}  -- ${e.why}`);
      lines.splice(e.start, e.end - e.start + 1);
      // The blank line a deleted block leaves behind goes too, so two sections never end up two apart.
      if (e.start > 0 && e.start < lines.length && !lines[e.start]!.trim() && !lines[e.start - 1]!.trim()) lines.splice(e.start, 1);
    }
    if (!dry) writeFileSync(join(root, file), lines.join("\n"));
    changed.push(file);
  }
  const pruned = pruneManifests(root, before, after, dry, say);
  say(
    `nv playbook: ${dry ? "would delete" : "deleted"} ${expired.length} bullet(s) across ${byFile.size} file(s)` +
      (pruned.files.length ? ` and ${dry ? "would prune" : "pruned"} ${pruned.files.length} manifest(s) of the selectors that named nothing else` : "") +
      ".",
  );
  return [...changed, ...pruned.files.sort()];
}

/**
 * Bullets whose missing path is the trap itself, keyed by `selector\npath` with the reason as the value.
 * They are printed under their own heading so the stale-path list can reach `none`. An entry no bullet
 * matches is reported.
 */
const DELIBERATE_STALE = new Map<string, string>([
  [
    "Tooling > a tool's prose citing\ntests/vectors.rs", // check-links:subject
    "the suffix `check-links.py` wrongly resolves to; the file is crates/nvs-stdlib/src/tests/vectors.rs, and the bullet names both because the relation between them is the trap",
  ],
]);

/**
 * `--check`: the retirement findings, the paths a bullet names that are gone, the selectors that do not
 * name exactly one bullet, and what each section costs. It exits 1 on an unresolvable selector or a
 * bullet with no readable trailer, and reports the rest.
 *
 * It differs from `playbook.py --check` in three ways. The sizes are of the bullets as the records hold
 * them, one line each, so they are smaller than the wrapped fragment files. A selector is built from the
 * whole bold lead, which can take a word more than one built from the lead's first line. And the file's
 * growth over `git log` and the cost of the live manifest's selectors are not printed: both were reports
 * that gated nothing, and no check reads them.
 */
async function runCheck(root: string = ROOT): Promise<number> {
  const book = playbookBook(root);
  const every = allBullets(book);
  const bytes = (s: string) => Buffer.byteLength(s, "utf8");
  console.log(`${PLAYBOOK_DIR}/: ${every.reduce((n, b) => n + bytes(b.text), 0)} bytes, ${every.length} bullets\n`);

  const { expired, owed, bad, rows, declared } = await expiryReport(root);
  console.log("== BULLETS WHOSE RETIREMENT CONDITION HOLDS  (delete them: `bun nv playbook --retire`)");
  for (const e of expired) console.log(`  ${e.file}:${e.line}  ${e.lead}\n      [until: ${e.kind} ${e.arg}]  -- ${e.why}`);
  if (!expired.length) console.log(`  none -- every one of the ${declared} declared condition(s) still stands`);
  else console.log(`\n  ${expired.length} bullet(s). Each is mechanically dead: the thing it waited for is on disk,\n  or the thing it was about is gone. \`git log -S\` keeps the text; the file need not.`);

  console.log(`\n== BULLETS OWED A RE-READ  (\`reviewed\` more than ${REVIEW_DAYS} days ago, or dated ahead)`);
  for (const e of owed) console.log(`  ${e.file}:${e.line}  ${e.lead}  -- ${e.why}`);
  console.log(owed.length ? `\n  ${owed.length} bullet(s). Read each; still true bumps its date, no longer true deletes it.` : "  none");

  console.log("\n== CARRIED-GAPS ROWS WHOSE OWNER WENT GREEN WITHOUT CLOSING THEM");
  for (const r of rows) console.log(`  ${r.file}:${r.line}  ${r.lead}  -- ${r.why}`);
  if (!rows.length) console.log("  none -- every carried-gaps owner is live or struck");

  console.log("\n== BULLETS THAT DECLARE NOTHING, OR DECLARE IT WRONGLY");
  for (const e of bad) console.log(`  ${e.file}:${e.line}  ${e.lead}\n      ${e.why}`);
  if (!bad.length) console.log("  none -- every bullet ends with a trailer this tool can read");

  console.log("\n== PATHS A BULLET NAMES THAT ARE NOT IN THE TREE");
  let stale = 0;
  let splits = 0;
  const deliberate: [string, string, string][] = [];
  for (const b of every) {
    const gone: string[] = [];
    for (const one of namedPaths(b.text)) {
      if (existsSync(join(root, one))) continue;
      const why = DELIBERATE_STALE.get(`${b.selector}\n${one}`);
      if (why !== undefined) deliberate.push([b.selector, one, why]);
      else gone.push(one);
    }
    if (!gone.length) continue;
    stale++;
    console.log(`  ${b.selector}`);
    for (const g of [...new Set(gone)].sort()) {
      // `foo.rs` gone while `foo/` stands is a file that was split, and its trap usually still stands.
      const asDir = g.replace(/(?<=[^/])\.[^./]*$/, "");
      if (asDir !== g && existsSync(join(root, asDir)) && statSync(join(root, asDir)).isDirectory()) {
        splits++;
        console.log(`      ${g}  -- split into ${asDir}/, so the module still stands`);
      } else console.log(`      ${g}`);
    }
  }
  if (!stale) {
    // The sentence's first clause is what a reader of this output matches on; keep it.
    console.log(`  none -- every path any bullet names still exists${deliberate.length ? `, or is quoted on purpose (${deliberate.length} below)` : ""}`);
  } else {
    console.log(`\n  ${stale} bullet(s). A trap describing a file that is gone is usually a trap\n  someone closed. Read it before deleting it; this reports, it never prunes.`);
    if (splits) console.log(`  ${splits} of the paths above are marked \`split into\` -- those are the weakest\n  signal of the lot, because the code moved rather than went away.`);
  }
  if (deliberate.length) {
    console.log("\n== PATHS A BULLET QUOTES ON PURPOSE  (already read; not a signal)");
    for (const [sel, path, why] of deliberate) console.log(`  ${sel}\n      ${path}  -- ${why}`);
  }
  const seen = new Set(deliberate.map(([s, p]) => `${s}\n${p}`));
  const unseen = [...DELIBERATE_STALE.keys()].filter((k) => !seen.has(k)).sort();
  if (unseen.length) {
    console.log("\n== DELIBERATE_STALE ENTRIES THAT NO LONGER APPLY");
    for (const k of unseen) console.log(`  ${k.replace("\n", "  ->  ")}`);
    console.log(`\n  ${unseen.length} entry(s) matched no bullet: the bullet was reworded or deleted, or the path\n  is back in the tree. Drop the entry from this file.`);
  }

  console.log("\n== SELECTORS THAT DO NOT RESOLVE TO EXACTLY ONE BULLET");
  let unresolved = 0;
  for (const b of every) {
    const [hits, complaint] = sliceBullets(book, b.selector);
    if (complaint || hits.length !== 1) {
      unresolved++;
      console.log(`  ${b.selector}  -> ${complaint ?? `${hits.length} hits`}`);
    }
  }
  if (!unresolved) console.log(`  none -- all ${every.length} bullets are individually selectable`);

  console.log("\n== WHAT EACH SECTION COSTS A SESSION THAT NAMES IT WHOLE");
  for (const s of book) {
    const size = s.bullets.reduce((n, b) => n + bytes(b.text), 0);
    console.log(`  ${String(size).padStart(6)} B  ${String(s.bullets.length).padStart(3)} bullets   ## ${s.title}`);
  }
  console.log("\n  Nothing here refuses over a size. This is a number to weigh when a goal is\n  written, which is the only moment it can be acted on cheaply.");

  // A stale path and a size are judgement calls and stay reports. An unresolvable selector is not:
  // `nv orient` fetches a trap by exactly that string.
  if (unresolved) {
    console.log(`\n  !! ${unresolved} selector(s) above do not resolve to exactly one bullet. \`nv orient\` fetches a trap\n  by its selector, so a goal naming one of these is a trap the loop cannot deliver. Reword the\n  colliding lead-in -- the bullet's text, not this tool, is the fix.`);
  }
  if (bad.length) {
    console.log(`\n  !! ${bad.length} bullet(s) declare nothing that retires them, or declare it in a form this\n  tool cannot read. A bullet without a trailer is one the file can never let go of.`);
  }
  return unresolved || bad.length ? 1 : 0;
}

/**
 * `--closes <slug>`: every `CARRIED_GAPS` § *Owned* row that still names the goal, and 1 if there is one.
 * The retired-owner rows `--check` prints appear only once a goal is retired, which is after it was
 * reached, so the driver asks this of the goal by name on the sweep that would reach it.
 */
function runCloses(slug: string, root: string = ROOT): number {
  const full = join(root, CARRIED_GAPS);
  const text = existsSync(full) ? readFileSync(full, "utf8").replace(/\r\n/g, "\n") : "";
  const rows = ownedRows(text).filter((r) => ownerSlug(r.owner) === slug);
  if (!rows.length) {
    console.log(`nv playbook: goal \`${slug}\` owns no ${CARRIED_GAPS} row`);
    return 0;
  }
  for (const r of rows) console.log(`  ${CARRIED_GAPS}:${r.line + 1}  ${head(r.gap, 70)}`);
  console.log(`nv playbook: goal \`${slug}\` still owns ${rows.length} ${CARRIED_GAPS} row(s). A goal is reached when each gap is closed and its row deleted, or its owner struck for a reason the row states; a tag is not a build.`);
  return 1;
}

const USAGE = "usage: bun nv playbook --show <selector> | --check | --closes <slug> | --retire [--dry-run]";

export async function run(args: string[]): Promise<number> {
  if (args[0] === "--show" && args.length === 2) {
    const [found, complaint] = sliceBullets(playbookBook(), args[1]!);
    if (complaint) {
      console.log(`nv playbook: ${complaint}`);
      return 1;
    }
    console.log(found.join("\n\n"));
    return 0;
  }
  if (args[0] === "--check" && args.length === 1) return runCheck();
  if (args[0] === "--closes" && args.length === 2) return runCloses(args[1]!);
  const dry = args.includes("--dry-run");
  if (!args.includes("--retire") || args.some((a) => a !== "--retire" && a !== "--dry-run")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 2;
  }
  const { expired, bad } = await expiryReport();
  if (bad.length) {
    console.log(`nv playbook: ${bad.length} bullet(s) declare nothing or declare it wrongly. Nothing is retired while a declaration cannot be read.`);
    for (const b of bad) console.log(`  ${b.file}:${b.line}  ${b.lead}\n      ${b.why}`);
    return 1;
  }
  if (!expired.length) {
    console.log("nv playbook: no bullet's retirement condition holds; nothing to delete.");
    return 0;
  }
  retire(expired, dry);
  return 0;
}
