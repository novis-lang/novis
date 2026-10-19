// `launch`: the launchers of the project in the current directory, the `[[launch]]` tables of its
// `looper.toml`, started, watched and stopped from their job folders (`../jobs/launch.ts`), as
// `crates/looper/src/cmd/launch.rs` does. Every subcommand works without looper's service.
//
// - `launch list` prints each launcher, one line each: its name, its state, its port, its job and its
//   title. A launcher with no live launch is `idle`.
// - `launch start <name>` refuses a launcher that runs already, and names its job. Otherwise it starts
//   the supervisor (`../jobs/supervise.ts`) and prints the job it names. `--port` replaces the
//   launcher's port, and `--args` gives the words of a launcher with `args = true`. Inside a job that
//   lets no process leave, such as a Bash call's, it says on stderr that the launch ends with that job.
// - `launch stop <name>` asks the launch to stop and waits for its exit.
// - `launch restart <name>` is a stop, when the launcher runs, and then a start.
// - `launch supervise <name>` is the supervisor, which `start` spawns. It has no stdio, so it writes
//   what went wrong to the launcher's `supervisor.log`.
//
// With `--json`, `list` prints `{"launchers"}`, `start` `{"name", "job"}` and `stop` `{"name", "job",
// "code"}`. A refusal exits 1.
//
// # The bounds it keeps
//
// - An `--args` text is at most `ARGS_MAX` bytes.

import { appendFileSync } from "node:fs";
import { join } from "node:path";
import { type Config, loadConfig } from "../goal/config.ts";
import { launchDir, liveLaunches, type Live, NotStarted, type Request, startLaunch, stopLaunch, SUPERVISOR_LOG } from "../jobs/launch.ts";
import { ARGS_MAX } from "../jobs/ports.ts";
import { launcher, runsAlready, supervise } from "../jobs/supervise.ts";
import { storeOf } from "../jobs/store.ts";
import { asker } from "./claims.ts";
import { type Command, Refusal, type Report, usage } from "./cli.ts";

/** The project's config, or the refusal that names what is wrong with it. */
function config(root: string): Config {
  try {
    return loadConfig(root);
  } catch (e) {
    throw new Refusal((e as Error).message);
  }
}

/** The launcher `name` of `cfg`, or a refusal. */
function named(cfg: Config, name: string) {
  const l = launcher(cfg, name);
  if (typeof l === "string") throw new Refusal(l);
  return l;
}

/** The live launch of launcher `name` in the project at `root`. */
function running(root: string, name: string): Live | undefined {
  return liveLaunches(storeOf(root)).find((l) => l.launch.name === name);
}

/** The request in `args`: the name, then `--port <n>` and `--args <text>` or `--args=<text>`. */
function request(args: string[], action: string): Request {
  const req: Partial<Request> = {};
  for (let i = 0; i < args.length; i++) {
    const a = args[i] as string;
    if (a === "--port") {
      const value = args[++i];
      if (value === undefined || !/^\d+$/.test(value) || Number(value) < 1 || Number(value) > 65_535) throw usage("`--port` takes a port from 1 to 65535");
      req.port = Number(value);
    } else if (a === "--args" || a.startsWith("--args=")) {
      const value = a === "--args" ? args[++i] : a.slice("--args=".length);
      if (value === undefined) throw usage("`--args` takes a text");
      req.args = value;
    } else if (a.startsWith("--") || req.name !== undefined) throw usage(`\`launch ${action}\` does not take \`${a}\``);
    else req.name = a;
  }
  if (req.name === undefined) throw usage(`\`launch ${action}\` takes a launcher's name`);
  return req as Request;
}

function list(root: string): Report {
  const cfg = config(root);
  const live = liveLaunches(storeOf(root));
  const launchers = cfg.launch.map((l) => {
    const x = live.find((v) => v.launch.name === l.name);
    return {
      name: l.name,
      title: l.title,
      about: l.about,
      port: l.port ?? null,
      args: l.args,
      live: x ? { job: x.id, phase: x.launch.phase, port: x.launch.port, ready: x.launch.ready, hidden: x.launch.hidden } : null,
    };
  });
  const lines = launchers.map((l) => {
    const [state, port, job] = l.live ? [l.live.phase, l.live.port, `job ${l.live.job}`] : ["idle", l.port, ""];
    return `${l.name.padEnd(20)} ${state.padEnd(9)} ${(port === null ? "" : `port ${port}`).padEnd(10)} ${job.padEnd(22)} ${l.title}`;
  });
  if (lines.length === 0) lines.push("looper.toml declares no launchers");
  return { doc: { launchers }, text: lines.join("\n") };
}

async function start(root: string, req: Request, err: (text: string) => void): Promise<Report> {
  const cfg = config(root);
  const l = named(cfg, req.name);
  const live = running(root, l.name);
  if (live) throw new Refusal(runsAlready(l.name, live.id));
  if (req.args !== undefined && Buffer.byteLength(req.args) > ARGS_MAX) throw new Refusal(`the --args text is longer than ${ARGS_MAX} bytes`);
  let started: { id: string; bound: boolean };
  try {
    started = await startLaunch(root, req);
  } catch (e) {
    throw new Refusal(e instanceof NotStarted ? e.message : String(e));
  }
  if (started.bound) err(`looper: ${l.name} runs in this job, which lets no process leave, and ends with it`);
  return {
    doc: { name: l.name, job: started.id },
    text: `started ${l.name} as launch job ${started.id}; \`exec log ${started.id}\` prints its output`,
  };
}

async function stop(root: string, name: string): Promise<Report> {
  const l = named(config(root), name);
  const live = running(root, name);
  if (!live) throw new Refusal(`\`${name}\` is not running`);
  const exit = await stopLaunch(storeOf(root), live, l.stop_grace * 1000, asker());
  const code = exit?.code ?? null;
  return {
    doc: { name, job: live.id, code },
    text: `stopped ${name}: launch job ${live.id} ${code === null ? "is lost" : `exited with ${code}`}`,
  };
}

/** The supervisor, which has no stdio: whatever stops it is written to the launcher's log. */
async function supervisor(root: string, req: Request): Promise<Report> {
  try {
    return { code: await supervise(root, config(root), req), own: true };
  } catch (e) {
    try {
      appendFileSync(join(launchDir(root, req.name), SUPERVISOR_LOG), `error: ${(e as Error).message}\n`);
    } catch {}
    return { code: 1, own: true };
  }
}

export const launchCommand: Command = {
  name: "launch",
  about: "Lists, starts, stops and restarts this project's launchers: `launch list|start|stop|restart`.",
  json: true,
  run: async (args, ctx) => {
    const [action, ...rest] = args;
    const root = process.cwd();
    switch (action) {
      case "list":
        if (rest.length > 0) throw usage(`\`launch list\` does not take \`${rest[0]}\``);
        return list(root);
      case "start":
        return start(root, request(rest, "start"), ctx.err);
      case "stop": {
        const req = request(rest, "stop");
        if (req.port !== undefined || req.args !== undefined) throw usage("`launch stop` takes only a launcher's name");
        return stop(root, req.name);
      }
      case "restart": {
        const req = request(rest, "restart");
        const stopped = running(root, req.name) ? await stop(root, req.name) : null;
        const started = await start(root, req, ctx.err);
        return { doc: { ...(started.doc as object), stopped: stopped?.doc ?? null }, text: [stopped?.text, started.text].filter((t) => t !== undefined).join("\n") };
      }
      case "supervise":
        return supervisor(root, request(rest, "supervise"));
      default: {
        const takes = "`launch` takes `list`, `start`, `stop` or `restart`";
        throw usage(action === undefined ? takes : `${takes}, and got \`${action}\``);
      }
    }
  },
};
