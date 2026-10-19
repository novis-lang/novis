// The kit's one door to the operating system: process trees and their named jobs, detached processes,
// signals, what the kit reads of a process by its pid, and a volume's free space. Every other module imports this file, never `windows.ts`.
//
// - Windows is the only OS it serves. `windows.ts` holds every call, with the desktop's calls in
//   `windows-desktop.ts`, and their head comments own the rules.
// - On another OS each function refuses with an error that names the OS. A port adds one file beside
//   `windows.ts` and picks it here, and nothing else in the kit changes.
// - `sameProcess` is how the kit tells whether a recorded pid still names the process it recorded: the
//   pid must run and must have started at the recorded time.

import * as windows from "./windows.ts";

export type { Detached, Mutex, SpawnOptions, Tree, TreeOptions } from "./windows.ts";
export { MAX_PROCS, READ_BYTES } from "./windows.ts";
export type { WindowWatcher } from "./windows-desktop.ts";
export { OFF } from "./windows-desktop.ts";

function serve(): typeof windows {
  if (process.platform === "win32") return windows;
  throw new Error(`the kit's OS module does not run on ${process.platform}`);
}

/** Starts `cmd` as a tree in a fresh Job Object that kills it on close. */
export function spawnTree(cmd: string[], opts?: windows.TreeOptions): windows.Tree {
  return serve().spawnTree(cmd, opts);
}

/** Whether a job holds the name `name`. */
export function jobExists(name: string): boolean {
  return serve().jobExists(name);
}

/** Terminates every process of the job named `name`, each with exit code `code`. Says whether it did. */
export function killJob(name: string, code?: number): boolean {
  return serve().killJob(name, code);
}

/** `arg` as one word of a command line, quoted as the C runtime reads it back. */
export function quote(arg: string): string {
  return serve().quote(arg);
}

/** Starts `cmd` with no handle of this process, in a group of its own, out of this job when it may. */
export function spawnDetached(cmd: string[], opts?: windows.SpawnOptions): windows.Detached {
  return serve().spawnDetached(cmd, opts);
}

/** Sends Ctrl+Break to the process group `group`, which must share this process's console. */
export function ctrlBreak(group: number): void {
  serve().ctrlBreak(group);
}

/** Whether the process `pid` is running. */
export function alive(pid: number): boolean {
  return serve().alive(pid);
}

/** When the running process `pid` started, in ms since the Unix epoch, or null when it does not run. */
export function started(pid: number): number | null {
  return serve().started(pid);
}

/** Whether `pid` runs and is the process that started at `at`, not a later one given the same pid. */
export function sameProcess(pid: number, at: number): boolean {
  return serve().started(pid) === at;
}

/** Terminates the running process `pid` alone. Says whether it did. */
export function terminate(pid: number, code?: number): boolean {
  return serve().terminate(pid, code);
}

/** The pids of the processes in the job named `name`, empty when no job holds the name. */
export function jobPids(name: string): number[] {
  return serve().jobPids(name);
}

/** Opens the named mutex `name`, which a crash of its holder releases. */
export function openMutex(name: string): windows.Mutex {
  return serve().Mutex.open(name);
}

/** Every listening TCP port with its pid, sorted by port. */
export function listeners(): [number, number][] {
  return serve().listeners();
}

/** The file name of the program process `pid` runs, or null when it cannot be read. */
export function imageName(pid: number): string | null {
  return serve().imageName(pid);
}

/** One GiB, the unit of `[loop] min_free_gb`. */
export const GIB = 2 ** 30;

/** The bytes free to this user on the volume that holds `path`, which the worker's disk gate reads. Throws when the path names no volume. */
export function freeBytes(path: string): number {
  return serve().freeBytes(path);
}

/** Shows no dialog for a critical error of this process, or of each process it starts from now on. */
export function noErrorBoxes(): void {
  serve().noErrorBoxes();
}

/** Terminates the running process `pid` and every process below it. Says whether it did. */
export function killTree(pid: number): boolean {
  return serve().killTree(pid);
}
