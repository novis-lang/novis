// The Cache tab of a request's page: its cache calls grouped by key and store, the key with the
// most calls first. Each row has the number of calls, the hits and misses of its `get` calls, and
// the time of all its calls. A call whose key is a secret has no key, and its row says so.
// What it spends: one group object per distinct key and store, for as long as the tab is open.

import { createMemo, For, Show } from "solid-js";
import type { Records } from "../../api.ts";
import { Empty } from "../../ui/empty.tsx";
import { duration } from "../requests.tsx";
import { cacheGroups } from "./outcome.ts";

const TIER_NAMES: Record<string, string> = { memory: "Memory", redis: "Shared" };

export function Cache(props: { detail: Records }) {
  const groups = createMemo(() => cacheGroups(props.detail));
  return (
    <section class="rt cache" aria-label="Cache">
      <Show when={groups().length > 0} fallback={<Empty icon="queries" title="This request made no cache calls." />}>
        <table class="rt-table rt-cache" aria-label="Cache calls by key">
          <thead>
            <tr>
              <th scope="col">Key</th>
              <th scope="col">Store</th>
              <th scope="col" class="rt-num">
                Calls
              </th>
              <th scope="col" class="rt-num">
                Hits
              </th>
              <th scope="col" class="rt-num">
                Misses
              </th>
              <th scope="col" class="rt-num">
                Time
              </th>
            </tr>
          </thead>
          <tbody>
            <For each={groups()}>
              {(g) => (
                <tr class="cache-group">
                  <td class="rt-code">{g.key ?? <span class="note">A secret's key, which is not recorded</span>}</td>
                  <td>{TIER_NAMES[g.tier] ?? "–"}</td>
                  <td class="rt-num">{g.count}</td>
                  <td class="rt-num">{g.hits}</td>
                  <td class="rt-num">{g.misses}</td>
                  <td class="rt-num">{duration(g.total_us)}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>
      <Show when={props.detail.cut.events}>
        <p class="note cut-note">The list of events stopped at its limit, so some cache calls are not shown.</p>
      </Show>
    </section>
  );
}
