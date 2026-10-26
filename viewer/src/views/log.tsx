// The log at `/log?<filter>`: the stream's feed with `kind=log`, filtered by level, route and trace,
// with the filter in the link. Each line shows its level, its message, its fields and the request it
// belongs to.
//
// What it spends: what the stream's feed spends (`stream.tsx`).

import { For } from "solid-js";
import { streamFilter } from "../api.ts";
import { query } from "../router.ts";
import { SaveButton } from "./saved.tsx";
import { clock, Feed, fieldsLine, FilterForm, OwnerCell } from "./stream.tsx";

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
      <For each={[query()]}>
        {(q) => (
          <Feed
            filter={logFilter(q)}
            label="Log lines, newest first"
            noun={["log line", "log lines"]}
            empty="No log lines match this filter. New lines show here when your program writes them."
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
                <span class="log-level" classList={{ bad: row.level === "error" || row.level === "critical", s4: row.level === "warning" }}>
                  {row.level}
                </span>
                <span class="text" title={row.text}>
                  {row.text}
                </span>
                <span class="note log-fields" title={fieldsLine(row.fields)}>
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
