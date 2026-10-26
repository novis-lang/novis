// One request, at `/requests/<trace id>/<span id>`: its row from `GET /api/requests?trace=&span=`, and
// everything it recorded from `GET /api/request`: the exception it threw, its timeline, each dump with
// its value tree, and its log lines. A link back goes to the list, and so does Escape.
//
// A link with the trace id alone shows the request when one has that trace id, and the matching rows
// when several do, because an inbound `traceparent` can give two requests one trace id. The records are
// fetched once the link names one request.
//
// What it spends: the one request's records, which the service already limited, for as long as the
// view is open.

import { createEffect, createResource, For, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import { request, requests, type Json, type RequestDetail, type RequestFilter, type RequestRow, type TraceKey } from "../api.ts";
import { ignored } from "../keys.ts";
import { navigate, requestLink } from "../router.ts";
import { duration, statusClass } from "./requests.tsx";
import { Timeline } from "./timeline.tsx";
import { ValueTree } from "./value-tree.tsx";

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

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e) || e.key !== "Escape") return;
    e.preventDefault();
    navigate("/");
  };
  onMount(() => document.addEventListener("keydown", onKey));
  onCleanup(() => document.removeEventListener("keydown", onKey));

  return (
    <section class="request" aria-label="Request">
      <a href="/" class="back">
        Back to the request list
      </a>
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
          <Details row={rows()[0]!} />
          <Switch>
            <Match when={detail.error}>
              <p class="note bad" role="alert">
                The records of this request could not load: {String(detail.error?.message ?? detail.error)}
              </p>
            </Match>
            <Match when={detail.loading}>
              <p class="note">Loading the timeline.</p>
            </Match>
            <Match when={detail()}>{(d) => <Records detail={d()} />}</Match>
          </Switch>
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

function Details(props: { row: RequestRow }) {
  const row = () => props.row;
  return (
    <>
      <h2 class="title">
        {row().method} {row().route ?? row().path}
      </h2>
      <dl class="fields">
        <dt>Path</dt>
        <dd class="path">{row().path}</dd>
        <dt>Status</dt>
        <dd class={`status ${statusClass(row().status)}`}>{row().status}</dd>
        <dt>Duration</dt>
        <dd>{duration(row().duration_us)}</dd>
        <dt>Started</dt>
        <dd>{new Date(row().start_us / 1000).toLocaleString()}</dd>
        <dt>Queries</dt>
        <dd>{row().queries}</dd>
        <dt>Dumps</dt>
        <dd>{row().dumps}</dd>
        <dt>Log lines</dt>
        <dd>{row().logs}</dd>
        <dt>Exception</dt>
        <dd class="exc">
          <Show when={row().exception} fallback="None">
            {(threw) => (
              <>
                <span class="bad">{threw().class}</span> at {threw().file}:{threw().line}
              </>
            )}
          </Show>
        </dd>
        <dt>Trace id</dt>
        <dd>
          <code>{row().trace}</code>
        </dd>
        <dt>Span id</dt>
        <dd>
          <code>{row().span ?? "None"}</code>
        </dd>
      </dl>
    </>
  );
}

/** True when a log line's fields have something to show. */
function hasFields(fields: Json): boolean {
  if (fields === null) return false;
  if (Array.isArray(fields)) return fields.length > 0;
  return typeof fields !== "object" || Object.keys(fields).length > 0;
}

/** What the request recorded: its exception, its timeline, its dumps and its log lines. */
function Records(props: { detail: RequestDetail }) {
  const d = () => props.detail;
  const at = (us: number) => duration(Math.max(0, us - d().request.start_us));
  return (
    <>
      <Show when={d().exception}>
        {(exc) => (
          <section class="exception" aria-labelledby="exception-title">
            <h3 id="exception-title">Exception</h3>
            <p class="exc-head">
              <span class="exc-class bad">{exc().class}</span>: <span class="exc-message">{exc().message}</span>
            </p>
            <p class="site exc-site">
              Thrown at {exc().file}:{exc().line}
            </p>
          </section>
        )}
      </Show>
      <Timeline detail={d()} />
      <section class="dumps" aria-labelledby="dumps-title">
        <h3 id="dumps-title">Dumps</h3>
        <Show when={d().dumps.length > 0} fallback={<p class="note">This request dumped no values.</p>}>
          <For each={d().dumps}>
            {(dump) => (
              <article id={`dump-${dump.id}`} class="dump">
                <h4 class="site">
                  {dump.file}:{dump.line} <span class="note">at {at(dump.at_us)}</span>
                </h4>
                <Show when={!dump.elided} fallback={<p class="note cut-note">This value was too long to record.</p>}>
                  <ValueTree value={dump.tree} prefix={`dump-${dump.id}`} name={`Value dumped at ${dump.file}:${dump.line}`} />
                </Show>
              </article>
            )}
          </For>
        </Show>
      </section>
      <section class="logs" aria-labelledby="logs-title">
        <h3 id="logs-title">Log lines</h3>
        <Show when={d().logs.length > 0} fallback={<p class="note">This request wrote no log lines.</p>}>
          <ul class="log-list">
            <For each={d().logs}>
              {(log) => (
                <li id={`log-${log.id}`} class="log" tabindex="-1">
                  <p class="log-head">
                    <span class="log-level" classList={{ bad: log.level === "error", s4: log.level === "warning" }}>
                      {log.level}
                    </span>
                    <span class="log-message">{log.message}</span>
                    <span class="note"> at {at(log.at_us)}</span>
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
      </section>
    </>
  );
}
