// The viewer's frame: the header with the search box, the theme control, the editor settings and the
// key list, the links to the views and the saved filters, and the region the view the path names renders into (`router.ts`).
// The frame answers the keys every view shares, `/`, `?` and `t`, and turns a click on a link inside
// the viewer into a move without a reload.
//
// The search box opens a pasted `traceresponse` value or trace id at once, and searches for anything
// else at `/search?q=`.

import { createEffect, createMemo, For, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import { traceKey } from "./api.ts";
import { ignored, KEYS } from "./keys.ts";
import { navigate, path, query, route, version } from "./router.ts";
import { nextTheme, setTheme, theme, type Theme } from "./theme.ts";
import { EditorSettings } from "./views/editor.tsx";
import { ExceptionsView } from "./views/exceptions.tsx";
import { LogView } from "./views/log.tsx";
import { RequestView } from "./views/request.tsx";
import { RequestList } from "./views/requests.tsx";
import { loadSettings, SavedList, SavedOpen } from "./views/saved.tsx";
import { SearchView } from "./views/search.tsx";
import { StreamView } from "./views/stream.tsx";

export function App() {
  let search!: HTMLInputElement;
  let keys!: HTMLDialogElement;
  const at = createMemo(() => route(path()));
  const request = createMemo(() => {
    const now = at();
    return now.view === "request" ? now : null;
  });
  const saved = createMemo(() => {
    const now = at();
    return now.view === "saved" ? now : null;
  });
  const view = createMemo(() => at().view);

  // On the search's page the box shows the search.
  createEffect(() => {
    if (view() === "search") search.value = new URLSearchParams(query()).get("q") ?? "";
  });

  /** Opens the request a pasted `traceresponse` value or trace id names, or searches for the text. */
  const submit = (e: SubmitEvent) => {
    e.preventDefault();
    const text = search.value.trim();
    if (!text) return;
    search.blur();
    const key = traceKey(text);
    if (key) {
      search.value = "";
      navigate(key.span ? `/requests/${key.trace}/${key.span}` : `/requests/${key.trace}`);
    } else navigate(`/search?${new URLSearchParams({ q: text })}`);
  };

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e)) return;
    if (e.key === "/") search.focus();
    else if (e.key === "?") keys.showModal();
    else if (e.key === "t") setTheme(nextTheme());
    else return;
    e.preventDefault();
  };

  const onClick = (e: MouseEvent) => {
    if (e.defaultPrevented || e.button !== 0 || e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;
    const link = e.target instanceof Element ? e.target.closest("a[href]") : null;
    const href = link?.getAttribute("href");
    if (!href?.startsWith("/") || href.startsWith("//") || href.startsWith("/api/")) return;
    e.preventDefault();
    navigate(href);
  };

  onMount(() => {
    document.addEventListener("keydown", onKey);
    document.addEventListener("click", onClick);
    loadSettings();
  });
  onCleanup(() => {
    document.removeEventListener("keydown", onKey);
    document.removeEventListener("click", onClick);
  });

  const current = (name: string) => (view() === name ? "page" : undefined);

  return (
    <>
      <header class="bar">
        <h1>Novis debug</h1>
        <form class="search" role="search" onSubmit={submit}>
          <input
            ref={search}
            type="search"
            aria-label="Search"
            aria-keyshortcuts="/"
            placeholder="Search, or paste a trace id"
            spellcheck={false}
            autocomplete="off"
            onKeyDown={(e) => {
              if (e.key === "Escape") search.blur();
            }}
          />
        </form>
        <label class="theme">
          Theme{" "}
          <select value={theme()} onChange={(e) => setTheme(e.currentTarget.value as Theme)} aria-keyshortcuts="t">
            <option value="system">System</option>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </label>
        <EditorSettings />
        <button type="button" class="btn" onClick={() => keys.showModal()} aria-keyshortcuts="?">
          Keys
        </button>
      </header>
      <nav class="views" aria-label="Views">
        <a href="/" aria-current={current("list")}>
          Requests
        </a>
        <a href="/stream" aria-current={current("stream")}>
          Stream
        </a>
        <a href="/log" aria-current={current("log")}>
          Log
        </a>
        <a href="/search" aria-current={current("search")}>
          Search
        </a>
        <a href="/exceptions" aria-current={current("exceptions")}>
          Exceptions
        </a>
        <SavedList />
      </nav>
      <main id="view">
        {/* Discarding the index raises `version`, which loads the view on screen again. */}
        <For each={[version()]}>
          {() => (
            <Switch fallback={<RequestList />}>
              <Match when={request()}>{(r) => <RequestView trace={r().trace} span={r().span} />}</Match>
              <Match when={view() === "stream"}>
                <StreamView />
              </Match>
              <Match when={view() === "log"}>
                <LogView />
              </Match>
              <Match when={view() === "search"}>
                <For each={[query()]}>{(q) => <SearchView query={q} />}</For>
              </Match>
              <Match when={view() === "exceptions"}>
                <For each={[query()]}>{(q) => <ExceptionsView query={q} />}</For>
              </Match>
              <Match when={saved()}>{(s) => <SavedOpen name={s().name} />}</Match>
            </Switch>
          )}
        </For>
      </main>
      <dialog ref={keys} class="keys" aria-labelledby="keys-title">
        <h2 id="keys-title">Keyboard shortcuts</h2>
        <table>
          <tbody>
            <For each={KEYS}>
              {([names, action]) => (
                <tr>
                  <th scope="row">
                    <For each={names}>{(name) => <kbd>{name}</kbd>}</For>
                  </th>
                  <td>{action}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
        <form method="dialog">
          <button class="btn">Close</button>
        </form>
      </dialog>
    </>
  );
}
