// The WSL distro that runs this repository's Linux work on Windows, and the one way to start a program in it.
//
// Every Linux step on Windows (`deep`, `linux-test`, `fuzz`, `garden`, `scaling`, `db-matrix`, `webp-lib`)
// runs in one distro that holds nothing else: `novis-test`, unless `NVS_WSL_DISTRO` names another.
// `bun nv wsl --setup` makes it from Ubuntu 26.04, the release CI's Linux jobs run on, and
// `tools/wsl/provision.sh` installs what the steps need at the versions CI reads. A developer's own
// distros are never used. Every call names the distro with `-d`, because the default distro can be any
// distro at all, and one a person works in is no place for a test to fill or break.
//
// The distro is disposable. Its disk holds caches only: target directories, the tree's copy and fuzz
// corpora, each of which a run makes again. When its disk fails, `bun nv wsl --reset` deletes it and makes
// it again, and nothing is lost. `health` runs before every sync of the tree's copy (`accept/mirror.ts`),
// so a disk the kernel remounted read-only, or one that logged an ext4 error, stops the run at once and
// names that command. No step runs for hours on a broken disk.
//
// What it costs: one `wsl.exe` round trip per sync of the copy, about a tenth of a second.

import { run } from "./proc.ts";

/** The distro's name, which `NVS_WSL_DISTRO` overrides. */
export const DISTRO = process.env.NVS_WSL_DISTRO || "novis-test";

/** The file `provision.sh` writes, which marks a distro as one this repository made and may delete. */
export const MARKER = "/etc/novis-test-distro";

/** The command that repairs a broken or missing distro. */
export const RESET = "bun nv wsl --reset";

/** How a line runs in the distro. */
export interface WslOptions {
  /** A login shell, which reads `~/.profile` and so finds cargo and bun. True unless set false. */
  login?: boolean;
  /** Run as root instead of the distro's own user. */
  root?: boolean;
}

/** The argv that runs the `bash` line `line` in the distro. */
export function wslArgv(line: string, opts: WslOptions = {}): string[] {
  const user = opts.root ? ["-u", "root"] : [];
  return ["wsl.exe", "-d", DISTRO, ...user, "--exec", "bash", opts.login === false ? "-c" : "-lc", line];
}

/** The argv that runs `argv` in the distro with no shell between. */
export function wslExec(argv: string[], opts: Pick<WslOptions, "root"> = {}): string[] {
  return ["wsl.exe", "-d", DISTRO, ...(opts.root ? ["-u", "root"] : []), "--exec", ...argv];
}

/** `wsl.exe`'s own messages are UTF-16; a program's output in the distro is UTF-8. This reads both. */
export function wslText(text: string): string {
  return text.replaceAll("\0", "").replaceAll("\r", "").trim();
}

/** The names of the registered distros. */
export async function distros(): Promise<string[]> {
  const r = await run(["wsl.exe", "--list", "--quiet"], { timeoutMs: 60_000, hang: false });
  return wslText(r.stdout).split("\n").map((s) => s.trim()).filter((s) => s !== "");
}

/**
 * The `bash` line `health` runs as root. It prints one line per fault and nothing on a healthy disk: the
 * root mounted read-only, a `MARKER` that is missing, or an ext4 error on the root's disk.
 *
 * Every distro shares one kernel, so its log holds every disk mounted since the VM booted, and a device
 * name such as `sdd` passes from one disk to the next. An error counts only when it names the root's
 * device after the last line that mounted the root's own filesystem, found by its UUID.
 */
export const HEALTH_LINE = [
  `src=$(findmnt -no SOURCE /); o=$(findmnt -no OPTIONS /)`,
  `case ",$o," in *,ro,*) echo "its disk is mounted read-only ($o)";; esac`,
  `[ -f ${MARKER} ] || echo "it was not made by bun nv wsl --setup, or its setup did not finish"`,
  `dev=\${src#/dev/}; uuid=$(blkid -s UUID -o value "$src" 2>/dev/null)`,
  `dmesg 2>/dev/null | awk -v d="$dev" -v u="$uuid" 'u != "" && index($0, "EXT4-fs (" d "): mounted filesystem " u) { on = 1; next } on && index($0, "(device " d ")") && /error|checksum/ { print "the kernel logged:" substr($0, index($0, "]") + 1); exit }'`,
  `true`,
].join("; ");

/** "" when the distro is there and its disk is healthy, else what is wrong and how to repair it. */
export async function health(): Promise<string> {
  if (process.platform !== "win32") return "";
  const r = await run(wslArgv(HEALTH_LINE, { login: false, root: true }), { timeoutMs: 120_000, hang: false });
  if (r.code !== 0) {
    const names = await distros();
    if (!names.includes(DISTRO)) return `the WSL distro \`${DISTRO}\` does not exist; make it with \`bun nv wsl --setup\``;
    return `the WSL distro \`${DISTRO}\` did not start (${wslText(r.stderr || r.stdout).split("\n").pop()}); repair it with \`${RESET}\``;
  }
  const faults = wslText(r.stdout);
  if (faults === "") return "";
  return `the WSL distro \`${DISTRO}\` is broken: ${faults.split("\n").join("; ")}. It holds caches only; repair it with \`${RESET}\``;
}
