// The slot client: how a kit job asks looper's service for a heavy-job slot, which spans every project on
// the machine only while the service runs (decision 0027). `docs/kit-contract.md` § *The heavy-job
// slots* is the endpoint, which goal `kit-supervisor` serves.
//
// - looper's home is `LOOPER_HOME`, or else the folder above the `bin` of the `looper.exe` on `PATH`
//   that has `current` beside it, as `looper service install` lays it out. The service's port is the
//   text of `<home>/looper.port`.
// - `takeSlots` asks `POST /api/slots/take` on `127.0.0.1` every `ASK_EVERY_MS` until the service
//   grants a lease of one slot. A job never asks for more than one (decision 0029). While it waits it
//   hands the service's waiting line to `onWait` once, and tells the hang watch of the job it runs in
//   that it waits on purpose (`hang.ts`).
// - The lease is held while the asking process runs. `Lease.hold` names the job that holds it, and
//   `Lease.release` gives it back. The service frees the lease of a process that ended, so no crash
//   leaks one.
// - `seenSlots` reads who holds and who waits for the slots, for `exec list`.
// - `askService` makes the same ask of any other endpoint, as `project add` registers a project. It
//   throws `Absent` where a slot ask gives `absent`.
// - It fails open. With no home, no port file or no service that answers, `takeSlots` gives `absent`
//   at once. A service that answers with something else gives `failed` with the reason. Either way the
//   job runs without a slot.
// - A holder sets `SLOT_VAR` to its lease's id for every process below it, and a job that finds it set
//   takes no slot.
//
// # The bounds it keeps
//
// - One ask holds one answer, read up to `MAX_ANSWER` bytes, and lasts at most `ASK_TIMEOUT_MS`.
// - A wait holds the one lease it asks for, and no history of its asks.

import { existsSync, readFileSync } from "node:fs";
import { delimiter, dirname, isAbsolute, join, resolve } from "node:path";
import { started } from "../os/index.ts";
import { waitOnPurpose } from "./hang.ts";
import type { Take } from "./heavy.ts";
import type { Kind } from "./store.ts";

/** The variable a slot holder sets for every process below it, to its lease's id. */
export const SLOT_VAR = "LOOPER_SLOT";
/** The file in looper's home that names the service's port. */
export const PORT_FILE = "looper.port";
/** How often a waiting job asks again. The service keeps its place while it asks within 10 s. */
export const ASK_EVERY_MS = 500;
/** The longest one ask may take before the service counts as absent. */
export const ASK_TIMEOUT_MS = 5_000;
/** The most bytes of one answer read. */
export const MAX_ANSWER = 64 << 10;

/** Who asks for the slots, as the service shows a waiter. */
export interface Asker {
  kind: Kind;
  project: string | null;
  command: string;
}

/** A lease of slots the service granted. */
export interface Lease {
  id: string;
  /** The indexes of the slots it holds. */
  slots: number[];
  /** Names `job` as the lease's holder. A failure is ignored. */
  hold(job: string): Promise<void>;
  /** Gives the slots back. A failure is ignored, since the service frees them once this process ends. */
  release(): Promise<void>;
}

/** What an ask came to: a lease, no service, or a service whose answer could not be used. */
export type Asked = { lease: Lease } | { absent: string } | { failed: string };

/** looper's home in `env`, or null when there is none. */
export function looperHome(env: Record<string, string | undefined> = process.env): string | null {
  const named = env.LOOPER_HOME;
  if (named !== undefined && named !== "") return isAbsolute(named) ? named : resolve(named);
  for (const dir of (env.PATH ?? env.Path ?? "").split(delimiter)) {
    if (dir === "") continue;
    if (existsSync(join(dir, "looper.exe")) && existsSync(join(dir, "current"))) return dirname(resolve(dir));
  }
  return null;
}

/** The service's port that `home` names, or why there is none. */
function portOf(home: string | null): number | string {
  if (home === null) return "looper's home is not found";
  let text: string;
  try {
    text = readFileSync(join(home, PORT_FILE), "utf8").trim();
  } catch {
    return `${join(home, PORT_FILE)} does not exist`;
  }
  const port = Number(text);
  return Number.isInteger(port) && port > 0 && port < 65_536 ? port : `${join(home, PORT_FILE)} names no port`;
}

/** Posts `body` to `path` of the service on `port`, or gets `path` without one, and gives its JSON answer. Throws `Absent` when nothing answers. */
async function post(port: number, path: string, body?: unknown): Promise<unknown> {
  let res: Response;
  try {
    res = await fetch(`http://127.0.0.1:${port}${path}`, {
      method: body === undefined ? "GET" : "POST",
      ...(body === undefined ? {} : { headers: { "content-type": "application/json" }, body: JSON.stringify(body) }),
      signal: AbortSignal.timeout(ASK_TIMEOUT_MS),
    });
  } catch (e) {
    throw new Absent(`the service on port ${port} does not answer: ${(e as Error).message}`);
  }
  const bytes = new Uint8Array(await res.arrayBuffer());
  if (bytes.length > MAX_ANSWER) throw new Error(`the service's answer passed ${MAX_ANSWER} bytes`);
  if (!res.ok) {
    let why = "";
    try {
      const error = (JSON.parse(new TextDecoder().decode(bytes)) as { error?: unknown }).error;
      if (typeof error === "string") why = `: ${error}`;
    } catch {}
    throw new Error(`the service answered ${res.status}${why}`);
  }
  return JSON.parse(new TextDecoder().decode(bytes));
}

/** No service answers: there is no home, no port file, or nothing on the port. */
export class Absent extends Error {}

/** Gets `path` of the service of `home`, or posts `body` to it, and gives its JSON answer. Throws `Absent` when nothing answers. */
export async function askService(home: string | null, path: string, body?: unknown): Promise<unknown> {
  const port = portOf(home);
  if (typeof port === "string") throw new Absent(port);
  return post(port, path, body);
}

/**
 * Asks the service of `home` for the slot `take` names, for `asker`, until it grants it. A wait
 * hands the service's waiting line to `onWait` once. Gives `absent` or `failed` at once for `none`.
 */
export async function takeSlots(home: string | null, take: Take, asker: Asker, onWait: (line: string) => void): Promise<Asked> {
  if (take === "none") return { absent: "the command takes no slot" };
  const port = portOf(home);
  if (typeof port === "string") return { absent: port };
  const body = { pid: process.pid, started: started(process.pid), ...asker };
  let waiting: (() => void) | null = null;
  try {
    for (;;) {
      let answer: unknown;
      try {
        answer = await post(port, "/api/slots/take", body);
      } catch (e) {
        return e instanceof Absent ? { absent: e.message } : { failed: (e as Error).message };
      }
      const a = answer as { lease?: unknown; slots?: unknown; waiting?: unknown };
      if (typeof a.lease === "string") {
        const slots = Array.isArray(a.slots) ? a.slots.filter((s): s is number => typeof s === "number") : [];
        return { lease: lease(port, a.lease, slots) };
      }
      if (a.lease !== null) return { failed: "the service's answer names no lease" };
      if (waiting === null) {
        onWait(typeof a.waiting === "string" ? a.waiting : "looper: waiting for a heavy slot");
        waiting = waitOnPurpose("a heavy slot");
      }
      await Bun.sleep(ASK_EVERY_MS);
    }
  } finally {
    waiting?.();
  }
}

/** Who holds and who waits for the service's slots, as `GET /api/slots` gives them. */
export interface Seen {
  count: number;
  /** Each held slot: its index, its lease and the job the lease names, null before `hold`. */
  held: { index: number; lease: string; job: string | null }[];
  /** The askers that wait, the longest waiting first. */
  waiting: { pid: number; since: number; kind: string; project: string | null; command: string }[];
}

/** The slots of the service of `home`, or null when no service answers with them. */
export async function seenSlots(home: string | null): Promise<Seen | null> {
  const port = portOf(home);
  if (typeof port === "string") return null;
  try {
    const seen = (await post(port, "/api/slots")) as Seen;
    return Number.isInteger(seen.count) && Array.isArray(seen.held) && Array.isArray(seen.waiting) ? seen : null;
  } catch {
    return null;
  }
}

/** The lease `id` of the service on `port`, holding `slots`. */
function lease(port: number, id: string, slots: number[]): Lease {
  const at = `/api/slots/${encodeURIComponent(id)}`;
  return {
    id,
    slots,
    hold: async (job) => {
      try {
        await post(port, `${at}/hold`, { job });
      } catch {}
    },
    release: async () => {
      try {
        await post(port, `${at}/release`, {});
      } catch {}
    },
  };
}
