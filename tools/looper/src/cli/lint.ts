// `lint` and `render [--check]`, as `crates/looper/src/cmd/lint.rs` prints them. Each runs on the
// project in the current directory.
//
// - `lint` prints each issue as `✗ <file>:<line>: <message>`, each warning as `! …`, and `✓ …` when
//   there is no issue. It exits 1 on any issue. Its `--json` document is
//   `{"clean", "issues", "warnings"?}`, with `warnings` left out when there are none.
// - `render` rewrites the README when it differs. With `--check` it writes nothing, and exits 1 with a
//   line on stderr when the README is stale. Its document is `{"readme", "changed", "current"?}`.

import { join } from "node:path";
import { type Config, loadConfig } from "../goal/config.ts";
import { render } from "../goal/edit.ts";
import { type Issue, issueText, launchWarnings, lintAll, readmeStale } from "../goal/lint.ts";
import { loadChain, README } from "../goal/model.ts";
import { type Command, type Context, Exit, type Report, usage } from "./cli.ts";

/** An issue as looper's JSON has it, with `line` null when there is none. */
function issueDoc(i: Issue) {
  return { file: i.file, line: i.line ?? null, message: i.message };
}

/** What `lint` reports on the project at `root`. */
export function lintReport(root: string, cfg: Config): Report {
  const issues = lintAll(root, cfg);
  const warnings = launchWarnings(cfg);
  const clean = issues.length === 0;
  const doc: Record<string, unknown> = { clean, issues: issues.map(issueDoc) };
  if (warnings.length > 0) doc.warnings = warnings.map(issueDoc);
  const text = [...issues.map((i) => `✗ ${issueText(i)}`), ...warnings.map((w) => `! ${issueText(w)}`)];
  if (clean) text.push("✓ goals, chain and README are clean");
  return { code: clean ? Exit.ok : Exit.refused, doc, text: `${text.join("\n")}\n` };
}

/** What `render` does to the README of the project at `root`, or with `check` only reports. */
export function renderReport(root: string, cfg: Config, check: boolean, ctx: Context): Report {
  const dir = join(root, cfg.project.goals_dir);
  const readme = `${cfg.project.goals_dir.replace(/\/+$/, "")}/${README}`;
  if (!check) {
    const wrote = render(dir);
    return {
      doc: { readme, changed: wrote ? [readme] : [] },
      text: `${wrote ? "wrote" : "unchanged:"} ${readme}`,
    };
  }
  const current = !readmeStale(dir, loadChain(dir));
  if (!current) ctx.err(`${readme} is stale: run \`looper render\``);
  return {
    code: current ? Exit.ok : Exit.refused,
    doc: { readme, changed: [], current },
    text: current ? `✓ ${readme} is current` : undefined,
  };
}

export const lint: Command = {
  name: "lint",
  about: "Reports every problem in the goals, the chain, the playbook and the meta layer.",
  json: true,
  run: (args) => {
    if (args.length > 0) throw usage(`\`lint\` takes no argument, and got \`${args[0]}\``);
    const root = process.cwd();
    return lintReport(root, loadConfig(root));
  },
};

export const renderCommand: Command = {
  name: "render",
  about: "Rewrites the goals README from the chain and the goal files. `--check` only compares.",
  json: true,
  run: (args, ctx) => {
    const extra = args.find((a) => a !== "--check");
    if (extra !== undefined) throw usage(`\`render\` takes only \`--check\`, and got \`${extra}\``);
    const root = process.cwd();
    return renderReport(root, loadConfig(root), args.includes("--check"), ctx);
  },
};
