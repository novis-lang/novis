// `guard`: Claude Code's `PreToolUse` hook, run as `bun tools/looper/main.ts guard`. It reads the hook's
// JSON on stdin, up to `MAX_INPUT` bytes, and `judge` in `src/guard/guard.ts` gives the verdict. A
// refusal exits 2 and a call the guard could not judge exits 1, each with its line on stderr. A pass
// exits 0 and prints nothing. It takes no `--json`.

import { judge, MAX_INPUT, outcome } from "../guard/guard.ts";
import { type Command, Exit, usage } from "./cli.ts";

/** Stdin up to `cap` bytes. A longer input is cut there, so it does not parse. */
async function stdin(cap: number): Promise<string> {
  const parts: Uint8Array[] = [];
  let size = 0;
  for await (const chunk of Bun.stdin.stream()) {
    const take = chunk.subarray(0, cap - size);
    parts.push(take);
    size += take.length;
    if (size >= cap) break;
  }
  return Buffer.concat(parts).toString();
}

export const guardCommand: Command = {
  name: "guard",
  about: "Judges a PreToolUse hook's call on stdin: exit 2 refuses it with the reason on stderr.",
  json: false,
  run: async (args, ctx) => {
    if (args.length > 0) throw usage(`\`guard\` takes no argument, and got \`${args[0]}\``);
    let input: string;
    try {
      input = await stdin(MAX_INPUT);
    } catch (e) {
      ctx.err(`looper guard: cannot read the hook's input: ${(e as Error).message}`);
      return { code: Exit.refused, own: true };
    }
    const { code, said } = outcome(await judge(input));
    if (said !== "") ctx.err(said);
    return { code, own: true };
  },
};
