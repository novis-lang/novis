// What `looper sync` writes: each managed file and block of a project, compared with the templates
// (`template.ts`), as `crates/looper-core/src/sync.rs` compares them. Lint and `sync --check` read the
// plan, and `apply` writes it, together with the kit's copy that `copy.ts` plans.
//
// `plan` returns one `Change` per file whose managed text differs from the templates, holding the file's
// text before and after. A project that carries the kit's copy is compared with the kit's variants of
// the templates. Each template is rendered with `varsFromConfig` first. Each kind is compared its own
// way:
//
// - **Whole**: the file is the rendered template. A missing file is written.
// - **Block**: each template block replaces the file's block of the same name. A block the file lacks
//   is appended after a blank line, except in `.gitignore` and `.gitattributes`, where it goes first so
//   the project's own lines win. The text outside the blocks stays as it is.
// - **Merged**: `.claude/settings.json` loses every `PowerShell(…)` allow entry, and gains each allow
//   entry and each `env` key it lacks. A shell prefix that runs the copy's `sh` takes the template's
//   value, and any other prefix is kept. An entry that runs the guard, `looper guard` or the copy's, takes
//   the template's matcher, and a `looper guard` command becomes the copy's. The template's `PreToolUse`
//   entry is added when none runs the guard. A file that changes is written with sorted keys, as
//   `serde_json` prints it.
// - **Seed**: never compared.
//
// A path the project's `[sync] skip` lists is never compared. A file whose markers do not parse, or a
// settings file that is not JSON, refuses the whole plan.
//
// A plan holds two copies of each changed file's text, and the templates are a fixed set.

import { mkdirSync, readFileSync, rmSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import type { Config } from "../goal/config.ts";
import { writeFile } from "../goal/edit.ts";
import { lines } from "../goal/model.ts";
import { COPY, type CopyChange, ROOT_SHIMS, SHIMS } from "./copy.ts";
import { blocks as parseBlocks, CORE, inclusiveLines, kind, PROFILES, profile, render, type Template, varsFromConfig } from "./template.ts";

/** One managed file whose text differs from the templates. `before` is `undefined` for a missing file. */
export interface Change {
  path: string;
  before: string | undefined;
  after: string;
}

/**
 * Writes each change into the project at `root`, each through a synced temp file and a rename, and
 * removes each file whose `after` is `undefined`. It writes in order, so a plan that ends on a manifest
 * names its files only once they stand.
 */
export function apply(root: string, changes: readonly CopyChange[]): void {
  for (const c of changes) {
    const path = join(root, c.path);
    if (c.after === undefined) {
      rmSync(path, { force: true });
      continue;
    }
    mkdirSync(dirname(path), { recursive: true });
    writeFile(path, c.after);
  }
}

/**
 * A line diff of one change: the lines before and after the part that differs, as `-` and `+` lines
 * under a `--- path` header. The lines both sides share at the start and the end are left out. A file
 * of the kit's copy or a shim gets the header alone, since its text is the kit's and not the project's.
 */
export function diff(c: CopyChange): string {
  const state = c.after === undefined ? "removed" : c.before === undefined ? "new" : undefined;
  const old = c.before === undefined ? [] : lines(c.before);
  const now = c.after === undefined ? [] : lines(c.after);
  let head = 0;
  while (head < old.length && head < now.length && old[head] === now[head]) head++;
  let tail = 0;
  while (tail < old.length - head && tail < now.length - head && old[old.length - 1 - tail] === now[now.length - 1 - tail]) tail++;
  let out = `--- ${c.path} (${state ?? `line ${head + 1}`})\n`;
  if (c.path.startsWith(`${COPY}/`) || [...SHIMS, ...ROOT_SHIMS].some(([shim]) => shim === c.path)) return out;
  for (const l of old.slice(head, old.length - tail)) out += `-${l}\n`;
  for (const l of now.slice(head, now.length - tail)) out += `+${l}\n`;
  return out;
}

/**
 * The templates written into a project of `stacks`: the core's, then each stack's. It throws on an
 * unknown stack.
 */
export function templates(stacks: string[]): Template[] {
  const all = profile(CORE) ?? [];
  for (const s of stacks) {
    const found = profile(s);
    if (found === undefined) throw new Error(`looper has no stack \`${s}\`: the stacks are ${PROFILES.join(", ")}`);
    all.push(...found);
  }
  return all;
}

/** The file at `path` as UTF-8, or `undefined` when it does not exist. */
function readText(path: string): string | undefined {
  let bytes: Buffer;
  try {
    bytes = readFileSync(path);
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return undefined;
    throw e;
  }
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    throw new Error("stream did not contain valid UTF-8");
  }
}

/** Every change sync makes to the project at `root`, in template order. */
export function plan(root: string, cfg: Config): Change[] {
  const vars = varsFromConfig(cfg);
  const changes: Change[] = [];
  for (const t of templates(cfg.project.stacks)) {
    const k = kind(t);
    if (k === "seed") continue;
    const path = render(t.path, vars);
    if (cfg.sync.skip.includes(path)) continue;
    const before = readText(join(root, path));
    let after: string;
    if (before === undefined || k === "whole") after = render(t.text, vars);
    else if (k === "block") after = mergeBlocks(path, before, render(t.text, vars));
    else after = merged(path, before, t.text);
    if (before !== after) changes.push({ path, before, after });
  }
  return changes;
}

/** `text` with each block of the rendered template in place of the file's block of the same name. */
function mergeBlocks(path: string, text: string, template: string): string {
  let have;
  try {
    have = parseBlocks(text);
  } catch (e) {
    throw new Error(`${path}: ${(e as Error).message}. Fix its markers by hand, then sync again`);
  }
  let want;
  try {
    want = parseBlocks(template);
  } catch (e) {
    throw new Error(`the template ${path}: ${(e as Error).message}`);
  }
  const lines = inclusiveLines(text);
  const placed = want
    .flatMap((w) => {
      const h = have.find((h) => h.name === w.name);
      return h === undefined ? [] : [{ w, h }];
    })
    .sort((a, b) => b.h.begin - a.h.begin);
  for (const { w, h } of placed) lines.splice(h.begin - 1, (h.end ?? lines.length) - (h.begin - 1), w.text);
  let out = lines.join("");
  const missing = want.filter((w) => !have.some((h) => h.name === w.name));
  if (goesFirst(path)) {
    let head = missing.map((w) => w.text).join("\n");
    if (head !== "" && out !== "") head += "\n";
    return head + out;
  }
  for (const w of missing) {
    if (out !== "" && !out.endsWith("\n")) out += "\n";
    if (out !== "") out += "\n";
    out += w.text;
  }
  return out;
}

/** Whether a block the file at `path` lacks goes before its text, so the project's own lines win. */
function goesFirst(path: string): boolean {
  const name = basename(path);
  return name === ".gitignore" || name === ".gitattributes";
}

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type JsonObject = { [key: string]: Json };

function isObject(v: Json | undefined): v is JsonObject {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** The value at `key` of `v`, or `undefined`, as `serde_json` indexes a value of any type. */
function at(v: Json | undefined, key: string | number): Json | undefined {
  if (typeof key === "number") return Array.isArray(v) ? v[key] : undefined;
  return isObject(v) && Object.hasOwn(v, key) ? v[key] : undefined;
}

/** Whether two JSON values are equal, with an object's keys in any order. */
function same(a: Json | undefined, b: Json | undefined): boolean {
  if (Array.isArray(a)) return Array.isArray(b) && a.length === b.length && a.every((x, i) => same(x, b[i]));
  if (isObject(a)) {
    if (!isObject(b)) return false;
    const keys = Object.keys(a);
    return keys.length === Object.keys(b).length && keys.every((k) => Object.hasOwn(b, k) && same(a[k], b[k]));
  }
  return a === b;
}

/** `v` with every object's keys sorted, as `serde_json` holds them. */
function sorted(v: Json): Json {
  if (Array.isArray(v)) return v.map(sorted);
  if (!isObject(v)) return v;
  const out: JsonObject = {};
  for (const k of Object.keys(v).sort()) out[k] = sorted(v[k] as Json);
  return out;
}

/**
 * The project's settings `text` merged with the template's: its `PowerShell(…)` allow entries gone, the
 * template's allow entries and `env` keys added, a shell prefix that runs the copy's `sh` given the
 * template's, and the guard hook added or given the template's matcher.
 * A guard hook that runs `looper guard` is given the template's command, which runs the copy.
 */
function merged(path: string, text: string, template: string): string {
  let have: Json;
  try {
    have = JSON.parse(text);
  } catch (e) {
    throw new Error(`${path}: not JSON: ${(e as Error).message}`);
  }
  let want: Json;
  try {
    want = JSON.parse(template);
  } catch (e) {
    throw new Error(`the template ${path}: ${(e as Error).message}`);
  }
  const before: Json = structuredClone(have);
  if (!isObject(have)) throw new Error(`${path}: not a JSON object`);
  const object = have;
  const allow = slot(object, "permissions", "allow");
  const kept = allow.filter((e) => !(typeof e === "string" && e.startsWith("PowerShell(")));
  allow.splice(0, allow.length, ...kept);
  const wantAllow = at(at(want, "permissions"), "allow");
  for (const entry of Array.isArray(wantAllow) ? wantAllow : []) {
    if (!allow.some((a) => same(a, entry))) allow.push(structuredClone(entry));
  }
  const keys = at(want, "env");
  if (isObject(keys)) {
    if (!Object.hasOwn(object, "env")) object.env = {};
    const env = object.env;
    if (!isObject(env)) throw new Error(`${path}: \`env\` is not what Claude Code reads`);
    for (const [key, value] of Object.entries(keys)) {
      const old = env[key];
      const ours = key === PREFIX && typeof old === "string" && KIT_SH.test(old);
      if (!Object.hasOwn(env, key) || ours) env[key] = structuredClone(value);
    }
  }
  const guarded = runsGuard(before);
  const hooks = slot(object, "hooks", "PreToolUse");
  const wantHooks = at(at(want, "hooks"), "PreToolUse");
  if (guarded) {
    const matcher = at(at(wantHooks, 0), "matcher") ?? null;
    const command = at(at(at(at(wantHooks, 0), "hooks"), 0), "command");
    for (const h of hooks) {
      if (!isGuard(h) || !isObject(h)) continue;
      h.matcher = structuredClone(matcher);
      const all = at(h, "hooks");
      if (typeof command !== "string" || !Array.isArray(all)) continue;
      for (const c of all) if (isObject(c) && typeof c.command === "string" && c.command.trim() === "looper guard") c.command = command;
    }
  } else {
    hooks.push(...(Array.isArray(wantHooks) ? structuredClone(wantHooks) : []));
  }
  if (same(have, before)) return text;
  return `${JSON.stringify(sorted(have), null, 2)}\n`;
}

/** The `env` key that names Claude Code's shell prefix. */
const PREFIX = "CLAUDE_CODE_SHELL_PREFIX";

/** A shell prefix that runs the copy's `sh`, whatever shell starts it. */
const KIT_SH = /\/tools\/looper\/main\.ts" sh "\$0"/;

/** The guard hook's command in the kit's variant of the settings, which runs the copy. */
const KIT_GUARD = /^bun "\$CLAUDE_PROJECT_DIR\/tools\/looper\/main\.ts" guard$/;

/**
 * Whether one of the `PreToolUse` entries of the parsed `.claude/settings.json` runs the guard,
 * `looper guard` or the copy's.
 */
export function runsGuard(settings: Json): boolean {
  const entries = at(at(settings, "hooks"), "PreToolUse");
  return Array.isArray(entries) && entries.some(isGuard);
}

/** Whether one `PreToolUse` entry runs `looper guard` or the copy's guard. */
function isGuard(entry: Json): boolean {
  const all = at(entry, "hooks");
  return Array.isArray(all) && all.some((c) => {
    const command = at(c, "command");
    return typeof command === "string" && (command.trim() === "looper guard" || KIT_GUARD.test(command.trim()));
  });
}

/** The array at `object[outer][inner]`, created empty when it is missing. */
function slot(object: JsonObject, outer: string, inner: string): Json[] {
  const not = (what: string) => new Error(`.claude/settings.json: \`${what}\` is not what Claude Code reads`);
  if (!Object.hasOwn(object, outer)) object[outer] = {};
  const o = object[outer];
  if (!isObject(o)) throw not(outer);
  if (!Object.hasOwn(o, inner)) o[inner] = [];
  const i = o[inner];
  if (!Array.isArray(i)) throw not(`${outer}.${inner}`);
  return i;
}
