// One request, at `/requests/<trace id>/<span id>`: its row from `GET /api/requests?trace=&span=` and
// a link back to the list. A link with the trace id alone shows the request when one has that trace
// id, and the matching rows when several do, because an inbound `traceparent` can give two requests
// one trace id. The timeline comes in a later stage of `docs/goals/debug-viewer.md`.

import { createEffect, createResource, For, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import { requests, type RequestFilter, type RequestRow } from "../api.ts";
import { ignored } from "../keys.ts";
import { navigate, requestLink } from "../router.ts";
import { duration, statusClass } from "./requests.tsx";

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
