// What a trace event came to, as the short text the timeline shows after its label: a cache call's
// store, an outbound call's status and sizes, a file call's bytes, a process's name and exit code, sent
// mail's recipient count, and a pushed job's script, queue and delay.
// A cache `get`'s hit or miss is a chip of its own (`Hit` in `../timeline.tsx`), so it is not in the text.
// The Cache tab's groups are here too (`cacheGroups`), so a unit test reads them without the page.

import type { RequestDetail, TraceEvent } from "../../api.ts";
import { delay, size } from "../../format.ts";

/** The cache calls of one key on one store. */
export interface CacheGroup {
  /** The key, or `null` for a call whose key is a secret. */
  key: string | null;
  tier: string;
  count: number;
  hits: number;
  misses: number;
  /** The time of all its calls, in microseconds. */
  total_us: number;
}

/** The request's cache events in groups by key and store, most calls first. */
export function cacheGroups(detail: Pick<RequestDetail, "events">): CacheGroup[] {
  const groups = new Map<string, CacheGroup>();
  for (const event of detail.events) {
    if (event.kind !== "cache") continue;
    const key = event.target ?? null;
    const tier = event.outcome?.tier ?? "";
    const id = `${tier}\u0000${key ?? "\u0001"}`;
    let group = groups.get(id);
    if (!group) {
      group = { key, tier, count: 0, hits: 0, misses: 0, total_us: 0 };
      groups.set(id, group);
    }
    group.count += 1;
    if (event.outcome?.hit === true) group.hits += 1;
    if (event.outcome?.hit === false) group.misses += 1;
    group.total_us += Math.max(0, event.end_us - event.start_us);
  }
  return [...groups.values()].sort((a, b) => b.count - a.count || b.total_us - a.total_us);
}

/** The outcome of `event` as text, such as `503 · 17 B in · 4 B out`, or `""` when it has none. */
export function outcomeText(event: TraceEvent): string {
  const o = event.outcome;
  if (!o) return "";
  const parts: string[] = [];
  if (o.tier) parts.push(o.tier === "redis" ? "shared store" : "memory store");
  if (o.http_status !== undefined) parts.push(String(o.http_status));
  if (o.bytes_in !== undefined) parts.push(`${size(o.bytes_in)} in`);
  if (o.bytes_out !== undefined) parts.push(`${size(o.bytes_out)} out`);
  if (o.bytes !== undefined) parts.push(size(o.bytes));
  if (o.command !== undefined) parts.push(o.command);
  if (o.exit !== undefined) parts.push(`exit code ${o.exit}`);
  if (o.recipients !== undefined) parts.push(o.recipients === 1 ? "1 recipient" : `${o.recipients} recipients`);
  if (o.ok === false) parts.push("not sent");
  if (o.job !== undefined) parts.push(o.job);
  if (o.queue !== undefined) parts.push(`queue ${o.queue}`);
  if (o.delay_ms !== undefined) parts.push(`due in ${delay(o.delay_ms)}`);
  return parts.join(" · ");
}
