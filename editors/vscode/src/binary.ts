// Which `nvs` answers, and which file is run to get it, for every process this extension starts.
//
// It has one home because there are five spawns — the server in `extension.ts`, the Tasks, the
// formatter, the AST panel and the test controller — and a user who points `nvs.path` at a build
// directory means all of them. A second read with its own fallback is how one of them ends up
// running a different binary than the others.
//
// There are three candidates, tried in order: `nvs.path`, then `nvs` on `PATH`, then the copy
// `nvs.downloadBinary` installed in this extension's own storage. The installed copy is last and
// is never written into `nvs.path`, so a toolchain the user installs later still wins
// (`rule:ide/the-extension-guides-an-install-and-never-bundles-one`).
//
// None of the five runs the file a candidate names. Each asks `runnable` at the moment it spawns and
// runs the verified copy that hands back (`rule:ide/the-extension-runs-a-copy-of-the-binary`), so
// nothing this extension does ever holds the named file, and nothing it starts is an older build
// than the one on disk: `shadow.ts`'s `Copies` reads the binary's stamp on every call.

import { join } from "node:path";

import { ExtensionContext, workspace } from "vscode";

import { Copies, Shadow, locate } from "./shadow";

let copies: Copies | undefined;
let storage: string | undefined;

/** Where the copies are kept: beside the code, in storage that is this extension's alone. */
export function install(context: ExtensionContext): void {
  storage = context.globalStorageUri.fsPath;
  copies = new Copies(join(storage, "server"));
}

/**
 * The directory `nvs.downloadBinary` unpacks into, which is the third candidate's home.
 *
 * It is beside the copies and not among them: a copy is made from a candidate, and the file here is
 * a candidate, so it is copied before it runs like the other two.
 */
export function installDirectory(): string | undefined {
  return storage === undefined ? undefined : join(storage, "install");
}

/**
 * The command that is `nvs`, as the user named it, for a message that has to say which one.
 *
 * An empty `nvs.path` is a lookup on `PATH`: the command is the bare name, and the platform's own
 * resolution finds it or does not.
 */
export function binary(): string {
  return workspace.getConfiguration("nvs").get<string>("path", "").trim() || "nvs";
}

/** Which of the three candidates a file was found through. */
export type Origin = "nvs.path" | "PATH" | "installed";

/** What to spawn, and what is known about it. */
export interface Runnable {
  /** The file to run: the copy, or the command as named when there is no copy. */
  readonly command: string;
  /** The file the candidate resolved to, or `binary()` itself when no candidate resolved. */
  readonly shown: string;
  /** The resolved file, which is what a caller that outlives one build watches. */
  readonly source: string | undefined;
  /** The candidate `source` was found through, or `undefined` when none resolved. */
  readonly origin: Origin | undefined;
  /** The copy `command` is, when it is one. */
  readonly copy: Shadow | undefined;
  /** Why `command` is the named file although that file exists, which is the case a user is told of. */
  readonly held: string | undefined;
}

/**
 * The file to spawn right now: the first of the three candidates that resolves to a file.
 *
 * It never throws. When no candidate resolves, `binary()` is handed back as it is, so the spawn
 * fails the way it always has and the caller's own message says so; a copy that cannot be made
 * falls back to the named file with the reason in `held`.
 */
export async function runnable(): Promise<Runnable> {
  const configured = workspace.getConfiguration("nvs").get<string>("path", "").trim();
  if (configured !== "") {
    const found = await candidate(configured, "nvs.path");
    if (found !== undefined) {
      return found;
    }
  }
  const onPath = await candidate("nvs", "PATH");
  if (onPath !== undefined) {
    return onPath;
  }
  const managed = await installed();
  if (managed !== undefined) {
    return managed;
  }
  const named = binary();
  return { command: named, shown: named, source: undefined, origin: undefined, copy: undefined, held: undefined };
}

/**
 * The third candidate alone, or `undefined` when nothing is installed.
 *
 * `extension.ts` asks for it when the candidate `runnable` chose did not start or was refused at
 * `initialize`, because "the first that answers" includes the version check.
 */
export async function installed(): Promise<Runnable | undefined> {
  const dir = installDirectory();
  if (dir === undefined) {
    return undefined;
  }
  return candidate(join(dir, process.platform === "win32" ? "nvs.exe" : "nvs"), "installed");
}

async function candidate(named: string, origin: Origin): Promise<Runnable | undefined> {
  const source = await locate(named, {
    path: process.env.PATH,
    pathext: process.env.PATHEXT,
    cwd: workspace.workspaceFolders?.[0]?.uri.fsPath,
    platform: process.platform,
  });
  if (source === undefined) {
    return undefined;
  }
  if (copies === undefined) {
    return { command: source, shown: source, source, origin, copy: undefined, held: undefined };
  }
  try {
    const copy = await copies.of(source);
    return { command: copy.path, shown: source, source, origin, copy, held: undefined };
  } catch (failure) {
    const held = failure instanceof Error ? failure.message : String(failure);
    return { command: source, shown: source, source, origin, copy: undefined, held };
  }
}
