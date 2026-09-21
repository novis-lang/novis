// Which `nvs` answers, and which file is run to get it, for every process this extension starts.
//
// It has one home because there are five spawns — the server in `extension.ts`, the Tasks, the
// formatter, the AST panel and the test controller — and a user who points `nvs.path` at a build
// directory means all of them. A second read with its own fallback is how one of them ends up
// running a different binary than the others.
//
// None of the five runs the file the user named. Each asks `runnable` at the moment it spawns and
// runs the verified copy that hands back (`rule:ide/the-extension-runs-a-copy-of-the-binary`), so
// nothing this extension does ever holds the named file, and nothing it starts is an older build
// than the one on disk: `shadow.ts`'s `Copies` reads the binary's stamp on every call.

import { join } from "node:path";

import { ExtensionContext, workspace } from "vscode";

import { Copies, Shadow, locate } from "./shadow";

let copies: Copies | undefined;

/** Where the copies are kept: beside the code, in storage that is this extension's alone. */
export function install(context: ExtensionContext): void {
  copies = new Copies(join(context.globalStorageUri.fsPath, "server"));
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

/** What to spawn, and what is known about it. */
export interface Runnable {
  /** The file to run: the copy, or the command as named when there is no copy. */
  readonly command: string;
  /** The file `binary()` resolved to, or that name itself when it resolved to nothing. */
  readonly shown: string;
  /** The resolved file, which is what a caller that outlives one build watches. */
  readonly source: string | undefined;
  /** The copy `command` is, when it is one. */
  readonly copy: Shadow | undefined;
  /** Why `command` is the named file although that file exists, which is the case a user is told of. */
  readonly held: string | undefined;
}

/**
 * The file to spawn right now.
 *
 * It never throws. A name that resolves to no file is handed back as it is, so the spawn fails the
 * way it always has and the caller's own message says so; a copy that cannot be made falls back to
 * the named file with the reason in `held`.
 */
export async function runnable(): Promise<Runnable> {
  const named = binary();
  const source = await locate(named, {
    path: process.env.PATH,
    pathext: process.env.PATHEXT,
    cwd: workspace.workspaceFolders?.[0]?.uri.fsPath,
    platform: process.platform,
  });
  if (source === undefined || copies === undefined) {
    return { command: named, shown: source ?? named, source, copy: undefined, held: undefined };
  }
  try {
    const copy = await copies.of(source);
    return { command: copy.path, shown: source, source, copy, held: undefined };
  } catch (failure) {
    const held = failure instanceof Error ? failure.message : String(failure);
    return { command: named, shown: source, source, copy: undefined, held };
  }
}
