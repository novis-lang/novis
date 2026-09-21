// The file `nvs lsp` is started from, and noticing when the binary it was copied from changes.
//
// The server lives as long as the window does, and every platform protects the file of a running
// program in its own way: Windows will not overwrite or delete it, Linux fails a write into it with
// "Text file busy", and macOS may kill a signed process whose file is rewritten under it. A user who
// rebuilds or upgrades `nvs` with an editor open meets that as a build that cannot write its output.
// So the server runs from a copy in the extension's own storage, and the binary the user named is
// never held open by this extension for longer than one read.
//
// A copy is only right while it is the same program as its source, and two things here keep it so.
// `shadow` hands back a copy only after hashing both files and finding them equal, whether it made
// the copy or found one already there, and `Copies` reads the source's stamp again on every call, so
// a process started a moment after a rebuild is started from that rebuild. `Watch` is for the one
// process that outlives a build, the server: it polls the source and fires once it has changed and
// then held still, which is when the client brings up the new build beside the old one and switches
// — so an updated binary is answering within a few seconds of being written, on every platform,
// with nothing for the user to remember and nothing for them to see.
//
// Every process the extension starts runs from a copy, the short ones too: a program left running in
// a Task's terminal holds its file exactly as the server does, and `binary.ts` is the one place all
// of them ask.
//
// What this spends is disk, not memory: one file the size of `nvs` per source binary, since `sweep`
// removes the copy of every earlier build each time a new one is made.
//
// Nothing here imports `vscode`, for the reason `version.ts` gives.

import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { chmod, copyFile, mkdir, readdir, rename, rm, stat } from "node:fs/promises";
import { delimiter, join, resolve } from "node:path";

/** How often `Watch` looks at the source. One `stat` a second is below anything a machine notices. */
const POLL_MS = 1000;

/** How many times `shadow` starts over when the source changes underneath it, and the wait between. */
const ATTEMPTS = 5;
const RETRY_MS = 500;

/** How long an unfinished copy is left alone, since another window may still be writing it. */
const PARTIAL_GRACE_MS = 60_000;

/** How long a copy of some *other* source binary is kept, since another window may be running it. */
const FOREIGN_GRACE_MS = 7 * 24 * 60 * 60 * 1000;

/** Every file this module writes starts with this, which is how `sweep` knows what is its to delete. */
const PREFIX = "nvs-";

/** What is compared to tell one build of a binary from the next without reading it. */
export interface Stamp {
  readonly size: number;
  /** Milliseconds since the epoch, fractional where the file system records finer than one. */
  readonly modified: number;
}

/** The stamp of the file at `path`, or `undefined` if there is no file there. */
export async function stamp(path: string): Promise<Stamp | undefined> {
  try {
    const found = await stat(path);
    return found.isFile() ? { size: found.size, modified: found.mtimeMs } : undefined;
  } catch {
    return undefined;
  }
}

export function same(a: Stamp, b: Stamp): boolean {
  return a.size === b.size && a.modified === b.modified;
}

/** Where a command is looked for, which is the environment of the process that would spawn it. */
export interface Where {
  /** The `PATH` to search for a bare name. */
  readonly path: string | undefined;
  /** Windows' `PATHEXT`; read only when `platform` is `win32`. */
  readonly pathext: string | undefined;
  /** What a relative path is relative to: the directory the client spawns the server in. */
  readonly cwd: string | undefined;
  readonly platform: NodeJS.Platform;
}

/**
 * The file `command` names, or `undefined` if no file answers to it.
 *
 * This is the platform's own resolution, done here because a copy needs a path and a bare `nvs` is
 * not one: a name with a separator in it is a path, and anything else is searched for along `PATH`.
 * On Windows a name with no extension is tried with each of `PATHEXT`'s, as the shell would — in
 * lower case, because `PATHEXT` is written in capitals and the file almost never is, and this path
 * is the one the status item shows.
 */
export async function locate(command: string, where: Where): Promise<string | undefined> {
  const windows = where.platform === "win32";
  const pathlike = command.includes("/") || (windows && command.includes("\\"));
  const dirs = pathlike ? [where.cwd ?? "."] : (where.path ?? "").split(delimiter).filter((dir) => dir !== "");
  const extensions = windows
    ? (where.pathext ?? ".COM;.EXE;.BAT;.CMD").split(";").filter((ext) => ext !== "")
    : [];
  const complete = extensions.some((ext) => command.toLowerCase().endsWith(ext.toLowerCase()));
  const names = !windows || complete ? [command] : extensions.map((ext) => command + ext.toLowerCase());
  for (const dir of dirs) {
    for (const name of names) {
      const candidate = resolve(dir, name);
      if ((await stamp(candidate)) !== undefined) {
        return candidate;
      }
    }
  }
  return undefined;
}

/**
 * The name a copy of `source` at `at` is kept under.
 *
 * The digest of the path keeps two source binaries apart, and the stamp keeps two builds of one
 * apart, so every window running the same build shares one file and no window ever needs to replace
 * a file another one is running. Windows decides what is runnable by extension, so the copy has one.
 */
export function copyName(source: string, at: Stamp, platform: NodeJS.Platform): string {
  return `${family(source)}${at.size}-${Math.trunc(at.modified)}${platform === "win32" ? ".exe" : ""}`;
}

/** The start of every name `copyName` gives a copy of `source`, whichever build it is a copy of. */
function family(source: string): string {
  return `${PREFIX}${createHash("sha256").update(source).digest("hex").slice(0, 8)}-`;
}

/** A copy the server may be started from. */
export interface Shadow {
  /** The binary the user named. */
  readonly source: string;
  /** The copy of it, which is the file to spawn. */
  readonly path: string;
  /** The source's stamp when the two were found equal, which is what `Watch` compares against. */
  readonly stamp: Stamp;
}

/**
 * A copy of `source` under `dir` that is byte for byte the same program, made if it is not there.
 *
 * Equality is a hash of both files and not a comparison of stamps, and the source's stamp is read
 * again afterwards: a linker writes its output in more than one step, and a copy taken between two
 * of them is a program that never existed. Such a copy is discarded and the whole thing starts over,
 * a bounded number of times, after which this throws and the caller decides what to run instead.
 *
 * The bytes land under a name of this process's own and are renamed once they are verified, so no
 * window ever spawns a file another window is still writing.
 */
export async function shadow(
  source: string,
  dir: string,
  platform: NodeJS.Platform = process.platform,
): Promise<Shadow> {
  for (let attempt = 0; attempt < ATTEMPTS; attempt += 1) {
    if (attempt > 0) {
      await pause(RETRY_MS);
    }
    const before = await stamp(source);
    if (before === undefined) {
      throw new Error(`${source} is not a file.`);
    }
    const path = join(dir, copyName(source, before, platform));
    if ((await stamp(path)) !== undefined) {
      if (await matches(source, path, before)) {
        return { source, path, stamp: before };
      }
      // A file under this build's name that is not this build. Removing it throws while another
      // window is running it, which is the right outcome: there is no honest copy to hand back.
      await rm(path);
    }
    await mkdir(dir, { recursive: true });
    const partial = `${path}.${process.pid}.part`;
    try {
      await copyFile(source, partial);
      if (platform !== "win32") {
        await chmod(partial, 0o755);
      }
      if (await matches(source, partial, before)) {
        await place(partial, path);
        return { source, path, stamp: before };
      }
    } finally {
      await rm(partial, { force: true });
    }
  }
  throw new Error(`${source} kept changing while it was being copied.`);
}

/**
 * The copy to run *now*, for a caller that spawns more than once: every process this extension
 * starts asks here first, so none of them is ever an older build than the one on disk.
 *
 * Asking costs two `stat`s while nothing has changed. A verified copy is remembered with the stamp
 * its source had and the stamp the copy itself had, and it is handed back only while both files
 * still read the same; a source that is a new build, and a copy that was touched or deleted, each
 * go back through `shadow` and its hashes. Calls take turns, because two copies of one build made
 * at once by one process would share the name of their unfinished file.
 */
export class Copies {
  private verified: { readonly shadow: Shadow; readonly copy: Stamp } | undefined;
  private last: Promise<unknown> = Promise.resolve();

  constructor(
    private readonly dir: string,
    private readonly platform: NodeJS.Platform = process.platform,
  ) {}

  of(source: string): Promise<Shadow> {
    const next = this.last.catch(() => undefined).then(() => this.current(source));
    this.last = next;
    return next;
  }

  private async current(source: string): Promise<Shadow> {
    const held = this.verified;
    if (held !== undefined && held.shadow.source === source) {
      const [now, copy] = await Promise.all([stamp(source), stamp(held.shadow.path)]);
      if (now !== undefined && copy !== undefined && same(now, held.shadow.stamp) && same(copy, held.copy)) {
        return held.shadow;
      }
    }
    this.verified = undefined;
    const made = await shadow(source, this.dir, this.platform);
    const copy = await stamp(made.path);
    if (copy === undefined) {
      throw new Error(`${made.path} was deleted as soon as it was made.`);
    }
    this.verified = { shadow: made, copy };
    void sweep(this.dir, source, made.path);
    return made;
  }
}

/** Whether `copy` holds the bytes of `source`, and `source` is still the build `at` describes. */
async function matches(source: string, copy: string, at: Stamp): Promise<boolean> {
  const [from, to] = await Promise.all([digest(source), digest(copy)]);
  const after = await stamp(source);
  return from === to && after !== undefined && same(at, after);
}

/**
 * Move a verified copy to its name. Another window may have put the same build there in the
 * meantime, and on Windows a file that is running cannot be renamed over; its being there is the
 * outcome wanted, so that is not a failure.
 */
async function place(partial: string, path: string): Promise<void> {
  try {
    await rename(partial, path);
  } catch (cause) {
    if ((await stamp(path)) === undefined) {
      throw cause;
    }
  }
}

function digest(path: string): Promise<string> {
  return new Promise((done, failed) => {
    const hash = createHash("sha256");
    createReadStream(path)
      .on("data", (chunk) => hash.update(chunk))
      .on("error", failed)
      .on("end", () => done(hash.digest("hex")));
  });
}

function pause(ms: number): Promise<void> {
  return new Promise((done) => setTimeout(done, ms));
}

/**
 * Delete what `dir` no longer needs, given that `keep` is the copy of `source` about to be run.
 *
 * Earlier builds of the same source go at once. A copy of a different source goes only once it is
 * old, because a window on another workspace may be running it, and so does another process's
 * unfinished copy. Every failure is ignored: a file that cannot be deleted is one some window is
 * running, and the next sweep after that window closes takes it.
 */
export async function sweep(dir: string, source: string, keep: string, now: number = Date.now()): Promise<void> {
  let names: string[];
  try {
    names = await readdir(dir);
  } catch {
    return;
  }
  for (const name of names) {
    const path = join(dir, name);
    if (!name.startsWith(PREFIX) || path === keep) {
      continue;
    }
    const grace = name.endsWith(".part") ? PARTIAL_GRACE_MS : name.startsWith(family(source)) ? 0 : FOREIGN_GRACE_MS;
    try {
      const found = await stat(path);
      // A copy keeps its source's modification time on Windows and takes a new one elsewhere, and
      // its creation time is the reverse, so the later of the two is when the copy was made.
      if (now - Math.max(found.mtimeMs, found.birthtimeMs) >= grace) {
        await rm(path);
      }
    } catch {
      // Running in another window, or gone already.
    }
  }
}

/**
 * Whether the source has become a different build than the one running, and has finished becoming
 * it: `seen` differs from `running` and equals `last`, the reading before it.
 *
 * Waiting for two equal readings is what keeps a restart from landing on a half-written file. A
 * source that is missing is never a change — a build deletes its output before writing it — and
 * `running` is `undefined` when there was no file to start from, so the first settled one is news.
 */
export function changed(running: Stamp | undefined, last: Stamp | undefined, seen: Stamp | undefined): boolean {
  if (seen === undefined || last === undefined || !same(seen, last)) {
    return false;
  }
  return running === undefined || !same(seen, running);
}

/**
 * Polls `source` and calls `fire` once, when `changed` first says so; then it is finished.
 *
 * It polls rather than subscribing because the file is replaced and not edited: a watch on the path
 * dies with the file it was opened on, a watch on the directory reports differently on every
 * platform and not at all on some network and WSL mounts, and a `stat` a second works everywhere.
 * It fires once because what follows is a restart, which builds the next `Watch` against the stamp
 * of whatever it started.
 */
export class Watch {
  private last: Stamp | undefined;
  private timer: NodeJS.Timeout | undefined;
  private polling = false;

  constructor(
    private readonly source: string,
    private readonly running: Stamp | undefined,
    private readonly fire: () => void,
    everyMs: number = POLL_MS,
  ) {
    this.last = running;
    this.timer = setInterval(() => void this.poll(), everyMs);
    // The extension host is not kept alive by this, and neither is a test run.
    this.timer.unref();
  }

  close(): void {
    clearInterval(this.timer);
    this.timer = undefined;
  }

  private async poll(): Promise<void> {
    if (this.polling) {
      return;
    }
    this.polling = true;
    try {
      const seen = await stamp(this.source);
      if (this.timer === undefined) {
        return;
      }
      if (changed(this.running, this.last, seen)) {
        this.close();
        this.fire();
      }
      this.last = seen;
    } finally {
      this.polling = false;
    }
  }
}
