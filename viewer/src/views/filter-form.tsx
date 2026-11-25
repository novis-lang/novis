// The filter form of the stream and the log: the kinds (the stream only), the level, the route and
// the trace id. Applying it opens the view's link with the filter in it, and a new link is a new feed.
//
// What it spends: nothing beyond its inputs.

import { For, Show } from "solid-js";
import { RECORD_KINDS, streamQuery, type RecordKind, type StreamFilter } from "../api.ts";
import { navigate } from "../router.ts";
import { KindDot } from "./feed.tsx";

/** The filter form of the stream and the log: kind (the stream only), level, route and trace. */
export function FilterForm(props: { view: "stream" | "log"; filter: StreamFilter }) {
  let level!: HTMLInputElement;
  let route!: HTMLInputElement;
  let trace!: HTMLInputElement;
  const go = (next: StreamFilter) => {
    // The log's link has no `kind`: the log is always `kind=log`.
    const q = streamQuery(props.view === "log" ? { ...next, kinds: [] } : next);
    navigate(q ? `/${props.view}?${q}` : `/${props.view}`);
  };
  const typed = (): StreamFilter => {
    const next: StreamFilter = { kinds: props.filter.kinds };
    const l = level.value.trim().toLowerCase();
    const r = route.value.trim();
    const t = trace.value.trim().toLowerCase();
    if (l) next.level = l;
    if (r) next.route = r;
    if (t) next.trace = t;
    return next;
  };
  const toggleKind = (kind: RecordKind, on: boolean) => {
    const kinds = RECORD_KINDS.filter((one) => (one === kind ? on : props.filter.kinds.includes(one)));
    go({ ...typed(), kinds });
  };
  return (
    <form
      class="filters"
      aria-label="Filter"
      onSubmit={(e) => {
        e.preventDefault();
        go(typed());
      }}
    >
      <Show when={props.view === "stream"}>
        <fieldset class="kinds">
          <legend>Kinds</legend>
          <For each={RECORD_KINDS}>
            {(kind) => (
              <label class="kind-toggle">
                <input
                  type="checkbox"
                  name="kind"
                  value={kind}
                  checked={props.filter.kinds.includes(kind)}
                  onChange={(e) => toggleKind(kind, e.currentTarget.checked)}
                />
                <KindDot kind={kind} />
                {kind}
              </label>
            )}
          </For>
        </fieldset>
      </Show>
      <label>
        Level <input ref={level} name="level" list="levels" size="8" value={props.filter.level ?? ""} autocomplete="off" />
      </label>
      <datalist id="levels">
        <For each={["debug", "info", "warning", "error", "critical"]}>{(one) => <option value={one} />}</For>
      </datalist>
      <label>
        Route <input ref={route} name="route" size="16" placeholder="/items/{id}" value={props.filter.route ?? ""} autocomplete="off" />
      </label>
      <label>
        Trace id <input ref={trace} name="trace" size="20" value={props.filter.trace ?? ""} autocomplete="off" spellcheck={false} />
      </label>
      <button type="submit" class="btn small">
        Apply
      </button>
    </form>
  );
}
