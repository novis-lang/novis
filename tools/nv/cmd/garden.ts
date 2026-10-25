// `bun nv garden`: HTTP Garden's mutation sweep against `nvs serve`, with hyper and Node as references.
//
//     bun nv garden                                  # 2000 rounds, seed 1
//     bun nv garden --rounds 5000 --seed 7
//     bun nv garden --refs hyper                     # one reference, and no Node image to build
//     bun nv garden --nvs-bin /path/to/linux/nvs     # skip the build
//     bun nv garden --keep-up                        # leave the origins running afterwards
//     bun nv garden --down                           # tear them down and stop
//
// The Garden (`narfindustries/http-garden`, pinned at `GARDEN_COMMIT`) is cloned into `.agent-tmp/garden/`
// on the first run and kept there for the next one. Its harness connects to each origin at the
// container's IP on the network `http-garden_default`, and only a container on that network can reach
// those addresses on every host Docker runs on, Docker Desktop included. So the harness runs as the
// `runner` service of `tests/garden/compose.yaml`, from an image its files are copied into, and no step
// bind-mounts anything from the host.
//
// The Node reference is the official Node image running the Garden's `app.js`, not the Garden's own
// image, which compiles Node from source: `tests/garden/node.Dockerfile` says why. hyper is the
// Garden's image, built once and reused.
//
// The origin is a debug `nvs` for Linux, because a debug build checks invariants a release build does
// not. On Windows it is built in WSL, from the mirror and target directory `linux-test` uses. On Linux it is
// this checkout's `cargo build`. Anywhere else, `--nvs-bin` names one.
//
// The report is the readings where `nvs` differs from every reference while the references agree, which
// a person sorts into bugs and deliberate strictness. The run fails when the `nvs` log shows a panic or
// the origin is no longer running at the end, because either is a bug whatever the readings say.

import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { join } from "node:path";
import { mirrorPath, q, syncMirror, wslBuildLine, wslPath } from "../accept/mirror.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough, run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs, pyInt, pyRepr } from "../lib/py.ts";
import { Fail } from "./bench.ts";
import { requireDocker } from "./bench-docker.ts";

export const summary = "HTTP Garden's mutation sweep against nvs serve, in Docker: nv garden [--rounds N] [--seed N] [--refs LIST] [--down]";

const GARDEN_REPO = "https://github.com/narfindustries/http-garden";
const GARDEN_COMMIT = "b417e806c1b15e8ea0b9312f81a91fbdcbc7a83e";

const SCRATCH = join(ROOT, ".agent-tmp", "garden");
const CHECKOUT = join(SCRATCH, "http-garden");
const STAGE = join(SCRATCH, "stage");
const ASSETS = join(ROOT, "tests", "garden");

// `targets.py` names the network `http-garden_default`, which is the project's name plus `_default`.
const PROJECT = "http-garden";
const WSL_TARGET = "/var/tmp/nvs-target-wsl";
const REFERENCES = ["hyper", "node_stdlib"];

const HOUR_MS = 60 * 60 * 1000;

const USAGE = "usage: nv garden [-h] [--rounds N] [--seed N] [--refs LIST] [--nvs-bin PATH] [--keep-up] [--down]";

interface Options {
  rounds: number;
  seed: number;
  refs: string[];
  nvsBin: string | null;
  keepUp: boolean;
  down: boolean;
}

export function options(args: string[]): Options | null {
  const p = parseArgs(args, { flags: ["--keep-up", "--down"], valued: ["--rounds", "--seed", "--refs", "--nvs-bin"] });
  if (p.flags.has("--help")) return null;
  const int = (o: string, fallback: number): number => {
    const word = p.values.get(o);
    if (word === undefined) return fallback;
    const n = pyInt(word);
    if (n === null || n < 1) throw new ArgError(`argument ${o}: not a positive whole number: ${pyRepr(word)}`);
    return n;
  };
  const refs = (p.values.get("--refs") ?? REFERENCES.join(",")).split(",").filter((r) => r !== "");
  if (refs.length === 0) throw new ArgError("argument --refs: names no origin");
  return {
    rounds: int("--rounds", 2000),
    seed: int("--seed", 1),
    refs,
    nvsBin: p.values.get("--nvs-bin") ?? null,
    keepUp: p.flags.has("--keep-up"),
    down: p.flags.has("--down"),
  };
}

function help(): string {
  return [
    USAGE,
    "",
    "Sends mutated HTTP/1.1 requests to nvs serve and to reference servers, and lists",
    "the requests that nvs reads differently from all of them.",
    "",
    "options:",
    "  -h, --help       show this help message and exit",
    "  --rounds N       how many mutated requests to send (default 2000)",
    "  --seed N         the random seed, so a run can be repeated (default 1)",
    `  --refs LIST      the reference servers, comma-separated (default ${REFERENCES.join(",")})`,
    "  --nvs-bin PATH   a Linux nvs to test, instead of building one",
    "  --keep-up        leave the servers running after the run",
    "  --down           stop the servers and exit",
  ].join("\n");
}

// ---------------------------------------------------------------------------------------------------

/** `docker compose` over the Garden's file and ours, in the project `targets.py` expects. */
function composeArgv(...args: string[]): string[] {
  return ["docker", "compose", "-p", PROJECT, "-f", join(CHECKOUT, "docker-compose.yml"), "-f", join(ASSETS, "compose.yaml"), ...args];
}

const composeEnv = { NV_GARDEN_STAGE: STAGE.replaceAll("\\", "/") };

async function must(argv: string[], what: string, timeoutMs = HOUR_MS, cwd = ROOT): Promise<void> {
  const code = await passthrough(argv, { cwd, env: composeEnv, timeoutMs });
  if (code !== 0) throw new Fail(`${what} failed with exit status ${code}`);
}

/** The Garden at `GARDEN_COMMIT`, cloned on the first run and fetched only when the pin moves. */
async function checkout(): Promise<void> {
  if (!existsSync(join(CHECKOUT, ".git"))) {
    mkdirSync(SCRATCH, { recursive: true });
    await must(["git", "clone", "-q", GARDEN_REPO, CHECKOUT], "cloning HTTP Garden");
  }
  const has = await runProc(["git", "cat-file", "-e", `${GARDEN_COMMIT}^{commit}`], { cwd: CHECKOUT });
  if (has.code !== 0) await must(["git", "fetch", "-q", "origin", GARDEN_COMMIT], "fetching the pinned Garden commit", HOUR_MS, CHECKOUT);
  await must(["git", "checkout", "-q", "--detach", GARDEN_COMMIT], "checking out the pinned Garden commit", HOUR_MS, CHECKOUT);
}

/** Puts a Linux debug `nvs` at `dest`, refusing a file that is not a Linux binary. */
async function linuxNvs(o: Options, dest: string): Promise<void> {
  if (o.nvsBin !== null) {
    if (!existsSync(o.nvsBin) || !statSync(o.nvsBin).isFile()) throw new Fail(`--nvs-bin ${o.nvsBin} is not a file`);
    copyFileSync(o.nvsBin, dest);
  } else if (process.platform === "win32") {
    const mirror = mirrorPath(WSL_TARGET, wslPath(ROOT));
    console.log(`nvs: building in WSL from ${mirror}`);
    const synced = await syncMirror(mirror);
    if (synced !== "") throw new Fail(`the WSL copy of the tree could not be synced: ${synced}`);
    const line = `${wslBuildLine(mirror, WSL_TARGET)} && cp ${q(`${WSL_TARGET}/debug/nvs`)} ${q(wslPath(dest))}`;
    await must(["wsl.exe", "--exec", "bash", "-lc", line], "the WSL build of nvs");
  } else if (process.platform === "linux") {
    console.log("nvs: cargo build");
    await must(["cargo", "build", "--quiet"], "cargo build");
    copyFileSync(join(ROOT, "target", "debug", "nvs"), dest);
  } else {
    throw new Fail(`no Linux nvs can be built on ${process.platform}; pass --nvs-bin with one`);
  }
  const magic = readFileSync(dest).subarray(0, 4);
  if (!magic.equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]))) throw new Fail(`${o.nvsBin ?? dest} is not a Linux ELF binary`);
}

/**
 * The three build contexts: the origin's three files, the Node reference's `app.js`, and the Garden's
 * harness with `sweep.py` added.
 */
async function stage(o: Options): Promise<void> {
  rmSync(STAGE, { recursive: true, force: true });
  const origin = join(STAGE, "origin");
  const node = join(STAGE, "node");
  const runner = join(STAGE, "runner");
  for (const dir of [origin, node, runner]) mkdirSync(dir, { recursive: true });

  await linuxNvs(o, join(origin, "nvs"));
  copyFileSync(join(ASSETS, "echo.nvs"), join(origin, "echo.nvs"));
  copyFileSync(join(ASSETS, "origin.Dockerfile"), join(origin, "Dockerfile"));

  copyFileSync(join(CHECKOUT, "images", "node_stdlib", "app.js"), join(node, "app.js"));
  copyFileSync(join(ASSETS, "node.Dockerfile"), join(node, "Dockerfile"));

  cpSync(join(CHECKOUT, "tools"), join(runner, "tools"), { recursive: true });
  for (const f of ["docker-compose.yml", "quirks.yml"]) copyFileSync(join(CHECKOUT, f), join(runner, f));
  copyFileSync(join(ASSETS, "external-services.yml"), join(runner, "external-services.yml"));
  copyFileSync(join(ASSETS, "sweep.py"), join(runner, "tools", "sweep.py"));
  copyFileSync(join(ASSETS, "runner.Dockerfile"), join(runner, "Dockerfile"));
}

async function down(): Promise<void> {
  if (!existsSync(join(CHECKOUT, "docker-compose.yml"))) return;
  await passthrough(composeArgv("down", "--remove-orphans"), { env: composeEnv, timeoutMs: 10 * 60 * 1000 });
}

/** The lines of the `nvs` log that say it panicked, and whether the origin is still running. */
async function origin(): Promise<{ panics: string[]; running: boolean }> {
  const logs = await runProc(composeArgv("logs", "--no-color", "nvs"), { env: composeEnv, timeoutMs: 120_000 });
  const panics = `${logs.stdout}\n${logs.stderr}`.split("\n").filter((l) => l.includes("panicked at"));
  const ps = await runProc(composeArgv("ps", "--status", "running", "-q", "nvs"), { env: composeEnv, timeoutMs: 120_000 });
  return { panics, running: ps.stdout.trim() !== "" };
}

async function main(o: Options): Promise<number> {
  requireDocker();
  if (o.down) {
    await down();
    return 0;
  }

  await checkout();
  await stage(o);
  try {
    if (o.refs.some((r) => r !== "nvs")) {
      await must(["docker", "build", "-q", "-t", "http-garden-soil", join(CHECKOUT, "images", "http-garden-soil")], "building the Garden's base image");
    }
    await must(composeArgv("up", "-d", "--build", "nvs", ...o.refs), "starting the origins");
    const swept = await passthrough(
      composeArgv("run", "--rm", "--build", "runner", "--refs", o.refs.join(","), "--rounds", String(o.rounds), "--seed", String(o.seed)),
      { env: composeEnv, timeoutMs: 4 * HOUR_MS },
    );
    if (swept !== 0) throw new Fail(`the sweep failed with exit status ${swept}`);

    const seen = await origin();
    for (const line of seen.panics.slice(0, 20)) console.log(`nvs log: ${line}`);
    if (seen.panics.length > 0) console.log(`nvs panicked ${seen.panics.length} time(s): \`docker compose -p ${PROJECT} logs nvs\` has the rest`);
    if (!seen.running) console.log("nvs is no longer running");
    return seen.panics.length > 0 || !seen.running ? 1 : 0;
  } finally {
    if (o.keepUp) console.log(`left running: \`bun nv garden --down\` stops it`);
    else await down();
    rmSync(STAGE, { recursive: true, force: true });
  }
}

export async function run(args: string[]): Promise<number> {
  let o: Options | null;
  try {
    o = options(args);
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv garden: error: ${e.message}`);
    return 2;
  }
  if (o === null) {
    console.log(help());
    return 0;
  }
  try {
    return await main(o);
  } catch (e) {
    if (!(e instanceof Fail)) throw e;
    console.error(`nv garden: ${e.message}`);
    return 1;
  }
}
