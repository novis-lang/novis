// `bun nv checkout-acl`: on Windows, removes the write grant other accounts inherit on this checkout, once,
// so `nvs serve` passes the ownership check on the repository's own configuration files.
//
// `rule:config/ownership-is-the-trust-boundary` refuses a configuration file, or its directory, that
// `Authenticated Users` or `Everyone` may write. Windows gives `Authenticated Users` modify rights on a
// non-system drive's root, and a checkout there inherits them, so every server the repository starts on a
// file in the checkout, and every served test under `target/`, stops with `E0607` until that grant is
// gone. This command breaks the checkout root's inheritance (the inherited entries are copied as its own),
// removes every grant to those two groups, and gives this account full control. Everything below the root
// inherits the result. `BUILTIN\Users` keeps its read access.
//
// A root that already has no inherited entry is left alone, so a second run does nothing. On any other
// platform the command does nothing: a Unix checkout is the mode bits git already wrote.
//
// To undo: `icacls <checkout> /inheritance:e` and `icacls <checkout> /grant *S-1-5-11:(OI)(CI)(M)`.
//
// CI's Windows jobs run it right after the checkout. Its cost is one propagation over the tree, which
// is why the CI step runs before the build writes `target/`.
//
//     bun nv checkout-acl            fix the checkout this file is in
//     bun nv checkout-acl --check    exit 1 when the root still inherits, and change nothing

import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";

export const summary = "on Windows, remove the write grant other accounts inherit on the checkout: nv checkout-acl [--check]";

/** The SIDs a configuration file may not be writable by, which this command removes from the root. */
const OTHERS = ["*S-1-5-11", "*S-1-1-0"];

/** The longest one propagation over a large tree may take. */
const PROPAGATION_MS = 30 * 60_000;

/** Whether the root's DACL still has an inherited entry, as `icacls` prints it: `(I)`. */
function inherits(listing: string): boolean {
  return listing.includes("(I)");
}

async function icacls(args: string[]): Promise<{ code: number; out: string }> {
  const r = await runProc(["icacls", ROOT, ...args], { timeoutMs: PROPAGATION_MS });
  return { code: r.timedOut ? -1 : r.code, out: r.stdout + r.stderr };
}

/**
 * This account's SID, from `whoami /user`, whose CSV row is the name and then the SID. It is Windows'
 * own `whoami.exe` by its full path: Git Bash puts a coreutils `whoami` first on `PATH`.
 */
async function mySid(): Promise<string | null> {
  const system = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "whoami.exe");
  const r = await runProc([system, "/user", "/fo", "csv", "/nh"]);
  const sid = r.stdout.trim().split(",").pop()?.replaceAll('"', "");
  return sid?.startsWith("S-1-") ? sid : null;
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const checkOnly = args.includes("--check");
  if (process.platform !== "win32") {
    console.log("nv checkout-acl: not Windows, nothing to do");
    return 0;
  }
  const listed = await icacls([]);
  if (listed.code !== 0) {
    console.error(`nv checkout-acl: icacls could not read ${ROOT}:\n${listed.out}`);
    return 1;
  }
  if (!inherits(listed.out)) {
    console.log(`nv checkout-acl: ${ROOT} inherits nothing, nothing to do`);
    return 0;
  }
  if (checkOnly) {
    console.log(`nv checkout-acl: ${ROOT} still inherits its drive's grants; run \`bun nv checkout-acl\``);
    return 1;
  }
  const sid = await mySid();
  if (sid === null) {
    console.error("nv checkout-acl: `whoami /user` named no SID for this account");
    return 1;
  }
  // This account is granted before the groups are removed, so it never loses write access in between.
  const steps: string[][] = [
    ["/inheritance:d"],
    ["/grant", `*${sid}:(OI)(CI)(F)`],
    ["/remove:g", ...OTHERS],
  ];
  for (const step of steps) {
    console.log(`$ icacls ${ROOT} ${step.join(" ")}`);
    const r = await icacls(step);
    if (r.code !== 0) {
      console.error(`nv checkout-acl: icacls ${step.join(" ")} failed:\n${r.out}`);
      return 1;
    }
  }
  console.log(`nv checkout-acl: ${ROOT} no longer lets other accounts write; everything below it inherits that`);
  return 0;
}
