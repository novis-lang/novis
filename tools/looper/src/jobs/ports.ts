// A launch's port: who holds a port now, the next free one, and the port and args each launcher was
// last started with. It ports `crates/looper-job/src/ports.rs`.
//
// - `readTaken` reads, once, what holds ports: the project's live launches (`launch.ts`), looper's
//   service from `<looper home>/looper.port`, and the OS's listening TCP ports with their pids.
//   `Taken.owner` names who holds a port, a launch first and a process last, and `Taken.nextFree`
//   finds the first port above one that nobody holds. `Taken.clash` gives both for a held port.
// - `lockPorts` takes the machine's port mutex. A start holds it from its check until its `launch.json`
//   names the port, so two starts on one port cannot both pass the check.
// - `.looper/launch/ports.json` maps each launcher's name to its last `Chosen` port and args.
//   `remember` writes one launcher's, and drops the entries of launchers that `looper.toml` no longer
//   declares. `remembered` reads one.
//
// Another project's launch that has not yet bound its port is not seen, since only its own project
// lists it. Its port shows once its program listens.
//
// # The bounds it keeps
//
// - `ports.json` holds one entry per declared launcher. `remember` refuses an args text longer than
//   `ARGS_MAX` bytes.
// - A `Taken` holds one entry per live launch and per listening socket of the machine.

import { mkdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { imageName, listeners, type Mutex, openMutex } from "../os/index.ts";
import { readState, writeState } from "../store/state.ts";
import { liveLaunches } from "./launch.ts";
import { looperHome, PORT_FILE } from "./slots.ts";
import type { Store } from "./store.ts";

/** The most bytes of a launcher's args text. */
export const ARGS_MAX = 1024;

/** The name of the mutex every start on the machine takes from its check until it names its port. */
export const PORTS_MUTEX = "Local\\looper-kit-ports";

/** How often a start that waits for the port mutex tries again. */
const LOCK_EVERY_MS = 20;

/** The longest a start waits for the port mutex. */
const LOCK_WAIT_MS = 30_000;

/** Who holds a port. */
export type Owner =
  | { kind: "launch"; project: string; name: string; job: string }
  | { kind: "looper" }
  | { kind: "process"; pid: number; name: string | null };

/** `owner` as a person reads it. */
export function ownerText(owner: Owner): string {
  switch (owner.kind) {
    case "launch":
      return `${owner.project}/${owner.name}`;
    case "looper":
      return "looper's own server";
    case "process":
      return owner.name === null ? `pid ${owner.pid}` : `${owner.name} (pid ${owner.pid})`;
  }
}

/** A port someone holds: who, and the next free port above it. */
export interface Clash {
  port: number;
  owner: Owner;
  next: number | null;
}

/** `clash` as the refusal of a start says it. */
export function clashText(clash: Clash): string {
  const next = clash.next === null ? "no port above it is free" : `the next free port is ${clash.next}`;
  return `port ${clash.port} is used by ${ownerText(clash.owner)}; ${next}`;
}

/** What held ports when it was read. */
export class Taken {
  constructor(
    readonly launches: [number, Owner][] = [],
    readonly looper: number | null = null,
    readonly listening: [number, number][] = [],
  ) {}

  /** Who holds `port`, or null when it is free. */
  owner(port: number): Owner | null {
    const launch = this.launches.find(([p]) => p === port);
    if (launch) return launch[1];
    if (this.looper === port) return { kind: "looper" };
    const listener = this.listening.find(([p]) => p === port);
    return listener ? { kind: "process", pid: listener[1], name: imageName(listener[1]) } : null;
  }

  /** Whether nobody holds `port`. */
  free(port: number): boolean {
    return !this.launches.some(([p]) => p === port) && this.looper !== port && !this.listening.some(([p]) => p === port);
  }

  /** The first port above `port` that nobody holds, or null when every one up to 65535 is held. */
  nextFree(port: number): number | null {
    for (let p = port + 1; p <= 65_535; p++) if (this.free(p)) return p;
    return null;
  }

  /** Who holds `port` and the next free port above it, or null when it is free. */
  clash(port: number): Clash | null {
    const owner = this.owner(port);
    return owner === null ? null : { port, owner, next: this.nextFree(port) };
  }
}

/** Reads what holds ports now, for the project at `root` whose job store is `store`. */
export function readTaken(store: Store, root: string): Taken {
  const launches: [number, Owner][] = [];
  for (const l of liveLaunches(store)) {
    if (l.launch.port === null) continue;
    const project = l.launch.project === "" ? basename(root) : l.launch.project;
    launches.push([l.launch.port, { kind: "launch", project, name: l.launch.name, job: l.id }]);
  }
  return new Taken(launches, looperPort(), listeners());
}

/** The port looper's service names in its home, or null. */
function looperPort(): number | null {
  const home = looperHome();
  if (home === null) return null;
  try {
    const port = Number(readFileSync(join(home, PORT_FILE), "utf8").trim());
    return Number.isInteger(port) && port > 0 ? port : null;
  } catch {
    return null;
  }
}

/** Takes the machine's port mutex, waiting while another start holds it. Close it to release it. */
export async function lockPorts(): Promise<Mutex> {
  const m = openMutex(PORTS_MUTEX);
  const until = Date.now() + LOCK_WAIT_MS;
  while (!m.tryTake()) {
    if (Date.now() > until) {
      m.close();
      throw new Error(`another start held the port lock for ${LOCK_WAIT_MS / 1000} s`);
    }
    await Bun.sleep(LOCK_EVERY_MS);
  }
  return m;
}

/** A launcher's last chosen port and args. */
export interface Chosen {
  port?: number;
  args?: string;
}

/** The file of each launcher's last `Chosen`, in the project at `root`. */
export function portsFile(root: string): string {
  return join(root, ".looper", "launch", "ports.json");
}

function readAll(root: string): Record<string, Chosen> {
  try {
    const all = readState<Record<string, Chosen>>(portsFile(root));
    return all !== null && typeof all === "object" && !Array.isArray(all) ? all : {};
  } catch {
    return {};
  }
}

/** The last `Chosen` of launcher `name` in the project at `root`. */
export function remembered(root: string, name: string): Chosen | undefined {
  return readAll(root)[name];
}

/** Remembers `chosen` for launcher `name`, and forgets each launcher not in `declared`. An args text past `ARGS_MAX` is refused. */
export function remember(root: string, declared: readonly string[], name: string, chosen: Chosen): void {
  if (chosen.args !== undefined && Buffer.byteLength(chosen.args) > ARGS_MAX) throw new Error(`an args text is at most ${ARGS_MAX} bytes`);
  const all = readAll(root);
  const kept: Record<string, Chosen> = {};
  for (const [n, c] of Object.entries(all)) if (declared.includes(n)) kept[n] = c;
  if (declared.includes(name)) kept[name] = chosen;
  mkdirSync(join(root, ".looper", "launch"), { recursive: true });
  writeState(portsFile(root), kept);
}
