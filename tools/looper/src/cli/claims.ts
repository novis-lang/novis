// `claims` and `claims release <session>`, as `crates/looper/src/cmd/claims.rs` prints them. Each runs
// on the project in the current directory, and reads its claim files under `.looper/claims/`
// (`../claims/claims.ts`).
//
// - `claims` prints each session that holds claims, with its loop or `interactive`, its goal and
//   whether it runs, and under it each file it claims with the claim's age. A claim whose file is
//   committed is released first, and not printed. With `--json` it prints `{"claims": [...]}`, one
//   `Listed` per claim.
// - `claims release <session>` frees that session's claims and records who asked (`asker`). An `agent`
//   may release only a dead session's claims, and is refused with exit 1 for a live one. With `--json`
//   it prints the `Released` record.
//
// # The bounds it keeps
//
// - The list holds one entry per claim, which `MAX_CLAIMS` per session and the sweep bound.

import { age, dir, type Listed, listing, machine, releaseAsked } from "../claims/claims.ts";
import { type Command, Refusal, usage } from "./cli.ts";

/**
 * Who asks: the `LOOPER_ASKER` a supervisor such as looper's service names, `agent` when `LOOPER_AGENT`
 * is set, or `cli:<the user's name>`.
 */
export function asker(): string {
  if (process.env.LOOPER_ASKER) return process.env.LOOPER_ASKER;
  if (process.env.LOOPER_AGENT) return "agent";
  return `cli:${process.env.USERNAME ?? "user"}`;
}

/** `listed` as of `now`: each session once, then its files. */
function text(listed: Listed[], now: number): string {
  if (listed.length === 0) return "No claims.";
  const lines: string[] = [];
  let session: string | undefined;
  for (const c of listed) {
    if (session !== c.session_id) {
      session = c.session_id;
      const where = c.loop === null ? "interactive" : `loop ${c.loop}`;
      lines.push(`${c.session_id}  ${where}  goal ${c.goal ?? "unknown"}  ${c.live ? "live" : "ended"}`);
    }
    const taken = c.taken_from === null ? "" : `  taken over from ${c.taken_from}`;
    lines.push(`  ${c.path}  ${age(now - c.at)}${taken}`);
  }
  return lines.join("\n");
}

export const claimsCommand: Command = {
  name: "claims",
  about: "Lists the edit claims of this project's sessions. `claims release <session>` frees one's.",
  json: true,
  run: (args) => {
    const claims = dir(process.cwd());
    const now = Date.now();
    const [action, session, ...rest] = args;
    if (action === undefined) {
      let listed: Listed[];
      try {
        listed = listing(claims, machine, now);
      } catch (e) {
        throw new Refusal(`cannot read the claims in ${claims}: ${(e as Error).message}`);
      }
      return { doc: { claims: listed }, text: text(listed, now) };
    }
    if (action !== "release") throw usage(`\`claims\` takes \`release <session>\` or nothing, and got \`${action}\``);
    if (session === undefined || rest.length > 0) throw usage("`claims release` takes one session id");
    let record: ReturnType<typeof releaseAsked>;
    try {
      record = releaseAsked(claims, session, asker(), machine, now);
    } catch (e) {
      throw new Refusal(`cannot release the claims of session ${session}: ${(e as Error).message}`);
    }
    if (typeof record === "string") throw new Refusal(record);
    return { doc: record, text: `Released ${record.paths.length} claim(s) of session ${session}, as ${record.by}.` };
  },
};
