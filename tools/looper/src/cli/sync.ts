// `sync [--check] [--from <kit>]`, as `crates/looper/src/cmd/sync.rs` prints it: it rewrites the
// managed files and blocks of the project in the current directory from the templates of the kit that
// runs, writes the kit's copy into `tools/looper/`, and prints the diff of each change.
//
// - `--from <kit>` names another kit's directory, such as looper's `kit/`. That kit's own sync then runs
//   on the project, with its templates and its code, and its output is this command's.
// - Without `--check` it writes every change through a temp file and a rename, the managed text first
//   and the copy's manifest last. Nothing outside a marker changes, and a seed file is never touched.
// - With `--check` it writes nothing and exits 1 when anything differs.
// - A file whose markers do not parse refuses the whole sync before anything is written.
// - Every project takes the copy, looper's own checkout too.
// - A root `loop` or `loop.cmd` that is not the shim an older sync wrote is left in place and named.
//
// Its `--json` document is `{"check", "drifted", "changed", "left"}`: each drift is `{"path", "new",
// "diff"}`, `changed` lists each file written or removed, and `left` each root shim left in place.

import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { type Config, loadConfig } from "../goal/config.ts";
import { type CopyChange, copyPlan, leftShims } from "../sync/copy.ts";
import { apply, diff, plan } from "../sync/sync.ts";
import { KIT_DIR } from "../sync/template.ts";
import { type Command, Exit, Refusal, type Report, usage } from "./cli.ts";

/**
 * Every change sync makes to the project at `root` from the kit at `kit`: the managed text, then the
 * copy.
 */
export function syncPlan(root: string, cfg: Config, kit: string): CopyChange[] {
  return [...plan(root, cfg), ...copyPlan(root, kit)];
}

/** What `sync` does to the project at `root` from the kit at `kit`, or with `check` only reports. */
export function syncReport(root: string, cfg: Config, kit: string, check: boolean): Report {
  const changes = syncPlan(root, cfg, kit);
  if (!check) apply(root, changes);
  const drifted = changes.map((c) => ({ path: c.path, new: c.before === undefined, diff: diff(c) }));
  const changed = check ? [] : changes.map((c) => c.path);
  const left = leftShims(root);
  let text = drifted.map((d) => d.diff).join("");
  const n = drifted.length;
  if (n === 0) text = "sync: every managed file is current\n";
  else if (check) text += `sync --check: ${n} managed file(s) differ from the templates. Run sync.\n`;
  else text += `sync: ${n} managed file(s) rewritten\n`;
  for (const path of left) {
    text += `sync: ${path} is left in place, since it is not the shim an older sync wrote. The shim is tools/${path} now.\n`;
  }
  return { code: check && n > 0 ? Exit.refused : Exit.ok, doc: { check, drifted, changed, left }, text };
}

/** Runs the sync of the kit at `kit` in the current directory, its output the terminal's. */
function syncFrom(kit: string, check: boolean, json: boolean): Report {
  const main = join(kit, "main.ts");
  if (!existsSync(main)) throw new Refusal(`${kit} holds no kit: it has no main.ts`);
  const argv = ["bun", main, ...(json ? ["--json"] : []), "sync", ...(check ? ["--check"] : [])];
  const ran = Bun.spawnSync(argv, { cwd: process.cwd(), stdin: "ignore", stdout: "inherit", stderr: "inherit" });
  return { code: ran.exitCode ?? Exit.refused, own: true };
}

export const syncCommand: Command = {
  name: "sync",
  about: "Rewrites the managed files and the kit's copy in tools/looper/. `--check` only compares, `--from <kit>` names the kit.",
  json: true,
  run: (args, ctx) => {
    let from: string | undefined;
    let check = false;
    for (let i = 0; i < args.length; i++) {
      const a = args[i] ?? "";
      if (a === "--check") check = true;
      else if (a === "--from") {
        from = args[++i];
        if (from === undefined) throw usage("`--from` needs the kit's directory");
      } else if (a.startsWith("--from=")) from = a.slice("--from=".length);
      else throw usage(`\`sync\` takes only \`--check\` and \`--from <kit>\`, and got \`${a}\``);
    }
    if (from !== undefined && resolve(from) !== resolve(KIT_DIR)) return syncFrom(resolve(from), check, ctx.json);
    const root = process.cwd();
    return syncReport(root, loadConfig(root), KIT_DIR, check);
  },
};
