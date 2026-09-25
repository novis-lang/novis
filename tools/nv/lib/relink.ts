// Freeing a binary's path when a running copy of it holds the path cargo has to link.
//
// Windows refuses to delete a file that is mapped into a running process, and cargo's last step is to
// remove the old executable and put the new one where it was. An editor whose `nvs.path` points into this
// tree runs `nvs lsp` for the life of its window, so every release build made while that window is open
// fails at the link with `failed to remove file`, and the tree keeps whatever binary was there before.
//
// That is an annoyance for somebody building by hand and a stop for the unattended loop, whose acceptance
// checks judge a proof against the release binary: the check asks cargo for one that is current, cargo
// cannot produce it, and a goal fails for a reason that has nothing to do with the tree. The failure also
// reads as *the tree does not build in release*, which is the opposite of what happened.
//
// **Renaming the file is allowed where deleting it is not.** The running process keeps executing the
// image it has already mapped, and the name it was loaded from falls free for the linker. So `linked`
// retries a build that failed this way once, with the old binary moved aside, and a moved copy is deleted
// by the next build that finds nobody holding it -- one of them per live editor window, inside `target/`,
// which is git-ignored and `bun nv disk`'s to clean.
//
// Nothing is moved ahead of time. Cargo relinks whenever its output is missing, so moving the binary aside
// before every build would add a link step to every no-op one, and a no-op release build is what most
// acceptance sweeps do. POSIX needs none of this, since unlinking a running binary is ordinary there; the
// code is not guarded by platform regardless, because a failure it does not recognise falls through
// untouched.

import { existsSync, readdirSync, renameSync, unlinkSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { ROOT } from "./paths.ts";

/** What cargo prints when the old executable could not be removed. Everything after it is the operating
 *  system's own wording and is translated on a localised Windows, so this prefix is the only part worth
 *  matching. */
const REMOVE_FAILED = "failed to remove file";

/** The suffix a moved-aside binary carries, before its number. */
const ASIDE = ".held-";

/** `target/release/nvs`, with this platform's extension. */
export function releaseCli(root: string = ROOT): string {
  return join(root, "target", "release", process.platform === "win32" ? "nvs.exe" : "nvs");
}

/** Whether `stderr` is cargo failing to replace `exe` because something is still running it. */
export function held(stderr: string, exe: string): boolean {
  return stderr.includes(REMOVE_FAILED) && stderr.includes(basename(exe));
}

/** Moves `exe` aside so a linker can write its path, and returns where it went. `null` when there was
 *  nothing to move or the move itself was refused, and then the caller has the build failure it already
 *  had rather than a second kind of one. */
export function free(exe: string): string | null {
  for (let n = 0; n < 64; n++) {
    const aside = `${exe}${ASIDE}${n}`;
    if (existsSync(aside)) continue;
    try {
      renameSync(exe, aside);
    } catch {
      return null;
    }
    return aside;
  }
  return null;
}

/** Deletes every copy moved aside whose holder has since exited, and returns how many went. One that is
 *  still held stays where it is: the next build sweeps what this one could not. */
export function sweep(exe: string): number {
  const dir = dirname(exe);
  const prefix = `${basename(exe)}${ASIDE}`;
  let gone = 0;
  let names: string[];
  try {
    names = readdirSync(dir);
  } catch {
    return 0;
  }
  for (const name of names) {
    if (!name.startsWith(prefix)) continue;
    try {
      unlinkSync(join(dir, name));
    } catch {
      continue;
    }
    gone++;
  }
  return gone;
}

/** Runs `build`, which links `exe`. When it failed because a running copy held `exe`, moves that copy
 *  aside and runs `build` once more. Either way, sweeps the copies nobody holds any longer. `errOf` reads
 *  the build's standard error off whatever `build` returns. */
export async function linked<R extends { code: number }>(exe: string, build: () => Promise<R>, errOf: (r: R) => string): Promise<R> {
  let r = await build();
  if (r.code !== 0 && held(errOf(r), exe) && free(exe) !== null) r = await build();
  sweep(exe);
  return r;
}
