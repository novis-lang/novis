// `goal …`: show a goal, and every edit of `../goal/edit.ts`, each printing what it changed, as
// `crates/looper/src/cmd/goal.rs` does. Each runs on the project in the current directory.
//
// - `show [slug]` prints a goal's stages, its checks and its place on the chain, the live goal by
//   default. Its `--json` document is `{"slug", "title", "position", "summary", "stages", "checks"}`.
// - `show --extra [--stage <id>]` prints the goal's `extra` with the stage's laid over it, as compact
//   JSON, with `--json` or without.
// - `handoff set --file -` reads the handoff's text from stdin.
// - An edit prints its line. Its `--json` document is `{"message", "changed"}`, and `changed` holds the
//   goal file and the README when their bytes differ after it.
// - `harvested <slug>` prints nothing, and exits 0 when the goal's findings are written and 1 when they
//   are not. Its `--json` document is `{"slug", "harvested"}`.

import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { loadConfig } from "../goal/config.ts";
import { addCheck, addContext, moveGoal, newGoal, removeCheck, removeGoal, setHandoff, setStage } from "../goal/edit.ts";
import { emptyCheck, extraJson, GoalError, liveGoal, loadChain, loadGoal, liveStage, README, Refused } from "../goal/model.ts";
import { harvested } from "../goal/orient.ts";
import { type Command, Exit, type Report, usage } from "./cli.ts";

/** The arguments of one subcommand: its positionals, its options by name, its flags, and what follows `--`. */
interface Parsed {
  positionals: string[];
  options: Map<string, string[]>;
  flags: Set<string>;
  rest: string[] | undefined;
}

/** Splits `args` for the subcommand `what`, which takes the options `options` and the flags `flags`. */
function parse(what: string, args: string[], options: string[], flags: string[]): Parsed {
  const out: Parsed = { positionals: [], options: new Map(), flags: new Set(), rest: undefined };
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--") {
      out.rest = args.slice(i + 1);
      break;
    }
    if (!a.startsWith("--")) {
      out.positionals.push(a);
      continue;
    }
    const eq = a.indexOf("=");
    const name = a.slice(2, eq < 0 ? undefined : eq);
    if (flags.includes(name) && eq < 0) {
      out.flags.add(name);
      continue;
    }
    if (!options.includes(name)) throw usage(`\`${what}\` takes no \`--${name}\``);
    const value = eq < 0 ? args[++i] : a.slice(eq + 1);
    if (value === undefined) throw usage(`\`--${name}\` needs a value`);
    out.options.set(name, [...(out.options.get(name) ?? []), value]);
  }
  return out;
}

/** The positionals of `p`, which must number between `min` and `names.length`. */
function positionals(what: string, p: Parsed, names: string[], min = names.length): string[] {
  if (p.positionals.length < min) throw usage(`\`${what}\` needs <${names[p.positionals.length]}>`);
  if (p.positionals.length > names.length) throw usage(`\`${what}\` got an extra argument \`${p.positionals[names.length]}\``);
  return p.positionals;
}

/** The one value of option `name`, or `undefined`. */
function option(what: string, p: Parsed, name: string): string | undefined {
  const values = p.options.get(name);
  if (values !== undefined && values.length > 1) throw usage(`\`${what}\` takes \`--${name}\` once`);
  return values?.[0];
}

function required(what: string, p: Parsed, name: string): string {
  const value = option(what, p, name);
  if (value === undefined) throw usage(`\`${what}\` needs \`--${name}\``);
  return value;
}

/** `text` as an unsigned integer, as clap reads a `u32` or a `usize`. */
function number(name: string, text: string | undefined): number | undefined {
  if (text === undefined) return undefined;
  if (!/^\+?[0-9]+$/.test(text) || !Number.isSafeInteger(Number(text))) {
    throw usage(`invalid value '${text}' for '<${name}>': invalid digit found in string`);
  }
  return Number(text);
}

/** The goal `slug`, or the live goal of the checkout at `root`. */
function slugOrLive(root: string, dir: string, slug: string | undefined) {
  const chain = loadChain(dir);
  const s = slug ?? liveGoal(root, dir, chain);
  if (s === undefined) throw new Refused("the chain is empty, so no goal is live");
  return { chain, slug: s };
}

/** The goal's `extra` with stage `stage`'s, or the live stage's, laid over it. `{}` when neither has one. */
function extra(root: string, dir: string, slug: string | undefined, stage: number | undefined): string {
  const s = slugOrLive(root, dir, slug).slug;
  const g = loadGoal(dir, s);
  if (stage !== undefined && !g.front.stages.some((st) => st.id === stage)) throw new Refused(`${s} has no stage ${stage}`);
  return extraJson(g, stage ?? liveStage(g)?.id) ?? "{}";
}

/** What `show` reports: the goal's title, place on the chain, summary, stages and checks. */
function shown(root: string, dir: string, slug: string | undefined) {
  const { chain, slug: s } = slugOrLive(root, dir, slug);
  const g = loadGoal(dir, s);
  const live = liveStage(g)?.id;
  const at = chain.chain.indexOf(s);
  return {
    slug: s,
    title: g.front.title,
    position: at < 0 ? null : at + 1,
    summary: g.front.summary,
    stages: g.front.stages.map((st) => ({
      id: st.id,
      title: st.title,
      status: st.status === "done" ? "done" : st.id === live ? "live" : "open",
    })),
    checks: g.front.checks.map((c) => ({ id: c.id, stage: c.stage ?? null, name: c.name ?? null, run: c.run })),
  };
}

/** The text of `show`: stages marked `[x]` done, `[>]` live and `[ ]` open, then the checks. */
function shownText(s: ReturnType<typeof shown>): string {
  const place = s.position === 1 ? "live" : s.position === null ? "a side goal" : `position ${s.position} on the chain`;
  let out = `${s.slug}: ${s.title} (${place})\nReached when: ${s.summary}\n\nStages:\n`;
  for (const st of s.stages) {
    const mark = st.status === "done" ? "[x]" : st.status === "live" ? "[>]" : "[ ]";
    out += `  ${mark} ${st.id}. ${st.title}\n`;
  }
  if (s.checks.length > 0) out += "\nChecks:\n";
  for (const c of s.checks) {
    const stage = c.stage === null ? "" : ` (stage ${c.stage})`;
    const name = c.name === null ? "" : `: ${c.name}`;
    out += `  ${c.id}${stage}${name} → ${c.run.join(" ")}\n`;
  }
  return out;
}

function show(root: string, dir: string, args: string[]): Report {
  const p = parse("goal show", args, ["stage"], ["extra"]);
  const [slug] = positionals("goal show", p, ["slug"], 0);
  const stage = number("STAGE", option("goal show", p, "stage"));
  if (p.flags.has("extra")) {
    // The `extra` is a JSON object already, so `--json` prints it as it is.
    const text = extra(root, dir, slug, stage);
    return { doc: JSON.parse(text), text };
  }
  if (stage !== undefined) throw usage("`--stage` needs `--extra`");
  const s = shown(root, dir, slug);
  return { doc: s, text: shownText(s).trimEnd() };
}

/** The edit `args` name: the slug whose file it touches, and the change, which gives its line. */
function edit(root: string, dir: string, args: string[]): { slug: string; act: () => string } {
  const [verb, ...rest] = args;
  const sub = (name: string, words: string[]) => {
    const [w, ...more] = words;
    if (w === undefined) throw usage(`\`goal ${name}\` needs a subcommand`);
    return { w, more };
  };
  switch (verb) {
    case "new": {
      const p = parse("goal new", rest, ["title", "summary", "stage"], ["side"]);
      const [slug] = positionals("goal new", p, ["slug"]) as [string];
      const title = required("goal new", p, "title");
      const summary = required("goal new", p, "summary");
      const stage = option("goal new", p, "stage") ?? "The first stage";
      return { slug, act: () => newGoal(dir, slug, title, summary, stage, !p.flags.has("side")) };
    }
    case "move": {
      const p = parse("goal move", rest, [], []);
      const [slug, position] = positionals("goal move", p, ["slug", "position"]) as [string, string];
      const at = number("POSITION", position)!;
      return { slug, act: () => moveGoal(dir, slug, at) };
    }
    case "remove": {
      const p = parse("goal remove", rest, [], []);
      const [slug] = positionals("goal remove", p, ["slug"]) as [string];
      return { slug, act: () => removeGoal(dir, slug) };
    }
    case "stage": {
      const { w, more } = sub("stage", rest);
      if (w !== "done" && w !== "open") throw usage(`\`goal stage\` takes \`done\` or \`open\`, and got \`${w}\``);
      const p = parse(`goal stage ${w}`, more, [], []);
      const [slug, id] = positionals(`goal stage ${w}`, p, ["slug", "id"]) as [string, string];
      const n = number("ID", id)!;
      return { slug, act: () => setStage(dir, slug, n, w) };
    }
    case "check": {
      const { w, more } = sub("check", rest);
      if (w === "add") {
        const p = parse("goal check add", more, ["id", "stage", "name", "test"], []);
        const [slug] = positionals("goal check add", p, ["slug"]) as [string];
        if (p.rest === undefined || p.rest.length === 0) throw usage("`goal check add` needs the check's argv after `--`");
        const tests = p.options.get("test") ?? [];
        const check = {
          ...emptyCheck(),
          id: required("goal check add", p, "id"),
          stage: number("STAGE", option("goal check add", p, "stage")),
          name: option("goal check add", p, "name"),
          run: p.rest,
          tests: tests.length > 0 ? tests : undefined,
        };
        return { slug, act: () => addCheck(dir, slug, check) };
      }
      if (w === "remove") {
        const p = parse("goal check remove", more, [], []);
        const [slug, id] = positionals("goal check remove", p, ["slug", "id"]) as [string, string];
        return { slug, act: () => removeCheck(dir, slug, id) };
      }
      throw usage(`\`goal check\` takes \`add\` or \`remove\`, and got \`${w}\``);
    }
    case "context": {
      const { w, more } = sub("context", rest);
      if (w !== "add") throw usage(`\`goal context\` takes \`add\`, and got \`${w}\``);
      const p = parse("goal context add", more, ["stage"], []);
      const [slug, path] = positionals("goal context add", p, ["slug", "path"]) as [string, string];
      const stage = number("STAGE", option("goal context add", p, "stage"));
      return { slug, act: () => addContext(dir, slug, path, stage) };
    }
    case "handoff": {
      const { w, more } = sub("handoff", rest);
      if (w !== "set") throw usage(`\`goal handoff\` takes \`set\`, and got \`${w}\``);
      const p = parse("goal handoff set", more, ["file"], []);
      const [slug] = positionals("goal handoff set", p, ["slug"]) as [string];
      const file = required("goal handoff set", p, "file");
      let text: string;
      try {
        text = readFileSync(file === "-" ? 0 : resolve(root, file), "utf8");
      } catch (e) {
        throw new GoalError(file, undefined, (e as Error).message);
      }
      return { slug, act: () => setHandoff(dir, slug, text) };
    }
    default:
      throw usage(
        verb === undefined
          ? "`goal` needs a subcommand: show, new, move, remove, stage, check, context or handoff"
          : `\`goal\` has no subcommand \`${verb}\``,
      );
  }
}

/** The bytes of `path`, or `undefined` when it cannot be read. */
function bytes(path: string): Buffer | undefined {
  try {
    return readFileSync(path);
  } catch {
    return undefined;
  }
}

/** What `goal <args>` does on the project at `root`. */
export function goalReport(root: string, args: string[]): Report {
  const cfg = loadConfig(root);
  const dir = join(root, cfg.project.goals_dir);
  if (args[0] === "show") return show(root, dir, args.slice(1));
  const goals = cfg.project.goals_dir.replace(/\/+$/, "");
  const { slug, act } = edit(root, dir, args);
  const files = [`${goals}/${slug}.md`, `${goals}/${README}`];
  const before = files.map((f) => bytes(join(root, f)));
  const message = act().trimEnd();
  const changed = files.filter((f, i) => {
    const now = bytes(join(root, f));
    const was = before[i];
    return now === undefined || was === undefined ? now !== was : !now.equals(was);
  });
  return { doc: { message, changed }, text: message };
}

/** What `harvested <slug>` does on the project at `root`: exit 0 when the goal's findings are written. */
export function harvestedReport(root: string, args: string[]): Report {
  const p = parse("harvested", args, [], []);
  const [slug] = positionals("harvested", p, ["slug"]) as [string];
  const done = harvested(root, loadConfig(root), slug);
  return { code: done ? Exit.ok : Exit.refused, doc: { slug, harvested: done } };
}

export const harvestedCommand: Command = {
  name: "harvested",
  about: "Exits 0 when the harvest file holds a section with content for the goal.",
  json: true,
  run: (args) => harvestedReport(process.cwd(), args),
};

export const goal: Command = {
  name: "goal",
  about: "Shows a goal, or edits it: new, move, remove, stage, check, context and handoff set.",
  json: true,
  run: (args) => goalReport(process.cwd(), args),
};
