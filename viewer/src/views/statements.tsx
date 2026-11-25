// The queries view: `/queries` lists the query call sites, the site with the most total time first.
// Each row has two lines: the statement text that ran there last, and its `file:line` as an editor link.
// The figures are the count, the total and the longest time, the requests that ran a query there, and
// how many of those ran one statement text there more than once. A repeated site has a gold wash.
// The arrow keys move the selection, and `Enter` or a click searches for the statement text.
// The list does not follow or pause: each new watermark loads the whole answer again.
// What it spends: the rows of one answer, at most 200 sites, while the view is open.

import { createEffect, createSignal, onCleanup, onMount, Show } from "solid-js";
import { statements, watermarks, type StatementSite, type Statements } from "../api.ts";
import { navigate } from "../router.ts";
import { Repeat } from "../ui/badges.tsx";
import { Empty } from "../ui/empty.tsx";
import { List, type RowKey } from "../ui/list.tsx";
import { Site } from "../ui/site.tsx";
import { duration } from "./requests.tsx";

/** A site row's height in pixels: two lines of text. */
const SITE_HEIGHT = 52;

/** The text of an error, for the list's error line. */
function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** A site's key: its file, its line and the hash of its statement. */
function siteKey(site: StatementSite): string {
  return `${site.file}:${site.line}:${site.hash}`;
}

/** The search for the statement text of `site`. */
function searchLink(site: StatementSite): string {
  return `/search?${new URLSearchParams({ q: site.sql })}`;
}

/**
 * Loads the answer now and again on each new watermark. One load runs at a time. A watermark that
 * arrives during a load starts one more load when it ends.
 */
function liveStatements() {
  const [answer, setAnswer] = createSignal<Statements | null>(null);
  const [error, setError] = createSignal<unknown>(null);
  const abort = new AbortController();
  let running = false;
  let again = false;

  /** Loads the answer, or marks that one more load is due when a load is running. */
  const refresh = () => {
    if (running) {
      again = true;
      return;
    }
    running = true;
    statements(abort.signal)
      .then((next) => {
        setAnswer(next);
        setError(null);
      })
      .catch((err) => {
        if (!abort.signal.aborted) setError(err);
      })
      .finally(() => {
        running = false;
        if (again && !abort.signal.aborted) {
          again = false;
          refresh();
        }
      });
  };

  onMount(() => {
    refresh();
    watermarks(() => refresh(), abort.signal);
  });
  onCleanup(() => abort.abort());

  return { answer, error };
}

/** The cells of one site's row, on two lines. */
function SiteCells(props: { site: StatementSite }) {
  const s = () => props.site;
  return (
    <>
      <span class="st-sql" title={s().sql}>
        {s().sql}
      </span>
      <span class="site st-at">
        <Site site={s()} tabindex={-1} />
      </span>
      <span class="num st-count">{s().count}</span>
      <span class="num st-total">{duration(s().total_us)}</span>
      <span class="num st-max">{duration(s().max_us)}</span>
      <span class="num st-requests">{s().requests}</span>
      <span class="num st-repeated">
        <Show when={s().repeated > 0} fallback={<span class="st-none">0</span>}>
          <Repeat count={s().repeated} title={`In ${s().repeated} requests, one statement ran here more than once`} />
        </Show>
      </span>
    </>
  );
}

/** `/queries`: every query call site, the most total time first, as the service sorts them. */
export function StatementsView() {
  const live = liveStatements();
  const rows = () => live.answer()?.statements ?? [];
  /** The note above the list when the answer did not read or show everything, or an empty string. */
  const note = () => {
    const a = live.answer();
    if (!a) return "";
    const parts: string[] = [];
    if (a.partial) parts.push("Read over the newest 100,000 events.");
    if (a.cut) parts.push("Only the 200 sites with the most total time are shown.");
    return parts.join(" ");
  };
  const [selected, setSelected] = createSignal<RowKey | null>(null);
  // A selected site that is gone from a new answer is no longer selected, and `List` selects the first.
  createEffect(() => {
    const at = selected();
    if (at !== null && !rows().some((site) => siteKey(site) === at)) setSelected(null);
  });

  return (
    <section class="statements" aria-label="Queries">
      <div class="statements-head">
        <h2 class="title">Queries</h2>
      </div>
      <Show when={note()}>{(text) => <p class="note st-note">{text()}</p>}</Show>
      <List
        rows={rows()}
        key={siteKey}
        idPrefix="statement"
        label="Query call sites, the most total time first"
        selected={selected()}
        onSelect={setSelected}
        onOpen={(site) => navigate(searchLink(site))}
        openOnClick
        grid="statements-grid"
        rowHeight={SITE_HEIGHT}
        rowClass={(site) => ({ repeated: site.repeated > 0 })}
        cols={
          <>
            <span>Query and call site</span>
            <span class="num">Count</span>
            <span class="num">Total</span>
            <span class="num">Max</span>
            <span class="num">Requests</span>
            <span class="num">Repeated</span>
          </>
        }
        loaded={live.answer() !== null}
        error={live.error() ? message(live.error()) : null}
        empty={
          <Empty icon="queries" title="No queries recorded yet">
            A query shows here once your program runs one.
          </Empty>
        }
      >
        {(site) => <SiteCells site={site} />}
      </List>
    </section>
  );
}
