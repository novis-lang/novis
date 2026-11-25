// The Queries tab of a request's page: its statements grouped by call site and SQL text, the slowest
// group first. A group that ran two or more times has a gold count and a gold row, and a note above
// the table names the site with the most repeats.
// What it spends: one group object per distinct site and statement, for as long as the tab is open.

import { createMemo, For, Show } from "solid-js";
import type { EventQuery, RequestDetail } from "../../api.ts";
import { Repeat } from "../../ui/badges.tsx";
import { Empty } from "../../ui/empty.tsx";
import { Site } from "../../ui/site.tsx";
import { duration } from "../requests.tsx";

/** The queries one call site sent with the same SQL text. */
interface Group {
  sql: string;
  site: EventQuery["site"];
  driver: string;
  count: number;
  /** The time of all its queries, in microseconds. */
  total_us: number;
  /** The rows summed, or `null` when no query of the group reported rows. */
  rows: number | null;
}

/** The request's query events in groups, by total time, largest first. A query with no site is
 *  grouped by its SQL text alone. */
export function queryGroups(detail: RequestDetail): Group[] {
  const groups = new Map<string, Group>();
  for (const event of detail.events) {
    const q = event.query;
    if (event.kind !== "query" || q === null) continue;
    const key = `${q.site ? `${q.site.file}:${q.site.line}` : ""}\u0000${q.sql}`;
    let group = groups.get(key);
    if (!group) {
      group = { sql: q.sql, site: q.site, driver: q.driver, count: 0, total_us: 0, rows: null };
      groups.set(key, group);
    }
    group.count += 1;
    group.total_us += Math.max(0, event.end_us - event.start_us);
    if (q.rows !== null) group.rows = (group.rows ?? 0) + q.rows;
  }
  return [...groups.values()].sort((a, b) => b.total_us - a.total_us);
}

export function Queries(props: { detail: RequestDetail }) {
  const groups = createMemo(() => queryGroups(props.detail));
  /** The group with the most repeats, when one ran two or more times. */
  const worst = () => groups().reduce<Group | null>((top, g) => (g.count >= 2 && (!top || g.count > top.count) ? g : top), null);
  return (
    <section class="rt queries" aria-label="Queries">
      <Show when={groups().length > 0} fallback={<Empty icon="queries" title="This request sent no database queries." />}>
        <Show when={worst()}>
          {(g) => (
            <p class="note rt-repeat-note" role="note">
              <Show when={g().site} fallback={<>The same query ran {g().count} times.</>}>
                {(site) => (
                  <>
                    The same query ran {g().count} times at <Site site={site()} />.
                  </>
                )}
              </Show>
            </p>
          )}
        </Show>
        <table class="rt-table rt-queries" aria-label="Queries by call site">
          <thead>
            <tr>
              <th scope="col">Query</th>
              <th scope="col" class="rt-num">
                Count
              </th>
              <th scope="col" class="rt-num">
                Time
              </th>
              <th scope="col" class="rt-num">
                Rows
              </th>
              <th scope="col">Driver</th>
              <th scope="col">Site</th>
            </tr>
          </thead>
          <tbody>
            <For each={groups()}>
              {(g) => (
                <tr class="query-group" classList={{ repeated: g.count >= 2 }}>
                  <td class="rt-code rt-sql">{g.sql}</td>
                  <td class="rt-num">
                    <Show when={g.count >= 2} fallback={<>1</>}>
                      <Repeat count={g.count} />
                    </Show>
                  </td>
                  <td class="rt-num">{duration(g.total_us)}</td>
                  <td class="rt-num">{g.rows === null ? "–" : String(g.rows)}</td>
                  <td>{g.driver}</td>
                  <td>
                    <Show when={g.site} fallback={<span class="note">–</span>}>
                      {(site) => <Site site={site()} />}
                    </Show>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>
      <Show when={props.detail.cut.events}>
        <p class="note cut-note">The list of events stopped at its limit, so some queries are not shown.</p>
      </Show>
    </section>
  );
}
