// `bun nv wsl`: the WSL distro that runs this repository's Linux work on Windows.
//
//     bun nv wsl                          is the distro there, healthy, and does it have every tool
//     bun nv wsl --setup                  make it, or bring its tools up to date
//     bun nv wsl --setup --location D:\wsl\novis-test   make it with its disk in that folder
//     bun nv wsl --reset                  delete it and make it again
//     bun nv wsl --remove                 delete it
//
// The distro is `novis-test`, or what `NVS_WSL_DISTRO` names (`lib/wsl.ts`, which says why it holds
// nothing else). `--setup` installs Ubuntu 26.04, the release CI's Linux jobs run on, under that name, and
// runs `tools/wsl/provision.sh` in it: once as root for the packages, Node, the user `novis`, `wsl.conf`
// and the sysctl, and once as `novis` for Rust, Miri, cargo-nextest, cargo-fuzz and Bun. Every version is
// read from the file CI reads it from, so a bump there reaches the distro at its next `--setup`. Running
// `--setup` again on a distro it made installs only what changed. Without `--location`, the disk goes where
// WSL puts every distro's disk. Docker comes from Docker Desktop, whose WSL integration is turned on for the
// distro by hand once: a second Docker engine in the same WSL VM would fight Docker Desktop's over the
// network.
//
// `--reset` and `--remove` delete only a distro that `--setup` made, which `MARKER` inside it shows, so a
// name that clashes with somebody's own distro never deletes it. Everything in the distro is a cache, so a
// reset loses only build time. What it spends: the disk, which grows with the target directories
// (`bun nv disk --deep` reports them), and about 20 minutes for a first `--setup`.
//
// Windows only: on Linux the steps run on the machine itself.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { wslPath } from "../accept/mirror.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough, run as runProc } from "../lib/proc.ts";
import { DISTRO, distros, health, MARKER, wslArgv, wslExec, wslText } from "../lib/wsl.ts";
import { TOOLS } from "./deep.ts";

export const summary = "makes, checks and resets the WSL distro the Linux steps run in: nv wsl [--setup [--location <dir>] | --reset | --remove]";

const USAGE = "usage: bun nv wsl [--setup [--location <dir>] | --reset | --remove]";

/** The release the distro is made from: what CI's Linux jobs run on (`ci.yml`'s `runs-on`). */
export const IMAGE = "Ubuntu-26.04";
/** The user the distro runs as. */
export const USER = "novis";
/** A download and the Rust toolchains it installs. */
const SETUP_TIMEOUT_MS = 60 * 60 * 1000;

/** Each version `provision.sh` installs, read from the file CI reads it from. Throws when a file lost it. */
export function versions(root: string = ROOT): Record<string, string> {
  const read = (file: string): string => readFileSync(join(root, file), "utf8");
  const find = (file: string, re: RegExp, what: string): string => {
    const m = re.exec(read(file));
    if (m === null) throw new Error(`${file} no longer names ${what} where ${re} looks for it`);
    return m[1]!;
  };
  return {
    NVS_NODE_MAJOR: find("editors/vscode/.nvmrc", /^\s*v?(\d+)/, "Node's major version"),
    NVS_BUN: find("package.json", /"packageManager":\s*"bun@([^"]+)"/, "Bun's version"),
    NVS_NEXTEST: find(".github/workflows/ci.yml", /NEXTEST_VERSION:\s*'([^']+)'/, "cargo-nextest's version"),
    NVS_FUZZ: find(".github/workflows/ci.yml", /cargo install cargo-fuzz --version (\S+)/, "cargo-fuzz's version"),
  };
}

/** The argv that runs `provision.sh <mode>` in the distro, as root for `system`. */
export function provisionArgv(mode: "system" | "user", root: string = ROOT): string[] {
  const repo = wslPath(root);
  const env = { NVS_REPO: repo, NVS_USER: USER, ...versions(root) };
  const argv = ["env", ...Object.entries(env).map(([k, v]) => `${k}=${v}`), "bash", `${repo}/tools/wsl/provision.sh`, mode];
  return mode === "system" ? wslExec(argv, { root: true }) : ["wsl.exe", "-d", DISTRO, "-u", USER, "--exec", ...argv];
}

/** The `bash` line that prints `made` when `--setup` made the distro, `foreign` when somebody uses it, else `fresh`. */
const OWNER_LINE = `if [ -f ${MARKER} ]; then echo made; elif getent passwd | awk -F: '$3 >= 1000 && $3 < 65534 { f = 1 } END { exit !f }'; then echo foreign; else echo fresh; fi`;

/** Whether `--setup` made the distro (`made`), it has somebody's user (`foreign`), it is fresh, or it does not start. */
async function owner(): Promise<"made" | "foreign" | "fresh" | "down"> {
  const r = await runProc(wslArgv(OWNER_LINE, { login: false, root: true }), { timeoutMs: 120_000, hang: false });
  const out = wslText(r.stdout);
  return r.code === 0 && (out === "made" || out === "foreign" || out === "fresh") ? out : "down";
}

async function wsl(args: string[], timeoutMs = 300_000): Promise<number> {
  return passthrough(["wsl.exe", ...args], { timeoutMs, hang: false });
}

async function setup(location: string | null): Promise<number> {
  if (!(await distros()).includes(DISTRO)) {
    console.log(`wsl: installing ${IMAGE} as \`${DISTRO}\``);
    const where = location === null ? [] : ["--location", location];
    const code = await wsl(["--install", IMAGE, "--name", DISTRO, "--no-launch", "--web-download", ...where], SETUP_TIMEOUT_MS);
    if (code !== 0) return fail(`wsl --install exited ${code}`);
  } else {
    const who = await owner();
    if (who === "foreign") {
      return fail(`\`${DISTRO}\` exists and was not made by this command. Set NVS_WSL_DISTRO to another name, or remove it yourself.`);
    }
    if (who === "down") return fail(`\`${DISTRO}\` does not start. Repair it with \`bun nv wsl --reset\`.`);
    if (location !== null) console.log(`wsl: \`${DISTRO}\` exists, so --location changes nothing`);
  }
  console.log(`wsl: provisioning \`${DISTRO}\` as root`);
  let code = await passthrough(provisionArgv("system"), { timeoutMs: SETUP_TIMEOUT_MS, hang: false });
  if (code !== 0) return fail(`provision.sh system exited ${code}`);
  // wsl.conf is read when the distro starts.
  await wsl(["--terminate", DISTRO]);
  console.log(`wsl: provisioning \`${DISTRO}\` as ${USER}`);
  code = await passthrough(provisionArgv("user"), { timeoutMs: SETUP_TIMEOUT_MS, hang: false });
  if (code !== 0) return fail(`provision.sh user exited ${code}`);
  await wsl(["--manage", DISTRO, "--set-default-user", USER]);
  await wsl(["--terminate", DISTRO]);
  return check();
}

async function remove(): Promise<number> {
  if (!(await distros()).includes(DISTRO)) {
    console.log(`wsl: \`${DISTRO}\` does not exist`);
    return 0;
  }
  const who = await owner();
  if (who !== "made" && who !== "fresh") {
    const why = who === "down" ? "does not start, so its marker cannot be read" : "was not made by `bun nv wsl --setup`";
    return fail(`\`${DISTRO}\` ${why}. If it is yours to delete, run: wsl --unregister ${DISTRO}`);
  }
  console.log(`wsl: deleting \`${DISTRO}\` and its disk`);
  await wsl(["--terminate", DISTRO]);
  const code = await wsl(["--unregister", DISTRO]);
  return code === 0 ? 0 : fail(`wsl --unregister exited ${code}`);
}

/** Prints the distro's health and every tool a step needs in it. Exits 1 when anything is missing. */
async function check(): Promise<number> {
  const sick = await health();
  if (sick !== "") return fail(sick);
  let missing = 0;
  for (const [name, tool] of Object.entries(TOOLS)) {
    if (tool.where !== "wsl") continue;
    const r = await runProc(wslArgv(tool.probe), { timeoutMs: 120_000, hang: false });
    if (r.code === 0) {
      console.log(`  ok       ${name}`);
    } else {
      missing++;
      console.log(`  missing  ${name}: ${tool.install}`);
    }
  }
  console.log(missing === 0 ? `wsl: \`${DISTRO}\` is healthy and has every tool` : `wsl: \`${DISTRO}\` is healthy, and ${missing} tool(s) are missing`);
  return missing === 0 ? 0 : 1;
}

function fail(message: string): number {
  console.error(`nv wsl: ${message}`);
  return 1;
}

export async function run(args: string[]): Promise<number> {
  if (args[0] === "--help" || args[0] === "-h") {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  if (process.platform !== "win32") {
    console.log("nv wsl: this host is not Windows; the Linux steps run on it directly");
    return 0;
  }
  const at = args.indexOf("--location");
  const location = at >= 0 ? (args[at + 1] ?? null) : null;
  const rest = at < 0 ? args : args.filter((_, i) => i !== at && i !== at + 1);
  if (at >= 0 && (location === null || rest[0] !== "--setup")) {
    console.error(`${USAGE}\nnv wsl: error: --location <dir> goes with --setup`);
    return 2;
  }
  switch (rest.join(" ")) {
    case "":
      return check();
    case "--setup":
      return setup(location);
    case "--reset": {
      const code = await remove();
      return code === 0 ? setup(null) : code;
    }
    case "--remove":
      return remove();
    default:
      console.error(`${USAGE}\nnv wsl: error: unknown arguments: ${args.join(" ")}`);
      return 2;
  }
}
