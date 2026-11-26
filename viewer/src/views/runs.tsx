// The run list, at `/runs`: the scheduled jobs, the queue jobs and the commands the index keeps,
// newest first, following each watermark as the request list does (`ui/list.tsx`).
//
// The follow bar has the filter chips: what started the run, how it ended, its duration and its
// memory. The filter is the view's query, such as `/runs?outcome=failed`, so a filtered list has a
// link. A row shows what started the run, its name, how it ended, the duration with one bar split by
// the kind of work, the memory peak, the queries, the dumps and the age. A run that threw shows its
// exception's class and site after its name. `Enter` opens the run's page (`run.tsx`).
//
// What it spends: what `createFeed` keeps, a few hundred bytes a row, and one timer that redraws the
// ages every ten seconds.

import { createMemo, createSignal, Show } from "solid-js";
import { runs, type RunFilter, type RunOf, type RunOutcome, type RunRow } from "../api.ts";
import { navigate, runLink } from "../router.ts";
import { Chip, Repeat } from "../ui/badges.tsx";
import { Empty } from "../ui/empty.tsx";
import { createFeed, List, type RowKey } from "../ui/list.tsx";
import { ChipMenu, type Choice } from "../ui/menu.tsx";
import { Site } from "../ui/site.tsx";
import { Meter } from "../ui/tile.tsx";
import { age, createNow, duration, memoryTitle, REQUEST_ROW, size, split } from "./requests.tsx";

/** What the run list filters on: the part of `RunFilter` its link has. */
export interface RunListFilter {
  of?: RunOf;
  outcome?: RunOutcome;
  min_us?: number;
  min_memory?: number;
}

const OFS: readonly RunOf[] = ["schedule", "queue", "command"];
const OUTCOMES: readonly RunOutcome[] = ["ok", "failed", "cancelled"];

/** The words for what started a run. */
export const OF_LABEL: Record<RunOf, string> = { schedule: "Schedule", queue: "Queue", command: "Command" };
/** The words for how a run ended. */
export const OUTCOME_LABEL: Record<RunOutcome, string> = { ok: "OK", failed: "Failed", cancelled: "Cancelled" };

/** Reads the run list's query into its filter. A key with a value the list does not offer is left
 *  out, so a hand-edited link still shows a list. */
export function runListFilter(query: string): RunListFilter {
  const params = new URLSearchParams(query);
  const filter: RunListFilter = {};
  const of = params.get("of");
  if (of && (OFS as readonly string[]).includes(of)) filter.of = of as RunOf;
  const outcome = params.get("outcome");
  if (outcome && (OUTCOMES as readonly string[]).includes(outcome)) filter.outcome = outcome as RunOutcome;
  const min = params.get("min_us");
  if (min && /^\d{1,12}$/.test(min)) filter.min_us = Number(min);
  const memory = params.get("min_memory");
  if (memory && /^\d{1,15}$/.test(memory)) filter.min_memory = Number(memory);
  return filter;
}

/** The query of `filter`, without the `?`, its keys always in one order. */
export function runListQuery(filter: RunListFilter): string {
  const params = new URLSearchParams();
  if (filter.of) params.set("of", filter.of);
  if (filter.outcome) params.set("outcome", filter.outcome);
  if (filter.min_us !== undefined) params.set("min_us", String(filter.min_us));
  if (filter.min_memory !== undefined) params.set("min_memory", String(filter.min_memory));
  return params.toString();
}

/** The link of the run list as it was last shown, so a run's page goes back to it. */
let lastRuns = "/runs";

/** The run list's link with the filter it last had. */
export function runListLink(): string {
  return lastRuns;
}

/** How a run ended, coloured as a status is: green when it is OK, red when it failed. */
export function Outcome(props: { value: RunOutcome }) {
  const cls = () => (props.value === "ok" ? "s2" : props.value === "failed" ? "s5" : "s4");
  return <span class={`status outcome ${cls()}`}>{OUTCOME_LABEL[props.value]}</span>;
}

function RunColumns() {
  return (
    <>
      <span>Kind</span>
      <span>Name</span>
      <span>Outcome</span>
      <span>Duration</span>
      <span class="num">Memory</span>
      <span class="num">Queries</span>
      <span class="num">Dumps</span>
      <span class="num">Age</span>
    </>
  );
}

function RunCells(props: { row: RunRow; now: number; scale: number }) {
  const row = () => props.row;
  return (
    <>
      <span class="run-of">{OF_LABEL[row().of]}</span>
      <span class="path" title={row().name}>
        <span class="path-text">{row().name}</span>
        <Show when={row().exception}>
          {(threw) => (
            <>
              <span class="exc">{threw().class}</span>
              <span class="site exc-at">
                <Site site={threw()} tabindex={-1} />
              </span>
            </>
          )}
        </Show>
      </span>
      <span class="status-cell">
        <Outcome value={row().outcome} />
      </span>
      <span class="duration">
        <span class="num">{duration(row().duration_us)}</span>
        <Meter parts={split(row())} total={Math.max(1, props.scale)} label="Time by kind" />
      </span>
      <span class="num memory" title={memoryTitle(row())}>
        {row().memory_peak === null ? "–" : size(row().memory_peak!)}
      </span>
      <span class="num queries">
        {row().queries}
        <Show when={row().repeats >= 2}>
          <Repeat count={row().repeats} />
        </Show>
      </span>
      <span class="num dumps">{row().dumps}</span>
      <span class="num age">{age(row().start_us, props.now)}</span>
    </>
  );
}

const SLOWER: Choice[] = [10, 100, 1000, 10_000, 60_000].map((ms) => ({ label: duration(ms * 1000), value: String(ms * 1000) }));
const HEAVIER: Choice[] = [1, 8, 32, 128].map((mib) => ({ label: size(mib * 1024 * 1024), value: String(mib * 1024 * 1024) }));

/** The filter chips. Each opens a menu of choices, and a choice opens the list's link with it. */
function Filters(props: { filter: RunListFilter }) {
  const go = (next: RunListFilter) => {
    const query = runListQuery(next);
    navigate(query ? `/runs?${query}` : "/runs");
  };
  const set = <K extends keyof RunListFilter>(key: K, value: RunListFilter[K] | undefined) => {
    const next = { ...props.filter };
    if (value === undefined) delete next[key];
    else next[key] = value;
    go(next);
  };
  return (
    <div class="filter-chips" role="group" aria-label="Filter">
      <ChipMenu
        label="Kind"
        value={props.filter.of ?? null}
        shown={props.filter.of ? OF_LABEL[props.filter.of] : undefined}
        choices={OFS.map((value) => ({ label: OF_LABEL[value], value }))}
        onPick={(value) => set("of", (value ?? undefined) as RunOf | undefined)}
      />
      <ChipMenu
        label="Outcome"
        value={props.filter.outcome ?? null}
        shown={props.filter.outcome ? OUTCOME_LABEL[props.filter.outcome] : undefined}
        choices={OUTCOMES.map((value) => ({ label: OUTCOME_LABEL[value], value }))}
        onPick={(value) => set("outcome", (value ?? undefined) as RunOutcome | undefined)}
      />
      <ChipMenu
        label="Slower than"
        value={props.filter.min_us === undefined ? null : String(props.filter.min_us)}
        shown={props.filter.min_us === undefined ? undefined : duration(props.filter.min_us)}
        choices={SLOWER}
        onPick={(value) => set("min_us", value === null ? undefined : Number(value))}
      />
      <ChipMenu
        label="Memory over"
        value={props.filter.min_memory === undefined ? null : String(props.filter.min_memory)}
        shown={props.filter.min_memory === undefined ? undefined : size(props.filter.min_memory)}
        choices={HEAVIER}
        onPick={(value) => set("min_memory", value === null ? undefined : Number(value))}
      />
      <Show when={runListQuery(props.filter) !== ""}>
        <Chip add onClick={() => go({})}>
          Clear
        </Chip>
      </Show>
    </div>
  );
}

export function RunList(props: { query: string }) {
  const filter = runListFilter(props.query);
  const query = runListQuery(filter);
  lastRuns = query ? `/runs?${query}` : "/runs";
  const filtered = query !== "";
  const feed = createFeed<RunRow, number>({
    load: async (at, signal) => {
      const asked: RunFilter = { ...filter, ...at };
      const page = await runs(asked, signal);
      return { rows: page.runs, older: page.older, watermark: page.watermark };
    },
    key: (row) => row.id,
    cursor: (row) => row.id,
  });
  const [selected, setSelected] = createSignal<RowKey | null>(null);
  const [shown, setShown] = createSignal<RunRow[]>([]);
  const scale = createMemo(() => Math.max(1, ...shown().map((row) => row.duration_us)));
  const now = createNow();

  const empty = () => {
    if (!filtered) {
      return (
        <Empty icon="runs" title="No runs yet">
          <p>
            A scheduled job, a queue job, <code>nvs run</code> and <code>nvs test</code> each record a run when <code>nvs.toml</code> has these
            lines:
          </p>
          <pre class="empty-code">{"[debug]\nstore = true"}</pre>
        </Empty>
      );
    }
    if (feed.more()) return <p class="note">Reading older runs.</p>;
    return (
      <Empty icon="runs" title="No run matches this filter">
        <button type="button" class="btn small" onClick={() => navigate("/runs")}>
          Clear the filter
        </button>
      </Empty>
    );
  };

  return (
    <section class="requests runs" aria-label="Runs">
      <div class="requests-body">
        <List
          rows={feed.rows()}
          key={(row) => row.id}
          idPrefix="run"
          label={filtered ? "Runs that match the filter, newest first" : "Runs, newest first"}
          selected={selected()}
          onSelect={setSelected}
          onOpen={(row) => navigate(runLink(row))}
          openOnClick
          grid="request-grid run-grid"
          rowHeight={REQUEST_ROW}
          rowClass={(row) => ({ threw: row.exception !== null })}
          cols={<RunColumns />}
          feed={feed}
          noun={["run", "runs"]}
          onShown={setShown}
          empty={empty()}
          actions={<Filters filter={filter} />}
        >
          {(row) => <RunCells row={row} now={now()} scale={scale()} />}
        </List>
      </div>
    </section>
  );
}
