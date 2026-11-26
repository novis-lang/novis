// One run, at `/runs/<trace id>`: a scheduled job, a queue job or a command, and everything it
// recorded, from `GET /api/run`.
//
// The header has what started the run, its name and how it ended, a line with its age, its
// exception and its trace id, the tiles, and its time split by kind with its legend. When the index
// has deleted records the run may have had, one note under the header gives the time before which
// they were deleted (`cut_us`). Below it are
// the request page's tabs without Request, Response and Coverage: Timeline, Queries, Cache, Dumps,
// Logs, and Exception when the run threw. The tab is part of the link (`?tab=queries`). Escape goes
// back to the run list.
//
// What it spends: the one run's records, which the service already limited, for as long as the view
// is open.

import { createResource, For, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import { ApiError, run, type RunDetail, type RunRow } from "../api.ts";
import { ignored } from "../keys.ts";
import { navigate, query, runLink } from "../router.ts";
import { Site } from "../ui/site.tsx";
import { type Tab, Tabs } from "../ui/tabs.tsx";
import { Meter, Tile } from "../ui/tile.tsx";
import { Dumps, ExceptionPanel, focusHash, Logs } from "./request.tsx";
import { Cache } from "./request/cache.tsx";
import { Queries } from "./request/queries.tsx";
import { age, createNow, duration, MemoryTile, split } from "./requests.tsx";
import { OF_LABEL, Outcome, runListLink } from "./runs.tsx";
import { Timeline } from "./timeline.tsx";

/** The tabs of a run's page, in the order the tab row shows them. */
const RUN_TABS = ["timeline", "queries", "cache", "dumps", "logs", "exception"] as const;
type RunTab = (typeof RUN_TABS)[number];

/** The link to one tab of a run's page. The Timeline tab is the page's own link. */
function runTabLink(row: Pick<RunRow, "trace">, tab: RunTab, hash = ""): string {
  const base = runLink(row);
  return `${tab === "timeline" ? base : `${base}?tab=${tab}`}${hash}`;
}

/** The tab the link names, or the Timeline. */
function currentTab(row: RunRow): RunTab {
  const asked = new URLSearchParams(query()).get("tab");
  const tab = (RUN_TABS as readonly string[]).includes(asked ?? "") ? (asked as RunTab) : location.hash === "#exception" ? "exception" : "timeline";
  return tab === "exception" && !row.exception ? "timeline" : tab;
}

export function RunView(props: { trace: string }) {
  const [detail] = createResource(() => props.trace, (trace) => run(trace));
  const missing = () => detail.error instanceof ApiError && detail.error.status === 404;

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e) || e.key !== "Escape") return;
    navigate(runListLink());
    e.preventDefault();
  };
  onMount(() => document.addEventListener("keydown", onKey));
  onCleanup(() => document.removeEventListener("keydown", onKey));

  return (
    <section class="request run" aria-label="Run">
      <Switch>
        <Match when={missing()}>
          <p class="note">No run has this trace id.</p>
        </Match>
        <Match when={detail.error}>
          <p class="note bad" role="alert">
            The run could not load: {String((detail.error as Error | undefined)?.message ?? detail.error)}
          </p>
        </Match>
        <Match when={detail.loading}>
          <p class="note">Loading.</p>
        </Match>
        <Match when={detail()}>
          {(d) => (
            <>
              <Header row={d().run} />
              <Show when={d().cut_us}>{(cut) => <Cut at={cut()} />}</Show>
              <TabbedRecords detail={d()} />
            </>
          )}
        </Match>
      </Switch>
    </section>
  );
}

/** The top of the page: the title line, the age and the ids, the tiles and the time split. */
function Header(props: { row: RunRow }) {
  const row = () => props.row;
  const now = createNow();
  return (
    <header class="req-head">
      <div class="req-title">
        <h2 class="title">
          <span class="run-of">{OF_LABEL[row().of]}</span> <span class="req-path">{row().name}</span>
        </h2>
        <Outcome value={row().outcome} />
      </div>
      <p class="req-crumb">
        <time datetime={new Date(row().start_us / 1000).toISOString()} title={new Date(row().start_us / 1000).toLocaleString()}>
          {age(row().start_us, now())} ago
        </time>
        <Show when={row().exception}>
          {(threw) => (
            <>
              {" · "}
              <span class="exc">
                <span class="bad">{threw().class}</span> at <Site site={threw()} />
              </span>
            </>
          )}
        </Show>
        {" · "}Trace <code class="req-id">{row().trace}</code>
      </p>
      <div class="tiles req-tiles">
        <Tile label="Duration" value={duration(row().duration_us)} />
        <Tile label="Queries" value={String(row().queries)} unit={row().repeats >= 2 ? `· ${row().repeats}× at one site` : undefined} />
        <Tile label="Dumps" value={String(row().dumps)} />
        <Tile label="Log lines" value={String(row().logs)} />
        <Show when={row().memory_peak !== null}>
          <MemoryTile row={row()} />
        </Show>
      </div>
      <Show when={split(row()).length > 0}>
        <Meter parts={split(row())} total={Math.max(1, row().duration_us)} label="Time by kind" />
        <ul class="peek-legend req-legend">
          <For each={split(row())}>
            {(part) => (
              <li>
                <i class={`k-${part.kind}`} aria-hidden="true" />
                {part.label}
              </li>
            )}
          </For>
        </ul>
      </Show>
    </header>
  );
}

/** The note that the index deleted this run's records before `at`, in microseconds. */
function Cut(props: { at: number }) {
  const when = () => new Date(props.at / 1000);
  return (
    <p class="note cut-note run-cut">
      Records before{" "}
      <time datetime={when().toISOString()}>{when().toLocaleString()}</time> were deleted.
    </p>
  );
}

/** The tab row and the tab the link names. */
function TabbedRecords(props: { detail: RunDetail }) {
  const d = () => props.detail;
  const row = () => d().run;
  const tab = () => currentTab(row());
  const tabs = (): Tab[] => {
    const all: Tab[] = [
      { id: "timeline", label: "Timeline", href: runTabLink(row(), "timeline") },
      { id: "queries", label: "Queries", href: runTabLink(row(), "queries"), count: row().queries },
      { id: "cache", label: "Cache", href: runTabLink(row(), "cache"), count: d().events.filter((e) => e.kind === "cache").length },
      { id: "dumps", label: "Dumps", href: runTabLink(row(), "dumps"), count: d().dumps.length },
      { id: "logs", label: "Logs", href: runTabLink(row(), "logs"), count: d().logs.length },
    ];
    if (row().exception) all.push({ id: "exception", label: "Exception", href: runTabLink(row(), "exception") });
    return all;
  };
  /** Opens a dump's or a log line's tab and focuses it there. */
  const reveal = (kind: "dump" | "log", id: number) => {
    navigate(runTabLink(row(), kind === "dump" ? "dumps" : "logs", `#${kind}-${id}`));
    focusHash();
  };
  return (
    <>
      <Tabs label="Parts of the run" tabs={tabs()} current={tab()} />
      <div class="req-panel" data-tab={tab()}>
        <Switch>
          <Match when={tab() === "timeline"}>
            <Timeline detail={d()} reveal={reveal} />
          </Match>
          <Match when={tab() === "queries"}>
            <Queries detail={d()} />
          </Match>
          <Match when={tab() === "cache"}>
            <Cache detail={d()} />
          </Match>
          <Match when={tab() === "dumps"}>
            <Dumps detail={d()} noun="run" />
          </Match>
          <Match when={tab() === "logs"}>
            <Logs detail={d()} noun="run" />
          </Match>
          <Match when={tab() === "exception"}>
            <ExceptionPanel detail={d()} noun="run" />
          </Match>
        </Switch>
      </div>
    </>
  );
}
