// `orient [--full]`: prints the orientation pack an interactive session starts from, as
// `crates/looper/src/cmd/orient.rs` does. With `--json` it prints the `Pack` of `../goal/orient.ts`.
//
// - The project's sections come from `[orient] extra`, as `crates/looper-engine/src/pack.rs` runs them.
//   Each command runs in order from the project root, with `[checks] env` and the goal's variables:
//   `LOOPER_GOAL`, `LOOPER_STAGE`, `LOOPER_EXTRA`, and `LOOPER_FULL=1` under `--full`.
// - A command that cannot start, fails or times out yields one line that names it and why, and the next
//   command runs. Its stderr is dropped, and its stdout is printed only when it exits 0.
// - Each command's stdout is kept up to `EXTRA_MAX` bytes, and the rest is read and dropped, with one
//   line naming the cut. Each runs for at most `EXTRA_TIMEOUT_S`, then its whole tree is killed.

import { join, resolve } from "node:path";
import { type Config, loadConfig } from "../goal/config.ts";
import { extraJson, liveGoal, liveStage, loadChain, loadGoal } from "../goal/model.ts";
import { loadPack, packText } from "../goal/orient.ts";
import { killTree } from "../os/index.ts";
import { type Command, type Report, usage } from "./cli.ts";

/** The bytes of one command's stdout the pack keeps. */
export const EXTRA_MAX = 64 * 1024;
/** The seconds one command may run. */
export const EXTRA_TIMEOUT_S = 60;

/** The variables a project command gets of the live goal. An `undefined` one is removed from its environment. */
export function goalVars(root: string, goalsDir: string): Record<string, string | undefined> {
  let slug: string | undefined;
  let stage: number | undefined;
  let extra: string | undefined;
  try {
    slug = liveGoal(root, goalsDir, loadChain(goalsDir));
    if (slug !== undefined) {
      const goal = loadGoal(goalsDir, slug);
      stage = liveStage(goal)?.id;
      extra = extraJson(goal, stage);
    }
  } catch {
    slug = undefined;
  }
  return { LOOPER_GOAL: slug, LOOPER_STAGE: stage?.toString(), LOOPER_EXTRA: extra, LOOPER_AGENT: undefined };
}

/**
 * The output of each `[orient] extra` command, run from `root`. `over`, the loop's variables, is laid
 * over the goal's. `stage`, when set, replaces the goal's live stage as `LOOPER_STAGE`, and `full` sets
 * `LOOPER_FULL=1`.
 */
export async function extra(
  root: string,
  cfg: Config,
  stage: number | undefined,
  full: boolean,
  over: Record<string, string | undefined> = {},
): Promise<string[]> {
  const vars = { ...goalVars(root, join(root, cfg.project.goals_dir)), ...over };
  if (stage !== undefined) vars.LOOPER_STAGE = String(stage);
  vars.LOOPER_FULL = full ? "1" : undefined;
  const env: Record<string, string | undefined> = { ...process.env, ...cfg.checks.env };
  for (const [k, v] of Object.entries(vars)) {
    for (const name of Object.keys(env).filter((n) => n.toUpperCase() === k)) delete env[name];
    if (v !== undefined) env[k] = v;
  }
  if (!cfg.checks.hide_windows) env.LOOPER_WINDOWS = "show";
  const out: string[] = [];
  for (const argv of cfg.orient.extra) {
    const text = await section(root, argv, env, cfg.checks.hide_windows).catch((e: Error) => {
      return { failed: e.message };
    });
    if (typeof text === "string") {
      if (text !== "") out.push(text);
    } else {
      out.push(`(the pack section \`${argv.join(" ")}\` ${text.failed}, so it prints nothing)`);
    }
  }
  return out;
}

/** One command's stdout, LF-ended lines without the last line break. It throws why it failed. */
async function section(root: string, argv: string[], env: Record<string, string | undefined>, hide: boolean): Promise<string> {
  if (argv.length === 0) throw new Error("cannot run: the argv is empty");
  let child: Bun.Subprocess<"ignore", "pipe", "ignore">;
  try {
    child = Bun.spawn(argv, { cwd: root, env, stdin: "ignore", stdout: "pipe", stderr: "ignore", windowsHide: hide });
  } catch (e) {
    throw new Error(`cannot run: ${(e as Error).message}`);
  }
  let timer: Timer | undefined;
  const timeout = new Promise<"timeout">((done) => {
    timer = setTimeout(() => done("timeout"), EXTRA_TIMEOUT_S * 1000);
  });
  const ran = await Promise.race([Promise.all([readBounded(child.stdout), child.exited]), timeout]);
  clearTimeout(timer);
  if (ran === "timeout") {
    killTree(child.pid);
    child.kill();
    await child.exited;
    throw new Error(`timed out after ${EXTRA_TIMEOUT_S}s`);
  }
  const [[kept, dropped], code] = ran;
  if (code === null) throw new Error("exited without a code, want 0");
  if (code !== 0) throw new Error(`exited ${code}, want 0`);
  let text = Buffer.from(kept).toString("utf8").replaceAll("\r\n", "\n").replace(/\n+$/, "");
  if (dropped > 0) text += `\n[the output was cut at ${EXTRA_MAX} bytes; ${dropped} more bytes dropped]`;
  return text;
}

/** Reads `stream` to its end, keeping its first `EXTRA_MAX` bytes, and counts the bytes past them. */
async function readBounded(stream: ReadableStream<Uint8Array>): Promise<[Uint8Array, number]> {
  const kept = new Uint8Array(EXTRA_MAX);
  let used = 0;
  let dropped = 0;
  try {
    for await (const chunk of stream) {
      const into = Math.min(EXTRA_MAX - used, chunk.length);
      kept.set(chunk.subarray(0, into), used);
      used += into;
      dropped += chunk.length - into;
    }
  } catch {
    // A read that fails ends the output, as looper's does.
  }
  return [kept.subarray(0, used), dropped];
}

/** What `orient <args>` does on the project at `root`. */
export async function orientReport(root: string, args: string[]): Promise<Report> {
  const full = args.includes("--full");
  const wrong = args.find((a) => a !== "--full");
  if (wrong !== undefined) throw usage(`\`orient\` takes only --full and --json, not \`${wrong}\``);
  const at = resolve(root);
  const cfg = loadConfig(at);
  const pack = loadPack(at, cfg, full, undefined, await extra(at, cfg, undefined, full));
  return { doc: pack, text: packText(pack) };
}

export const orient: Command = {
  name: "orient",
  about: "Prints the orientation pack for the live goal, every stage of it with --full.",
  json: true,
  run: (args) => orientReport(process.cwd(), args),
};
