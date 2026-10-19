// `import --from <kind> [--dry-run]`, as `crates/looper/src/cmd/import.rs` prints it: it writes the goals
// of the project in the current directory in looper's format, from the format its own tooling kept, and
// prints each file it wrote. `sync/import.ts` does the work.
//
// - With `--dry-run` it writes nothing and prints each file it would write.
// - A refused import writes nothing and exits 1.
//
// Its `--json` document is `{"dry_run", "files", "notes", "todos", "changed"}`: each file is
// `{"path", "unchanged"}`, and `changed` lists each file written, empty in a dry run.

import { loadConfig } from "../goal/config.ts";
import { KINDS, type Kind, planImport, readImport, writeImport } from "../sync/import.ts";
import { type Command, usage } from "./cli.ts";

export const importCommand: Command = {
  name: "import",
  about: "Writes the project's goals, chain and floor in looper's format from the format its own tooling kept. `--dry-run` writes nothing.",
  json: true,
  run: (args) => {
    let from: string | undefined;
    let dryRun = false;
    for (let i = 0; i < args.length; i++) {
      const a = args[i] ?? "";
      if (a === "--dry-run") dryRun = true;
      else if (a === "--from") {
        from = args[++i];
        if (from === undefined) throw usage(`\`--from\` needs one of ${KINDS.join(", ")}`);
      } else if (a.startsWith("--from=")) from = a.slice("--from=".length);
      else throw usage(`\`import\` takes only \`--from <kind>\` and \`--dry-run\`, and got \`${a}\``);
    }
    if (from === undefined) throw usage(`\`import\` needs \`--from\`, one of ${KINDS.join(", ")}`);
    if (!(KINDS as readonly string[]).includes(from)) throw usage(`there is no import format ${from}: it is one of ${KINDS.join(", ")}`);
    const root = process.cwd();
    const cfg = loadConfig(root);
    const read = readImport(from as Kind, root, cfg);
    const { targets, todos } = dryRun ? planImport(root, cfg, read) : writeImport(root, cfg, read);
    const changed = dryRun ? [] : targets.filter((t) => !t.unchanged).map((t) => t.path);
    const verb = dryRun ? "would write" : "wrote";
    const lines = [
      ...targets.map((t) => (t.unchanged ? `unchanged: ${t.path}` : `${verb} ${t.path}`)),
      ...read.notes.map((n) => `note: ${n}`),
      ...todos.map((t) => `todo: ${t}`),
    ];
    if (todos.length > 0) {
      lines.push(
        "`looper lint` refuses every TODO check until it runs a real command (docs/upgrade-guide.md § What makes a good check)",
      );
    }
    return {
      doc: { dry_run: dryRun, files: targets, notes: read.notes, todos, changed },
      text: `${lines.join("\n")}\n`,
    };
  },
};
