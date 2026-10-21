// `bun nv services --up`: a goal's Docker services, up and healthy, before anything runs against them.
//
//     bun nv services --up [--goal <slug>]
//
// A goal's services are its record's `env.docker`: the compose file and the services its checks reach.
// The goal is `--goal`'s, else the one `LOOPER_GOAL` names, else the current one (`lib/chain.ts`
// `namedPlan`). `preflight` checks for a reachable Docker daemon, and `bringUp` then starts the services
// with `--wait`, so "up" means healthy. The driver calls both at the start of every turn, before a
// session or a sweep is spent against checks that cannot pass.
//
// When the daemon does not answer, `preflight` starts Docker Desktop itself, detached, and waits for it.
// That start is the one process left outside every job: on Windows each `Bun.spawn` child is in a job that
// kills all its members when the spawning process exits, and a session's whole tree is in its turn's, so a
// Docker Desktop a session started dies with that turn and the next sweep finds nothing listening.
//
// A goal with no `env.docker` needs nothing, and exits 0. Otherwise it exits 0 when the services are up,
// 1 when the daemon or a service is not, and 2 on a usage error.

import { spawn } from "node:child_process";
import { type Goal, namedPlan } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { failLine, run as runProcess, type RunOptions, type RunResult } from "../lib/proc.ts";

export const summary = "bring up a goal's docker services, healthy: nv services --up [--goal <slug>]";

const USAGE = "usage: bun nv services --up [--goal <slug>]";

/** What a service step printed, one line to the caller. */
type Say = (line: string) => void;

/** A goal record's `env.docker`. */
export interface Docker {
  compose: string;
  services: string[];
}

/** Everything `preflight` and `bringUp` start or wait on, replaced under test. Each one left out is the real one. */
export interface DockerSeams {
  /** The path of a program on `PATH`, or null. */
  which(exe: string): string | null;
  run(argv: string[], o: RunOptions): Promise<Pick<RunResult, "code" | "stdout" | "stderr">>;
  /** Starts a program outside this process's job, and does not wait for it. */
  detach(exe: string, args: string[]): void;
  sleep(ms: number): Promise<void>;
  /** How long `preflight` waits for a Docker Desktop it started to answer. */
  waitMs: number;
}

const REAL: DockerSeams = {
  which: (exe) => Bun.which(exe),
  run: runProcess,
  // node:child_process, not `Bun.spawn`: only `detached` keeps the child out of this process's job.
  detach: (exe, args) => spawn(exe, args, { detached: true, stdio: "ignore", windowsHide: true }).on("error", () => {}).unref(),
  sleep: (ms) => Bun.sleep(ms),
  waitMs: 300_000,
};

/** The daemon's version, or null when `docker info` fails. */
async function daemonVersion(s: DockerSeams): Promise<string | null> {
  const r = await s.run(["docker", "info", "--format", "{{.ServerVersion}}"], { timeoutMs: 120_000 });
  return r.code === 0 ? r.stdout.trim() : null;
}

/** Whether a Docker daemon answers, after starting Docker Desktop when it does not. Returns "" when it does, or why the goal cannot run. */
export async function preflight(docker: Docker | undefined, say: Say, over: Partial<DockerSeams> = {}): Promise<string> {
  if (docker === undefined) return "";
  const s: DockerSeams = { ...REAL, ...over };
  const exe = s.which("docker");
  if (!exe) return "this goal needs Docker and the `docker` command is not on PATH. Install Docker Desktop, or run this goal by hand.";
  let version = await daemonVersion(s);
  if (version === null) {
    // Docker Desktop that runs while its engine is stuck at `stopping` answers `status` and needs a restart.
    const verb = (await s.run([exe, "desktop", "status"], { timeoutMs: 60_000 })).code === 0 ? "restart" : "start";
    say(`no docker daemon answers -- \`docker desktop ${verb}\`, detached so Docker Desktop outlives this turn`);
    s.detach(exe, ["desktop", verb, "--timeout", String(s.waitMs / 1000)]);
    const until = Date.now() + s.waitMs;
    while (version === null && Date.now() < until) {
      await s.sleep(5_000);
      version = await daemonVersion(s);
    }
  }
  if (version === null) return "this goal needs a reachable Docker daemon, and `docker info` still failed after Docker Desktop was started. Start Docker Desktop and run again; nothing has been spent.";
  say(`docker daemon ${version} is up`);
  return "";
}

/** `docker compose -f <compose> up -d --wait <services>`. Returns "" when they are up, or why not. */
export async function bringUp(docker: Docker | undefined, say: Say, root: string = ROOT, over: Partial<DockerSeams> = {}): Promise<string> {
  if (docker === undefined) return "";
  const s: DockerSeams = { ...REAL, ...over };
  say(`bringing up ${docker.services.length > 0 ? docker.services.join(", ") : "every service"} from ${docker.compose}`);
  // The CLI waits silently while Docker pulls and starts the services outside its tree, so only the time limit bounds it.
  const r = await s.run(["docker", "compose", "-f", docker.compose, "up", "-d", "--wait", ...docker.services], { cwd: root, timeoutMs: 1_800_000, hang: false });
  if (r.code === 0) return "";
  return `\`docker compose -f ${docker.compose} up\` failed -- ${failLine(`${r.stderr}\n${r.stdout}`, r.code, "docker compose up")}. The live goal's checks need those services.`;
}

/** What the command reads, replaced under test. */
export interface ServicesSeams {
  plan(slug: string | undefined): { slug: string; goal: Goal } | null;
  docker: Partial<DockerSeams>;
  say: Say;
}

/** Parses, brings the services up and prints; returns the exit code. */
export async function services(args: string[], over: Partial<ServicesSeams> = {}): Promise<number> {
  const s: ServicesSeams = { plan: namedPlan, docker: {}, say: (l) => console.log(l), ...over };
  let slug: string | undefined;
  let up = false;
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--up") up = true;
    else if (a === "--goal" && args[i + 1] !== undefined) slug = args[++i];
    else {
      console.error(`${USAGE}\nnv services: error: ${a === "--goal" ? "--goal needs a slug" : `unexpected argument ${a}`}`);
      return 2;
    }
  }
  if (!up) {
    console.error(`${USAGE}\nnv services: error: name what to do: --up`);
    return 2;
  }
  const plan = s.plan(slug);
  if (plan === null) {
    console.error(`${USAGE}\nnv services: error: ${slug ? `no goal record for ${slug}` : "no goal is named or current"}`);
    return 2;
  }
  const docker = plan.goal.env.docker;
  if (docker === undefined) {
    s.say(`goal ${plan.slug} names no docker services -- nothing to bring up`);
    return 0;
  }
  const down = (await preflight(docker, s.say, s.docker)) || (await bringUp(docker, s.say, ROOT, s.docker));
  if (down !== "") {
    s.say(`RED ${down}`);
    return 1;
  }
  s.say(`GREEN ${docker.services.join(", ") || "every service"} up for ${plan.slug}`);
  return 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return services(args);
}
