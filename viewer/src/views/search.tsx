// The search at `/search?q=<search>`: the matches of `GET /api/search`, grouped by request, newest
// first. The search box is the one in the frame's header (`app.tsx`), so `/` reaches it from every view.
//
// A snippet's matched words are marked with U+0002 and U+0003 by the service. They become `<mark>`
// elements around text nodes, so no text of a record is ever read as markup. A search that is a trace
// id alone and names one request (`opens`) moves straight to that request's page. A search the service
// cannot read is a `400`, and its message is shown as it was sent.
//
// What it spends: the pages of results loaded so far. Each page is at most one page of groups.

import { createEffect, createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { search, snippetParts, type SearchGroup, type SearchResults } from "../api.ts";
import { ignored } from "../keys.ts";
import { navigate, requestLink } from "../router.ts";
import { duration, statusClass } from "./requests.tsx";
import { SaveButton } from "./saved.tsx";
import { clock, when } from "./stream.tsx";

/** A snippet with its matched words in `<mark>`, built from text nodes. */
export function Snippet(props: { text: string }) {
  return (
    <For each={snippetParts(props.text)}>{(part) => (part.match ? <mark>{part.text}</mark> : part.text)}</For>
  );
}

/** The search in a link's query: its `q`. */
function searched(q: string): string {
  return new URLSearchParams(q).get("q") ?? "";
}

/** One group: its request, linked, and its matches in time order. */
function Group(props: { group: SearchGroup }) {
  const req = () => props.group.request;
  const link = () => {
    const r = req();
    return r ? requestLink(r) : undefined;
  };
  return (
    <li class="group">
      <Show
        when={req()}
        fallback={<p class="group-head note">Not part of a request</p>}
      >
        {(r) => (
          <p class="group-head">
            <a href={link()} class="group-link">
              {r().method} {r().route ?? r().path}
            </a>{" "}
            <span class={`status ${statusClass(r().status)}`}>{r().status}</span>{" "}
            <span class="note">
              {duration(r().duration_us)} at {when(r().start_us)}
            </span>
            <Show when={r().exception}>{(exc) => <span class="exc"> {exc().class}</span>}</Show>
          </p>
        )}
      </Show>
      <Show when={props.group.hits.length > 0}>
        <ul class="hits">
          <For each={props.group.hits}>
            {(hit) => (
              <li class="hit">
                <span class="hit-kind">{hit.kind}</span>
                <span class="note hit-at">{clock(hit.at_us)}</span>
                <Show when={link()} fallback={<span class="snippet">{<Snippet text={hit.snippet} />}</span>}>
                  {(to) => (
                    <a href={to()} class="snippet">
                      <Snippet text={hit.snippet} />
                    </a>
                  )}
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={props.group.more > 0}>
        <p class="note more">
          {props.group.more === 1 ? "1 more match in this request." : `${props.group.more} more matches in this request.`}
        </p>
      </Show>
    </li>
  );
}

/** The results of the search in `query`, the part of the link after `?`. A new search is a new view. */
export function SearchView(props: { query: string }) {
  let results!: HTMLElement;
  const q = searched(props.query);
  const [first] = createResource(() => (q ? search(q) : Promise.resolve(null)));
  const [pages, setPages] = createSignal<SearchResults[]>([]);
  const [older, setOlder] = createSignal<string | null>(null);
  const [moreError, setMoreError] = createSignal<string | null>(null);
  const [loadingMore, setLoadingMore] = createSignal(false);

  const page = () => (first.error ? null : first());
  const groups = () => [...(page()?.groups ?? []), ...pages().flatMap((one) => one.groups)];
  const next = () => (pages().length > 0 ? older() : (page()?.older ?? null));

  // A search that names one request opens it, and Back skips the search.
  createEffect(() => {
    const opens = page()?.opens;
    if (opens) navigate(requestLink(opens), true);
  });

  const loadMore = () => {
    const before = next();
    if (before === null) return;
    setLoadingMore(true);
    setMoreError(null);
    search(q, before)
      .then((more) => {
        setPages([...pages(), more]);
        setOlder(more.older);
      })
      .catch((err) => setMoreError(err instanceof Error ? err.message : String(err)))
      .finally(() => setLoadingMore(false));
  };

  /** Moves the focus to the next or the previous link among the results. */
  const step = (by: number) => {
    const links = [...results.querySelectorAll<HTMLAnchorElement>("a[href]")];
    if (links.length === 0) return;
    const at = links.indexOf(document.activeElement as HTMLAnchorElement);
    const to = at < 0 ? (by > 0 ? 0 : links.length - 1) : Math.min(links.length - 1, Math.max(0, at + by));
    links[to]!.focus();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Enter" || ignored(e)) return;
    if (e.key === "j" || e.key === "ArrowDown") step(1);
    else if (e.key === "k" || e.key === "ArrowUp") step(-1);
    else return;
    e.preventDefault();
  };
  onMount(() => document.addEventListener("keydown", onKey));
  onCleanup(() => document.removeEventListener("keydown", onKey));

  return (
    <section class="search-view" aria-label="Search results" ref={results}>
      <div class="toolbar">
        <span role="status" class="search-state">
          <Show when={!first.loading && page()}>
            {(shown) => (shown().groups.length === 0 ? "Nothing matches this search." : `Results for ${q}, newest first.`)}
          </Show>
        </span>
        <Show when={q}>
          <SaveButton view="search" query={() => props.query} />
        </Show>
      </div>
      <Show when={first.error}>
        <p class="note bad search-error" role="alert">
          {first.error instanceof Error ? first.error.message : String(first.error)}
        </p>
      </Show>
      <Show when={!q}>
        <div class="note search-help">
          <p>Type a search in the box at the top, then press Enter. Press / to go to the box.</p>
          <p>Words find records that contain them. Put a phrase in quotes to find it as written.</p>
          <p>
            To narrow a search, add <code>status:500</code>, <code>status:5xx</code>, <code>method:POST</code>,{" "}
            <code>route:/items/{"{id}"}</code>, <code>kind:query</code>, <code>level:error</code>, <code>duration:&gt;200ms</code> or{" "}
            <code>trace:</code> and a trace id.
          </p>
        </div>
      </Show>
      <Show when={first.loading}>
        <p class="note">Searching.</p>
      </Show>
      <ol class="groups">
        <For each={groups()}>{(group) => <Group group={group} />}</For>
      </ol>
      <Show when={moreError()}>
        <p class="note bad" role="alert">
          More results could not load: {moreError()}
        </p>
      </Show>
      <Show when={next() !== null}>
        <button type="button" class="btn older" disabled={loadingMore()} onClick={loadMore}>
          Show older results
        </button>
      </Show>
    </section>
  );
}
