// The log at `/log?<filter>`: the record feed (`feed.tsx`) with `kind=log`, filtered by level, route
// and trace, with the filter in the link. Each line shows its time, its level as a coloured chip, its
// message, its fields and the request it belongs to.
//
// What it spends: what the record feed spends (`feed.tsx`).

import { For } from "solid-js";
import { streamFilter } from "../api.ts";
import { query } from "../router.ts";
import { clock, Feed, fieldsLine, LevelChip, OwnerCell } from "./feed.tsx";
import { FilterForm } from "./filter-form.tsx";
import { SaveButton } from "./saved.tsx";

/** The log's filter: the link's level, route and trace, and the one kind `log`. */
function logFilter(q: string) {
  return { ...streamFilter(q), kinds: ["log" as const] };
}

export function LogView() {
  return (
    <section class="stream log-view" aria-label="Log">
      <div class="toolbar">
        <FilterForm view="log" filter={logFilter(query())} />
        <SaveButton view="log" query={query} />
      </div>
      {/* A new link is a new filter, and a new filter is a new feed. */}
      <For each={[query()]}>
        {(q) => (
          <Feed
            filter={logFilter(q)}
            label="Log lines, newest first"
            noun={["log line", "log lines"]}
            empty={{
              icon: "logs",
              title: "No log lines match this filter",
              text: "New lines show here when your program writes them.",
            }}
            grid="log-grid"
            cols={
              <>
                <span>Time</span>
                <span>Level</span>
                <span>Message</span>
                <span>Fields</span>
                <span>Request</span>
              </>
            }
            cells={(row) => (
              <>
                <span class="at">{clock(row.at_us)}</span>
                <span class="level-cell">{row.level ? <LevelChip level={row.level} /> : null}</span>
                <span class="text log-message" title={row.text}>
                  {row.text}
                </span>
                <span class="log-fields" title={fieldsLine(row.fields)}>
                  {fieldsLine(row.fields)}
                </span>
                <OwnerCell record={row} />
              </>
            )}
          />
        )}
      </For>
    </section>
  );
}
