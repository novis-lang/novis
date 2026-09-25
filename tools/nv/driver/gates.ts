// What the driver runs between sessions besides the acceptance sweep: the two goal-end gates, a goal's
// services, and the disk.
//
// **The goal-end gates** run on a sweep that would reach the goal, and a goal is not reached while either
// is red. The rustdoc gate is `bun nv verify --doc`: a broken intra-doc link stops no build, so a goal in
// progress may carry one, and what has to be clean is the tree a goal leaves behind. The owner gate is
// `bun nv owners --closes <slug>` and `bun nv playbook --closes <slug>`: a gap or a trap still naming the
// goal is a tag, not a build, and the floor's own check of it would only fire one goal late, once the goal
// has walked. Each writes its standing verdict to its file under `.loop/`, `{ when, failed, session }`,
// which `nv orient` prints to the next session; a red gate holds the goal open without ending the run.
//
// **A goal's services** are its record's `env.docker`: the compose file and the services its checks
// reach. `preflight` asks for a reachable Docker daemon before a session is spent against checks that
// cannot pass, and `bringUp` starts the services once per run and once per goal switch, with `--wait`, so
// "up" means healthy.
//
// **The disk.** A run refuses to start below `--min-free-gb`, since a run that fills the disk dies inside a
// session with the tree half edited. `sweepDisk` is `nv disk --clean`'s sweep, run in-process after every
// session's acceptance sweep: the command itself refuses while `.loop/running` exists, and the driver is
// the one process that knows nothing is building.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { clean, freeGb, freedLines, human, total } from "../cmd/disk.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { RUNDIR } from "./launch.ts";

export const DOC_GATE = `${RUNDIR}/doc-gate.json`;
export const OWNER_GATE = `${RUNDIR}/owner-gate.json`;

/** What a gate or a service step printed, one line to the caller. */
type Say = (line: string) => void;

/** The first non-empty line of a process's output that starts `error`, or its last one. */
function why(out: string, code: number, what: string): string {
  const lines = out.replace(/\r\n/g, "\n").split("\n").map((l) => l.trim()).filter(Boolean);
  return lines.find((l) => l.startsWith("error")) ?? lines.at(-1) ?? `\`${what}\` exited ${code}`;
}

/** Writes a gate's standing verdict: `failed` is "" when it is green. */
function writeGate(path: string, failed: string, session: string, root: string): void {
  const file = join(root, path);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, `${JSON.stringify({ when: Date.now() / 1000, failed, session }, null, 1)}\n`);
}

/** `bun nv verify --doc`. Returns the finding, or "" when green. */
export async function docGate(session: string, say: Say, root: string = ROOT): Promise<string> {
  say("rustdoc gate: every link in a doc comment, resolved (the acceptance list is green)");
  const r = await run(["bun", "nv", "verify", "--doc"], { cwd: root, timeoutMs: 900_000 });
  const found = r.code === 0 ? "" : why(`${r.stdout}\n${r.stderr}`, r.code, "bun nv verify --doc");
  writeGate(DOC_GATE, found, found ? session : "", root);
  return found;
}

/** `nv owners --closes <slug>` and `nv playbook --closes <slug>`. Returns the findings joined, or "" when green. */
export async function ownerGate(slug: string, session: string, say: Say, root: string = ROOT): Promise<string> {
  say(`owner gate: no gap or trap names goal \`${slug}\` (the acceptance list is green)`);
  const found: string[] = [];
  for (const tool of ["owners", "playbook"]) {
    const r = await run(["bun", "nv", tool, "--closes", slug], { cwd: root, timeoutMs: 300_000 });
    if (r.code === 0) continue;
    const lines = `${r.stdout}\n${r.stderr}`.replace(/\r\n/g, "\n").split("\n").map((l) => l.trim()).filter(Boolean);
    found.push(lines.at(-1) ?? `\`bun nv ${tool} --closes ${slug}\` exited ${r.code}`);
  }
  const joined = found.join("; ");
  writeGate(OWNER_GATE, joined, joined ? session : "", root);
  return joined;
}

/** A goal record's `env.docker`. */
export interface Docker {
  compose: string;
  services: string[];
}

/** Whether a Docker daemon answers. Returns "" when it does, or why the goal cannot run. */
export async function preflight(docker: Docker | undefined, say: Say): Promise<string> {
  if (docker === undefined) return "";
  if (!Bun.which("docker")) return "this goal needs Docker and the `docker` command is not on PATH. Install Docker Desktop, or run this goal by hand.";
  const r = await run(["docker", "info", "--format", "{{.ServerVersion}}"], { timeoutMs: 120_000 });
  if (r.code !== 0) return "this goal needs a reachable Docker daemon and `docker info` failed. Start Docker Desktop and run again; nothing has been spent.";
  say(`docker daemon ${r.stdout.trim()} is up`);
  return "";
}

/** `docker compose -f <compose> up -d --wait <services>`. Returns "" when they are up, or why not. */
export async function bringUp(docker: Docker | undefined, say: Say, root: string = ROOT): Promise<string> {
  if (docker === undefined) return "";
  say(`bringing up ${docker.services.length > 0 ? docker.services.join(", ") : "every service"} from ${docker.compose}`);
  const r = await run(["docker", "compose", "-f", docker.compose, "up", "-d", "--wait", ...docker.services], { cwd: root, timeoutMs: 1_800_000 });
  if (r.code === 0) return "";
  return `\`docker compose -f ${docker.compose} up\` failed -- ${why(`${r.stderr}\n${r.stdout}`, r.code, "docker compose up")}. The live goal's checks need those services.`;
}

/** Whether the disk has room for a run. Returns "" when it has, or the refusal. */
export function enoughDisk(minFreeGb: number, root: string = ROOT): string {
  const free = freeGb(root);
  if (minFreeGb <= 0 || free >= minFreeGb) return "";
  return `${free.toFixed(1)}G free, and a run needs ${minFreeGb}G. \`bun nv disk\` says what holds it, and \`--clean\` drops the superseded build generations; --min-free-gb 0 starts anyway.`;
}

/** `nv disk --clean`'s sweep. Returns the one-line summary the ledger keeps, and says each part. */
export async function sweepDisk(keepRuns: number, say: Say, root: string = ROOT): Promise<string> {
  const before = freeGb(root);
  let freed: Record<string, number | null>;
  try {
    freed = await clean({ keepRuns });
  } catch (e) {
    const failed = `disk: the sweep failed -- ${(e as Error).message}`;
    say(failed);
    return failed;
  }
  let summary = `disk: freed ${human(total(freed))}, ${before.toFixed(1)}G -> ${freeGb(root).toFixed(1)}G free`;
  say(summary);
  for (const line of freedLines(freed)) say(`  ${line}`);
  if (freed["target/deps"] === null) summary += "; deps/ left alone, cargo could not name the live set";
  return summary;
}
