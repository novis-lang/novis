// The viewer's frame: the header with the search box, the theme control and the key list, and the
// region the view the path names renders into (`router.ts`). The frame answers the keys every view
// shares, `/`, `?` and `t`, and turns a click on a link inside the viewer into a move without a reload.

import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { traceKey } from "./api.ts";
import { ignored, KEYS } from "./keys.ts";
import { navigate, path, route } from "./router.ts";
import { nextTheme, setTheme, theme, type Theme } from "./theme.ts";
import { RequestView } from "./views/request.tsx";
import { RequestList } from "./views/requests.tsx";

export function App() {
  let search!: HTMLInputElement;
  let keys!: HTMLDialogElement;
  const [note, setNote] = createSignal("");
  const request = createMemo(() => {
    const at = route(path());
    return at.view === "request" ? at : null;
  });

  /** Opens the request a pasted `traceresponse` value or trace id names. */
  const openPasted = (e: SubmitEvent) => {
    e.preventDefault();
    const key = traceKey(search.value);
    if (!key) return setNote("This is not a trace id.");
    setNote("");
    search.value = "";
    search.blur();
    navigate(key.span ? `/requests/${key.trace}/${key.span}` : `/requests/${key.trace}`);
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
  });
  onCleanup(() => {
    document.removeEventListener("keydown", onKey);
    document.removeEventListener("click", onClick);
  });

  return (
    <>
      <header class="bar">
        <h1>Novis debug</h1>
        <form class="search" role="search" onSubmit={openPasted}>
          <input
            ref={search}
            type="search"
            aria-label="Trace id"
            aria-describedby="search-note"
            aria-keyshortcuts="/"
            placeholder="Paste a trace id or a traceresponse value"
            spellcheck={false}
            autocomplete="off"
            onInput={() => setNote("")}
            onKeyDown={(e) => {
              if (e.key !== "Escape") return;
              setNote("");
              search.blur();
            }}
          />
          <span id="search-note" class="note bad" role="status">
            {note()}
          </span>
        </form>
        <label class="theme">
          Theme{" "}
          <select value={theme()} onChange={(e) => setTheme(e.currentTarget.value as Theme)} aria-keyshortcuts="t">
            <option value="system">System</option>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </label>
        <button type="button" class="btn" onClick={() => keys.showModal()} aria-keyshortcuts="?">
          Keys
        </button>
      </header>
      <main id="view">
        <Show when={request()} fallback={<RequestList />}>
          {(at) => <RequestView trace={at().trace} span={at().span} />}
        </Show>
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
