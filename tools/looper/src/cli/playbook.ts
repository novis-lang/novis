// `playbook retire [--check] [--commit]`: deletes every playbook bullet whose `until` condition holds,
// as `retire` in `../goal/playbook.ts` does, and prints each one, as `crates/looper/src/cmd/playbook.rs`
// does. It runs on the project in the current directory.
//
// - `--check` changes nothing, prints what would go, and exits 1 when anything would.
// - `--commit` commits only the tracked playbook paths it changed, with `SUBJECT`. A bullet file git
//   never held is deleted and has nothing to commit.
// - Its `--json` document is `{"bullets", "changed", "commits"}`. Each bullet is `{"file", "line",
//   "text"}`, and each commit `{"sha", "subject", "paths"}`.
//
// The kit never retires a bullet on its own. A project runs this where it wants, such as a wrap check.

import { loadConfig } from "../goal/config.ts";
import { Refused } from "../goal/model.ts";
import { loadPlaybook, retirable, retire } from "../goal/playbook.ts";
import { type Command, type Context, Exit, type Report, usage } from "./cli.ts";

/** The subject of the commit `--commit` makes. */
export const SUBJECT = "docs(playbook): retire the bullets whose condition holds";

/** A commit the command made: its short hash, its subject and the paths it holds. */
interface Commit {
  sha: string;
  subject: string;
  paths: string[];
}

/** What `playbook <args>` does on the project at `root`. */
export function playbookReport(root: string, args: string[], ctx: Context): Report {
  const [verb, ...rest] = args;
  if (verb !== "retire") {
    throw usage(verb === undefined ? "`playbook` needs a subcommand: retire" : `\`playbook\` has no subcommand \`${verb}\``);
  }
  const extra = rest.find((a) => a !== "--check" && a !== "--commit");
  if (extra !== undefined) throw usage(`\`playbook retire\` takes no \`${extra}\``);
  const check = rest.includes("--check");
  const commit = rest.includes("--commit");
  if (check && commit) throw usage("`--commit` cannot be used with `--check`");

  const cfg = loadConfig(root);
  const book = loadPlaybook(root, cfg);
  if (book === undefined) throw new Refused(`${cfg.project.playbook} does not exist`);
  const gone = retirable(book, root, cfg);
  // The text goes out line by line as it runs, so a refusal still follows the bullets it printed.
  const say = (text: string) => {
    if (!ctx.json) ctx.line(text);
  };
  if (gone.length === 0) say("no bullet's condition holds");
  const said = check ? "would retire" : "retired";
  for (const b of gone) say(`${said} ${b.file}:${b.line}: ${b.text.split("\n")[0]}`);
  let changed: string[] = [];
  const commits: Commit[] = [];
  if (gone.length > 0 && !check) {
    changed = retire(root, cfg, gone);
    if (commit) {
      const made = commitPaths(root, changed, say);
      if (made !== undefined) commits.push(made);
    }
  }
  const bullets = gone.map((b) => ({ file: b.file, line: b.line, text: b.text }));
  return { code: check && gone.length > 0 ? Exit.refused : Exit.ok, doc: { bullets, changed, commits } };
}

/**
 * Commits the tracked ones of `changed`, and only them, and gives the commit. On a failed commit the
 * index is put back. It tells `say` what it did.
 */
function commitPaths(root: string, changed: string[], say: (text: string) => void): Commit | undefined {
  const tracked = git(root, ["ls-files", "-z", "--"], changed).split("\0");
  const paths = changed.filter((p) => tracked.includes(p));
  if (paths.length === 0) {
    say("nothing to commit: git held none of the paths");
    return undefined;
  }
  git(root, ["add", "-A", "--"], paths);
  try {
    git(root, ["commit", "-q", "-m", SUBJECT, "--only", "--"], paths);
  } catch (e) {
    try {
      git(root, ["reset", "-q", "--"], paths);
    } catch {
      // The commit's own failure is the one to report.
    }
    throw e;
  }
  say(`committed ${paths.join(" ")}: ${SUBJECT}`);
  const sha = git(root, ["rev-parse", "--short", "HEAD"], []).trim();
  return { sha, subject: SUBJECT, paths };
}

/** Runs git in `root` and gives its stdout, or refuses with its stderr. */
function git(root: string, args: string[], paths: string[]): string {
  let out: { exitCode: number; stdout: Buffer; stderr: Buffer };
  try {
    out = Bun.spawnSync(["git", ...args, ...paths], { cwd: root, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  } catch (e) {
    throw new Refused(`git would not start: ${(e as Error).message}`);
  }
  if (out.exitCode !== 0) throw new Refused(`git ${args.join(" ")}: ${out.stderr.toString().trim()}`);
  return out.stdout.toString();
}

export const playbook: Command = {
  name: "playbook",
  about: "Retires the playbook bullets whose condition holds: playbook retire [--check] [--commit].",
  json: true,
  run: (args, ctx) => playbookReport(process.cwd(), args, ctx),
};
