// `links`, `layout [--check]`, `prose [--all]` and `hook commit-msg <file>`, as
// `crates/looper/src/cmd/gates.rs` and `crates/looper/src/cmd/hook.rs` print them. Each runs on the
// project in the current directory.
//
// - The three gates print their issues as `lint` does, and its `{"clean", "issues"}` document with
//   `--json`. `links` and `prose` exit 1 on any issue. `layout` prints what is wrong and exits 0, and
//   `layout --check` exits 1 on any issue.
// - A gate the project's `[gates]` names a command for runs that command from the root instead. Its
//   output reaches the terminal as it is, and its exit code is the gate's. It gets `--json` when the kit
//   does and `[gates] json` names the gate, and no other flag, so `--check` and `--all` leave it as it is.
// - `hook commit-msg` checks the message file git passes, with the `[commits]` of the current
//   directory's `looper.toml`. It names each problem on stderr and exits 1. It takes no `--json`.

import { readFileSync } from "node:fs";
import { checkEnv, type Config, gatesSet, loadConfig, takesJson } from "../goal/config.ts";
import type { Issue } from "../goal/lint.ts";
import { issueText } from "../goal/lint.ts";
import { check } from "../gates/commit-msg.ts";
import { layout, links, prose } from "../gates/gates.ts";
import { type Command, type Context, Exit, Refusal, type Report, usage } from "./cli.ts";

/** Runs the project's command for `gate` from `root`, with this process's stdio, or `undefined` when the project names none. */
export function projectGate(root: string, cfg: Config, gate: string, json: boolean): Report | undefined {
  const set = gatesSet(cfg.gates).find(([g]) => g === gate);
  if (set === undefined) return undefined;
  const argv = [...set[1]];
  if (json && takesJson(cfg.gates, gate)) argv.push("--json");
  const named = argv.join(" ");
  let ran: ReturnType<typeof Bun.spawnSync>;
  try {
    ran = Bun.spawnSync(argv, { cwd: root, env: checkEnv(cfg.checks), stdin: "ignore", stdout: "inherit", stderr: "inherit" });
  } catch (e) {
    throw new Refusal(`[gates] ${gate}: \`${named}\` cannot start: ${(e as Error).message}`);
  }
  return { code: ran.exitCode ?? Exit.refused, own: true };
}

/** The issues as `lint` prints them, with `clean` the text when there are none. Exits 1 on any issue when `fail`. */
export function findings(issues: Issue[], clean: string, fail: boolean): Report {
  const ok = issues.length === 0;
  const text = issues.map((i) => `✗ ${issueText(i)}`);
  if (ok) text.push(`✓ ${clean}`);
  return {
    code: fail && !ok ? Exit.refused : Exit.ok,
    doc: { clean: ok, issues: issues.map((i) => ({ file: i.file, line: i.line ?? null, message: i.message })) },
    text: `${text.join("\n")}\n`,
  };
}

/** The one flag `name` takes, or a usage refusal for any other argument. */
function flag(name: string, args: string[], allowed: string | undefined): boolean {
  const extra = args.find((a) => a !== allowed);
  if (extra !== undefined) {
    throw usage(allowed === undefined ? `\`${name}\` takes no argument, and got \`${extra}\`` : `\`${name}\` takes only \`${allowed}\`, and got \`${extra}\``);
  }
  return allowed !== undefined && args.includes(allowed);
}

export const linksCommand: Command = {
  name: "links",
  about: "Reports every dead, miscased or unanchored link in the Markdown files.",
  json: true,
  run: (args, ctx) => {
    flag("links", args, undefined);
    const root = process.cwd();
    const cfg = loadConfig(root);
    return projectGate(root, cfg, "links", ctx.json) ?? findings(links(root, cfg),"every link resolves", true);
  },
};

export const layoutCommand: Command = {
  name: "layout",
  about: "Compares the README's layout table with the tree. `--check` exits 1 on a difference.",
  json: true,
  run: (args, ctx) => {
    const strict = flag("layout", args, "--check");
    const root = process.cwd();
    const cfg = loadConfig(root);
    return projectGate(root, cfg, "layout", ctx.json) ?? findings(layout(root), "the layout table and the tree agree", strict);
  },
};

export const proseCommand: Command = {
  name: "prose",
  about: "Checks the shape of the touched Markdown prose. `--all` reads every file.",
  json: true,
  run: (args, ctx) => {
    const all = flag("prose", args, "--all");
    const root = process.cwd();
    const cfg = loadConfig(root);
    const scope = all ? "every Markdown file" : "the touched Markdown files";
    return projectGate(root, cfg, "prose", ctx.json) ?? findings(prose(root, cfg, all), `the prose of ${scope} is in shape`, true);
  },
};

/** `hook commit-msg <file>` on the project at `root`. */
function commitMsg(root: string, file: string, ctx: Context): Report {
  let text: string;
  try {
    text = readFileSync(file, "utf8");
  } catch (e) {
    ctx.err(`commit-msg: looper cannot read ${file}: ${(e as Error).message}. The commit is refused.`);
    return { code: Exit.refused };
  }
  let cfg: Config;
  try {
    cfg = loadConfig(root);
  } catch (e) {
    ctx.err(`commit-msg: ${(e as Error).message}. The commit is refused.`);
    return { code: Exit.refused };
  }
  const problems = check(text, cfg.commits);
  if (problems.length === 0) return {};
  ctx.err("commit-msg: the message breaks the shape of docs/agent/conventions.md § A commit message:");
  for (const p of problems) ctx.err(`  ${p}`);
  ctx.err("Fix the message and commit again. Never pass --no-verify.");
  return { code: Exit.refused };
}

export const hookCommand: Command = {
  name: "hook",
  about: "The checks a project's git hooks call: `hook commit-msg <file>`.",
  json: false,
  run: (args, ctx) => {
    const [name, file, ...rest] = args;
    if (name !== "commit-msg") throw usage(`\`hook\` takes \`commit-msg <file>\`, and got \`${name ?? ""}\``);
    if (file === undefined || rest.length > 0) throw usage("`hook commit-msg` takes one message file");
    return commitMsg(process.cwd(), file, ctx);
  },
};
