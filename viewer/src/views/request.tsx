// One request, at `/requests/<trace id>/<span id>`: its row from `GET /api/requests?trace=&span=`, and
// everything it recorded from `GET /api/request`.
//
// The header has the method, the path and the status,
// the actions, a line with the route, the age and the ids, four tiles, and the request's time split by
// kind with its legend. The header and the tab row draw from the row alone, so they show before the
// records load.
//
// Below it are tabs, and the tab is part of the link (`?tab=queries`): Timeline, Request, Response,
// Queries, Dumps, Logs, Coverage, and Exception when the request threw. The Timeline tab also lists the
// request's profiles and the lines that copied a value. A link that ends in `#exception`, as the
// exceptions view opens a request, opens the Exception tab and focuses it. Enter on a dump or a log line
// in the timeline opens its tab at that record. Escape goes back to the list.
//
// A link with the trace id alone shows the request when one has that trace id, and the matching rows
// when several do, because an inbound `traceparent` can give two requests one trace id. The records are
// fetched once the link names one request.
//
// The actions are to compare the request with the previous request to the same route (`c`), or with
// the request marked in the list, to make a test from it (`t`, `to-test.tsx`), to copy it as a `curl`
// command (`request/curl.ts`), and to replay it (`r`, `replay.tsx`), the page's one primary action.
// The keyed ones are in the command palette while the page is open. The previous request is the
// nearest older row with the same method and route on one page of `before`, so the lookup is one
// bounded `GET /api/requests`.
//
// What it spends: the one request's records, which the service already limited, and one page of older
// rows while the previous request is looked up, for as long as the view is open.

import { createEffect, createResource, For, type JSX, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import { profileDownload, request, requests, type Json, type RequestDetail, type RequestFilter, type RequestRow, type TraceKey } from "../api.ts";
import { profileLink } from "../editor-link.ts";
import { ignored } from "../keys.ts";
import { compareLink, navigate, query, requestLink } from "../router.ts";
import { Method, Status } from "../ui/badges.tsx";
import { Empty } from "../ui/empty.tsx";
import { addCommands } from "../ui/palette.tsx";
import { Site } from "../ui/site.tsx";
import { Stack } from "../ui/stack.tsx";
import { type Tab, Tabs } from "../ui/tabs.tsx";
import { Meter, Tile } from "../ui/tile.tsx";
import { toast } from "../ui/toast.tsx";
import { isMarked, marked, setMarked } from "./compare.tsx";
import { ReplayAction } from "./replay.tsx";
import { ToTestAction } from "./to-test.tsx";
import { saved } from "./saved.tsx";
import { age, createNow, duration, listLink, MemoryTile, size, split, statusClass } from "./requests.tsx";
import { Coverage } from "./coverage.tsx";
import { curl } from "./request/curl.ts";
import { RequestInput } from "./request/input.tsx";
import { RequestOutput } from "./request/output.tsx";
import { Queries } from "./request/queries.tsx";
import { Timeline } from "./timeline.tsx";
import { ValueTree } from "./value-tree.tsx";

/** The tabs of a request's page, in the order the tab row shows them. */
export const TABS = ["timeline", "request", "response", "queries", "dumps", "logs", "coverage", "exception"] as const;
export type TabId = (typeof TABS)[number];

/** The link to one tab of a request's page. The Timeline tab is the page's own link. */
export function tabLink(row: Pick<RequestRow, "trace" | "span">, tab: TabId, hash = ""): string {
  const base = requestLink(row);
  return `${tab === "timeline" ? base : `${base}?tab=${tab}`}${hash}`;
}

/** The tab the link names, or the Timeline. A link that ends in `#exception` opens the Exception tab. */
function currentTab(row: RequestRow): TabId {
  const asked = new URLSearchParams(query()).get("tab");
  const tab = (TABS as readonly string[]).includes(asked ?? "") ? (asked as TabId) : location.hash === "#exception" ? "exception" : "timeline";
  return tab === "exception" && !row.exception ? "timeline" : tab;
}

export function RequestView(props: { trace: string; span: string | undefined }) {
  const [page] = createResource(
    () => ({ trace: props.trace, span: props.span }),
    ({ trace, span }) => {
      const filter: RequestFilter = { trace };
      if (span) filter.span = span;
      return requests(filter);
    },
  );
  // A resource that failed throws when it is read, so its error is checked first.
  const rows = (): RequestRow[] => (page.error ? [] : (page()?.requests ?? []));

  // A trace id alone that names one request moves to that request's full link. While a new link
  // loads, the resource still gives the last link's rows, so the rows must be this trace's.
  createEffect(() => {
    const found = rows();
    if (page.loading || props.span || found.length !== 1) return;
    if (found[0]!.trace === props.trace && found[0]!.span) navigate(requestLink(found[0]!), true);
  });

  // The records load once the link names one request of this trace.
  const [detail] = createResource(
    () => {
      const found = rows();
      if (page.loading || found.length !== 1 || found[0]!.trace !== props.trace) return false;
      if (props.span && found[0]!.span !== props.span) return false;
      const key: TraceKey = { trace: props.trace };
      if (props.span) key.span = props.span;
      return key;
    },
    (key) => request(key),
  );
  const loaded = () => (detail.error ? undefined : detail());

  // The previous request to the same route: the nearest older row on one page with the same method
  // and route, or the same path when the request has no route.
  const [previous] = createResource(
    () => {
      const row = loaded()?.request;
      return row?.span ? row : false;
    },
    async (row) => {
      const page = await requests({ before: row.id, limit: PREVIOUS_PAGE });
      const where = (r: RequestRow) => r.route ?? r.path;
      return page.requests.find((r) => r.id < row.id && r.span && r.method === row.method && where(r) === where(row)) ?? null;
    },
  );
  /** The link that compares the previous request, on the left, with this one. */
  const previousLink = (): string | null => {
    const row = loaded()?.request;
    const before = previous.error ? null : previous();
    return row?.span && before?.span ? compareLink({ trace: before.trace, span: before.span }, { trace: row.trace, span: row.span }) : null;
  };
  let openReplay: (() => void) | undefined;
  let openToTest: (() => void) | undefined;
  const copyCurl = () => {
    const d = loaded();
    if (!d?.input) return;
    const made = curl(d.request, d.input);
    navigator.clipboard
      .writeText(made.command)
      .then(() => toast(made.bodyLeftOut ? "The command is copied. Its body is not included, because it was not recorded as text." : "The command is copied."))
      .catch(() => toast("The browser did not allow copying.", true));
  };
  onCleanup(
    addCommands([
      { label: "Replay this request", group: "Actions", hint: "r", run: () => openReplay?.() },
      { label: "Make a test from this request", group: "Actions", hint: "t", run: () => openToTest?.() },
      { label: "Copy this request as curl", group: "Actions", run: copyCurl },
      {
        label: "Compare with the previous request to this route",
        group: "Actions",
        hint: "c",
        run: () => {
          const link = previousLink();
          if (link) navigate(link);
        },
      },
    ]),
  );

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e)) return;
    if (e.key === "Escape") navigate(listLink());
    else if (e.key === "c" && previousLink()) navigate(previousLink()!);
    else if (e.key === "r" && openReplay) openReplay();
    else if (e.key === "t" && openToTest) openToTest();
    else return;
    e.preventDefault();
  };
  onMount(() => document.addEventListener("keydown", onKey));
  onCleanup(() => document.removeEventListener("keydown", onKey));

  const actions = (d: RequestDetail) => (
    <div class="actions" role="group" aria-label="Actions">
      <Show when={previousLink()}>
        {(to) => (
          <a href={to()} class="btn compare-previous" aria-keyshortcuts="c" title="Compare with the previous request to this route">
            Compare
          </a>
        )}
      </Show>
      <Show when={marked() && d.request.span && !isMarked(d.request) ? marked() : null}>
        {(m) => (
          <a href={compareLink(m(), { trace: d.request.trace, span: d.request.span! })} class="btn compare-marked" onClick={() => setMarked(null)}>
            Compare with {m().method} {m().path}
          </a>
        )}
      </Show>
      <ToTestAction detail={d} ref={(open) => (openToTest = open)} />
      <button
        type="button"
        class="btn curl-action"
        disabled={d.input === null}
        title={d.input === null ? "The input of this request was not recorded." : undefined}
        onClick={copyCurl}
      >
        Copy as curl
      </button>
      <ReplayAction detail={d} ref={(open) => (openReplay = open)} />
    </div>
  );

  return (
    <section class="request" aria-label="Request">
      <Switch>
        <Match when={page.error}>
          <p class="note bad" role="alert">
            The request could not load: {String(page.error?.message ?? page.error)}
          </p>
        </Match>
        <Match when={page.loading}>
          <p class="note">Loading.</p>
        </Match>
        <Match when={rows().length === 0}>
          <p class="note">{props.span ? "No request has this trace id and span id." : "No request has this trace id."}</p>
        </Match>
        <Match when={rows().length === 1 || props.span}>
          <Header row={rows()[0]!} actions={<Show when={loaded()}>{(d) => actions(d())}</Show>} />
          <TabbedRecords row={rows()[0]!} detail={detail} />
        </Match>
        <Match when={rows().length > 1}>
          <h2>{rows().length} requests have this trace id</h2>
          <ul class="matches">
            <For each={rows()}>
              {(row) => (
                <li>
                  <a href={requestLink(row)}>
                    {row.method} {row.path} <span class={statusClass(row.status)}>{row.status}</span>
                  </a>
                </li>
              )}
            </For>
          </ul>
        </Match>
      </Switch>
    </section>
  );
}

/** How many older rows the previous-request lookup reads: one page of the most the service sends. */
const PREVIOUS_PAGE = 500;

/** The top of the page: the title line with the actions, the route and the ids, the tiles and the
 *  time split. */
function Header(props: { row: RequestRow; actions: JSX.Element }) {
  const row = () => props.row;
  const now = createNow();
  return (
    <header class="req-head">
      <div class="req-title">
        <h2 class="title">
          <Method value={row().method} /> <span class="req-path">{row().path}</span>
        </h2>
        <Status code={row().status} />
        {props.actions}
      </div>
      <p class="req-crumb">
        <Show when={row().route}>
          {(route) => (
            <>
              Route <code>{route()}</code> ·{" "}
            </>
          )}
        </Show>
        <Show when={row().handler}>
          {(handler) => (
            <>
              Handler <code class="req-handler">{handler()}</code> ·{" "}
            </>
          )}
        </Show>
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
        <Show when={row().span}>
          {(span) => (
            <>
              {" · "}Span <code class="req-id">{span()}</code>
            </>
          )}
        </Show>
      </p>
      <div class="tiles req-tiles">
        <Tile label="Duration" value={duration(row().duration_us)} />
        <Tile label="Queries" value={String(row().queries)} unit={row().repeats >= 2 ? `· ${row().repeats}× at one site` : undefined} />
        <Tile label="Dumps" value={String(row().dumps)} />
        <Tile label="Log lines" value={String(row().logs)} />
        <Show when={row().memory_peak !== null}>
          <MemoryTile row={row()} />
        </Show>
        <Show when={row().bytes_out !== null || row().bytes_in !== null}>
          <Tile
            label="Response size"
            value={row().bytes_out === null ? "–" : size(row().bytes_out!)}
            unit={row().bytes_in === null ? undefined : `· ${size(row().bytes_in!)} read`}
          />
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

/** The tab row and the tab the link names, once the records load. */
function TabbedRecords(props: { row: RequestRow; detail: { (): RequestDetail | undefined; loading: boolean; error: unknown } }) {
  const row = () => props.row;
  const tab = () => currentTab(row());
  const tabs = (): Tab[] => {
    const d = props.detail.error ? undefined : props.detail();
    const all: Tab[] = [
      { id: "timeline", label: "Timeline", href: tabLink(row(), "timeline") },
      { id: "request", label: "Request", href: tabLink(row(), "request") },
      { id: "response", label: "Response", href: tabLink(row(), "response") },
      { id: "queries", label: "Queries", href: tabLink(row(), "queries"), count: row().queries },
      { id: "dumps", label: "Dumps", href: tabLink(row(), "dumps"), count: d?.dumps.length ?? row().dumps },
      { id: "logs", label: "Logs", href: tabLink(row(), "logs"), count: d?.logs.length ?? row().logs },
      { id: "coverage", label: "Coverage", href: tabLink(row(), "coverage") },
    ];
    if (row().exception) all.push({ id: "exception", label: "Exception", href: tabLink(row(), "exception") });
    return all;
  };
  /** Opens a dump's or a log line's tab and focuses it there. The tab renders at once, because the
   *  move sets the router's signal. */
  const reveal = (kind: "dump" | "log", id: number) => {
    navigate(tabLink(row(), kind === "dump" ? "dumps" : "logs", `#${kind}-${id}`));
    focusHash();
  };
  return (
    <>
      <Tabs label="Parts of the request" tabs={tabs()} current={tab()} />
      <Switch>
        <Match when={props.detail.error}>
          <p class="note bad" role="alert">
            The records of this request could not load: {String((props.detail.error as Error | undefined)?.message ?? props.detail.error)}
          </p>
        </Match>
        <Match when={props.detail.loading}>
          <p class="note">Loading the records.</p>
        </Match>
        <Match when={props.detail()}>
          {(d) => (
            <div class="req-panel" data-tab={tab()}>
              <Switch>
                <Match when={tab() === "timeline"}>
                  <Timeline detail={d()} reveal={reveal} />
                  <Profiles detail={d()} />
                  <Copies detail={d()} />
                </Match>
                <Match when={tab() === "request"}>
                  <RequestInput detail={d()} />
                </Match>
                <Match when={tab() === "response"}>
                  <RequestOutput detail={d()} />
                </Match>
                <Match when={tab() === "queries"}>
                  <Queries detail={d()} />
                </Match>
                <Match when={tab() === "dumps"}>
                  <Dumps detail={d()} />
                </Match>
                <Match when={tab() === "logs"}>
                  <Logs detail={d()} />
                </Match>
                <Match when={tab() === "coverage"}>
                  <Show when={d().coverage} fallback={<Empty title="No coverage was recorded for this request." />}>
                    {(c) => <Coverage coverage={c()} />}
                  </Show>
                </Match>
                <Match when={tab() === "exception"}>
                  <ExceptionPanel detail={d()} />
                </Match>
              </Switch>
            </div>
          )}
        </Match>
      </Switch>
    </>
  );
}

/** Focuses the record the link's hash names: a dump's first tree node, a log line or the exception. */
function focusHash() {
  const target = location.hash.slice(1);
  if (!target) return;
  const el = document.getElementById(target);
  const node = target.startsWith("dump-") ? el?.querySelector<HTMLElement>(`[role=treeitem][tabindex="0"]`) : el;
  node?.focus();
}

/** A request's time from its start to `us`. */
function at(detail: RequestDetail, us: number): string {
  return duration(Math.max(0, us - detail.request.start_us));
}

/** True when a log line's fields have something to show. */
function hasFields(fields: Json): boolean {
  if (fields === null) return false;
  if (Array.isArray(fields)) return fields.length > 0;
  return typeof fields !== "object" || Object.keys(fields).length > 0;
}

function ExceptionPanel(props: { detail: RequestDetail }) {
  onMount(focusHash);
  return (
    <Show when={props.detail.exception} fallback={<Empty title="This request threw no exception." />}>
      {(exc) => (
        <section id="exception" class="exception" aria-labelledby="exception-title" tabindex="-1">
          <h3 id="exception-title">Exception</h3>
          <p class="exc-head">
            <span class="exc-class bad">{exc().class}</span>: <span class="exc-message">{exc().message}</span>
          </p>
          <p class="site exc-site">
            Thrown at <Site site={exc()} />
          </p>
          <Stack stack={exc()} />
        </section>
      )}
    </Show>
  );
}

function Dumps(props: { detail: RequestDetail }) {
  const d = () => props.detail;
  onMount(focusHash);
  return (
    <section class="dumps" aria-label="Dumps">
      <Show when={d().dumps.length > 0} fallback={<Empty title="This request dumped no values." />}>
        <For each={d().dumps}>
          {(dump) => (
            <article id={`dump-${dump.id}`} class="dump">
              <h4 class="site">
                <Site site={dump} /> <span class="note">at {at(d(), dump.at_us)}</span>
              </h4>
              <Show when={!dump.elided} fallback={<p class="note cut-note">This value was too long to record.</p>}>
                <ValueTree value={dump.tree} prefix={`dump-${dump.id}`} name={`Value dumped at ${dump.file}:${dump.line}`} />
              </Show>
            </article>
          )}
        </For>
      </Show>
      <Show when={d().cut.dumps}>
        <p class="note cut-note">The list of dumps stopped at its limit, so some dumps are not shown.</p>
      </Show>
    </section>
  );
}

function Logs(props: { detail: RequestDetail }) {
  const d = () => props.detail;
  onMount(focusHash);
  return (
    <section class="logs" aria-label="Log lines">
      <Show when={d().logs.length > 0} fallback={<Empty title="This request wrote no log lines." />}>
        <ul class="log-list">
          <For each={d().logs}>
            {(log) => (
              <li id={`log-${log.id}`} class="log" tabindex="-1">
                <p class="log-head">
                  <span class="log-level" classList={{ bad: log.level === "error", s4: log.level === "warning" }}>
                    {log.level}
                  </span>
                  <span class="log-message">{log.message}</span>
                  <span class="note"> at {at(d(), log.at_us)}</span>
                </p>
                <Show when={log.elided}>
                  <p class="note cut-note">This log line was too long to record in full.</p>
                </Show>
                <Show when={log.lossy}>
                  <p class="note">Some bytes of this line were not valid text. They are replaced with �.</p>
                </Show>
                <Show when={hasFields(log.fields)}>
                  <ValueTree value={log.fields} prefix={`log-${log.id}`} name={`Fields of the log line "${log.message}"`} />
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={d().cut.logs}>
        <p class="note cut-note">The list of log lines stopped at its limit, so some log lines are not shown.</p>
      </Show>
    </section>
  );
}

function Profiles(props: { detail: RequestDetail }) {
  const d = () => props.detail;
  return (
    <Show when={d().profiles.length > 0}>
      <section class="profiles" aria-labelledby="profiles-title">
        <h3 id="profiles-title">Profiles</h3>
        <ul class="profile-list">
          <For each={d().profiles}>
            {(profile) => (
              <li id={`profile-${profile.id}`} class="profile">
                <span class="site-text">{profile.path}</span> <span class="note">at {at(d(), profile.at_us)}</span>{" "}
                <Show when={profileLink(profile.path, saved())}>
                  {(to) => (
                    <>
                      <a href={to()} class="profile-open">
                        Open in VS Code
                      </a>{" "}
                    </>
                  )}
                </Show>
                <a href={profileDownload(profile.id)} class="profile-download" download="">
                  Download
                </a>
              </li>
            )}
          </For>
        </ul>
      </section>
    </Show>
  );
}

function Copies(props: { detail: RequestDetail }) {
  const d = () => props.detail;
  return (
    <Show when={d().copies.length > 0}>
      <section class="copies" aria-labelledby="copies-title">
        <h3 id="copies-title">Copies</h3>
        <p class="note">These lines copied a value. The line that copied the most comes first.</p>
        <ul class="copy-list">
          <For each={d().copies}>
            {(copy) => (
              <li id={`copy-${copy.id}`} class="copy">
                <p class="site">
                  <Site site={copy} />{" "}
                  <span class="note">
                    {copy.copies} {copy.copies === 1 ? "copy" : "copies"}, {copy.units} entries or bytes
                  </span>
                </p>
                <Show when={copy.help}>{(help) => <p class="copy-help">{help()}</p>}</Show>
              </li>
            )}
          </For>
        </ul>
        <Show when={d().cut.copies}>
          <p class="note cut-note">This request copied at more lines. Only these are shown.</p>
        </Show>
      </section>
    </Show>
  );
}
