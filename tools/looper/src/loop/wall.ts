// The usage wall. A wall belongs to the agent's account, not to a loop. It ports
// `crates/looper-engine/src/wall.rs`, with a state file where looper has a store table.
//
// - When a session ends on a wall, `note` records it. A wall never moves earlier: a later report that
//   resets sooner keeps the reset already on record.
// - Before every session, each loop that reads the same file reads it with `standing`, and holds until
//   `clearsAt`, the reset plus `MARGIN`. A running session is never killed for a wall.
// - `lift` ends a wall early, which is what `retry-now` does.
//
// A wall the agent stated only in words carries no reset time, so it is recorded as resetting
// `BLIND_WAIT` after it was seen. Times are ms since the Unix epoch.
//
// The file is a state file (`../store/state.ts`), `{"<account>": {"resetsAt", "kind"}}`. Every loop
// that names the same file holds on the same walls. Without the service, that is the loops of one
// project. One writer writes at a time: two loops that note a wall in the same instant may lose one
// note, and the loop that lost it meets the wall again in its next session.
//
// The walls span every project only while looper's service runs (decision 0027), so `Walls` given
// looper's home also shares them through the service. `docs/kit-contract.md` § *The usage wall* is the
// endpoint.
//
// - `pull` reads the wall the service holds on the account, and notes it in the file. So a wall another
//   project's loop met holds this loop too. When the service answers that it holds none, `pull` ends
//   the file's wall, so a `retry-now` in one project lifts the wall in every project.
// - A wall noted while the service did not answer is ended by the next `pull` that reaches it. The
//   loop then meets that wall again in its next session, and reports it.
// - `report` sends a wall this loop noted, and `liftShared` ends the account's wall on the service as
//   well as in the file.
// - Each fails open: with no home or no service that answers, the file alone holds the walls.
//
// # The bounds it keeps
//
// The file holds one entry per account, and `note` drops every wall that has cleared. One ask of the
// service holds one answer, as `askService` bounds it.

import { askService } from "../jobs/slots.ts";
import { readState, writeState } from "../store/state.ts";

/** How long after a wall's reset sessions start again, in ms. */
export const MARGIN = 60_000;
/** How long a wall stated only in words is waited out, in ms. */
export const BLIND_WAIT = 30 * 60_000;

/** One account's wall: when it resets and which window it is, such as `five_hour`. */
export interface Wall {
  resetsAt: number;
  kind: string;
}

/** When sessions on the wall's account may start again. */
export function clearsAt(w: Wall): number {
  return w.resetsAt + MARGIN;
}

/** The walls on record in one state file, shared through the service of looper's `home` when one is given. */
export class Walls {
  constructor(
    readonly path: string,
    readonly home: string | null = null,
  ) {}

  /** Notes the wall the service holds on `account` at `now`, or ends the file's wall when the service holds none. */
  async pull(account: string, now: number): Promise<void> {
    if (this.home === null) return;
    let answer: unknown;
    try {
      answer = await askService(this.home, `/api/walls/${encodeURIComponent(account)}`);
    } catch {
      return;
    }
    const w = (answer as { wall?: { resets_at?: unknown; kind?: unknown } | null }).wall;
    if (w === null) {
      this.lift(account);
      return;
    }
    if (w === undefined || typeof w.resets_at !== "number") return;
    this.keep(account, { resetsAt: w.resets_at, kind: typeof w.kind === "string" ? w.kind : "" }, now);
  }

  /** Sends the wall `w` on `account` to the service. */
  async report(account: string, w: Wall): Promise<void> {
    if (this.home === null) return;
    try {
      await askService(this.home, `/api/walls/${encodeURIComponent(account)}`, { resets_at: w.resetsAt, kind: w.kind });
    } catch {}
  }

  /** Ends the wall on `account` now, in the file and on the service. */
  async liftShared(account: string): Promise<void> {
    this.lift(account);
    if (this.home === null) return;
    try {
      await askService(this.home, `/api/walls/${encodeURIComponent(account)}/lift`, {});
    } catch {}
  }

  /** The wall that holds `account` at `now`, if one does. */
  standing(account: string, now: number): Wall | null {
    const w = this.read()[account];
    return w !== undefined && clearsAt(w) > now ? w : null;
  }

  /**
   * Records a wall on `account` seen at `now`. `resetsAt` is the agent's unix seconds, or null for a
   * wall stated only in words. Gives the wall now on record.
   */
  note(account: string, resetsAt: number | null, kind: string, now: number): Wall {
    const resets = resetsAt === null ? now + BLIND_WAIT : resetsAt * 1000;
    return this.keep(account, { resetsAt: resets, kind }, now);
  }

  /** Records `wall` on `account` unless one that resets as late is on record. Gives the wall now on record. */
  private keep(account: string, wall: Wall, now: number): Wall {
    const all = this.read();
    const on = all[account];
    if (on !== undefined && on.resetsAt >= wall.resetsAt) return on;
    const kept: Record<string, Wall> = {};
    for (const [a, w] of Object.entries(all)) if (clearsAt(w) > now) kept[a] = w;
    kept[account] = wall;
    writeState(this.path, kept);
    return wall;
  }

  /** Ends the wall on `account` now. */
  lift(account: string): void {
    const all = this.read();
    if (all[account] === undefined) return;
    delete all[account];
    writeState(this.path, all);
  }

  private read(): Record<string, Wall> {
    return readState<Record<string, Wall>>(this.path) ?? {};
  }
}
