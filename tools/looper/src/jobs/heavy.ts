// Which Bash calls take a heavy-job slot, judged from the command's text alone.
//
// - The command splits into simple commands at `;`, `&`, `|`, `(`, `)`, `{`, `}` and newlines, and
//   each loses its leading `NAME=value` words. Quotes are not parsed, so a separator inside one splits
//   too, which can only make a call take a slot it did not need.
// - A pattern is a command's first words, such as `cargo test`. It matches a simple command whose
//   words start with its words.
// - A call with a simple command of `OWN_SLOTS` takes none, because their units take their own.
//   Otherwise a call that matches `BUILT_IN` or `[jobs] heavy` takes one.
// - A call never takes more than one slot (decision 0029). A wait for every slot never ends while a
//   job of another project holds one.
// - looper's own commands count in both forms: `looper <command>`, and the kit's
//   `bun tools/looper/main.ts <command>`.

import type { Jobs } from "../goal/config.ts";

/** The commands that take a slot in every project. */
export const BUILT_IN = ["cargo build", "cargo test", "cargo run", "cargo clippy", "cargo bench", "cargo nextest", "bun test"];

/** How a project runs its copy of the kit. */
const KIT = "bun tools/looper/main.ts";

/** The commands whose units take slots of their own. */
const OWN_SLOTS = ["looper verify", "looper check", `${KIT} verify`, `${KIT} check`];

/** The slots a call takes: none, or one. */
export type Take = "none" | "one";

/** The slots `command` takes, with the project's `[jobs]`. */
export function take(command: string, jobs: Pick<Jobs, "heavy">): Take {
  const commands = command.split(/[;&|(){}\n]/).map((simple) => {
    const words = simple.split(/\s+/).filter((w) => w !== "");
    let at = 0;
    while (at < words.length && isAssignment(words[at] as string)) at++;
    return words.slice(at);
  });
  if (matches(commands, OWN_SLOTS)) return "none";
  if (matches(commands, [...BUILT_IN, ...jobs.heavy])) return "one";
  return "none";
}

/** Whether any of `commands`, each as its words, starts with the words of any of `patterns`. */
function matches(commands: string[][], patterns: readonly string[]): boolean {
  return patterns.some((p) => {
    const want = p.split(/\s+/).filter((w) => w !== "");
    return want.length > 0 && commands.some((words) => want.every((w, i) => words[i] === w));
  });
}

/** Whether `word` is `NAME=value` with a shell variable's name. */
function isAssignment(word: string): boolean {
  const eq = word.indexOf("=");
  return eq > 0 && /^[A-Za-z_][A-Za-z0-9_]*$/.test(word.slice(0, eq));
}
