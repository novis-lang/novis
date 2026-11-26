// The viewer's frame, as `viewer/design/reference.html` draws it: a top bar with the logo, the search
// box, the live state (`views/health.tsx`), the theme button and the key list; a rail on the left
// with the views, the count of new error groups, the saved filters and the settings; and the region
// the view the path names renders into (`router.ts`).
//
// The frame binds the keys every view shares in the key map (`keys.ts`): `/` focuses the search box,
// `?` lists the keys, and `Ctrl+K` (`Cmd+K` on macOS) opens the command palette (`ui/palette.tsx`).
// It also turns a click on a link inside the viewer into a move without a reload.
//
// The search box opens a pasted `traceresponse` value or trace id at once, and searches for anything
// else at `/search?q=`.

import { createEffect, createMemo, For, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import { traceKey } from "./api.ts";
import { embedded, under } from "./base.ts";
import { listenToEditor } from "./embed.ts";
import { bind, bindings, dispatch, label } from "./keys.ts";
import { navigate, path, query, route, version } from "./router.ts";
import { nextTheme, setTheme, theme, type Theme } from "./theme.ts";
import { Dialog } from "./ui/dialog.tsx";
import { Icon, type IconName } from "./ui/icon.tsx";
import { type Command, Palette } from "./ui/palette.tsx";
import { Toasts } from "./ui/toast.tsx";
import { CompareView } from "./views/compare.tsx";
import { ExceptionsView } from "./views/exceptions.tsx";
import { createErrorCount, LivePill } from "./views/health.tsx";
import { LogView } from "./views/log.tsx";
import { RequestView } from "./views/request.tsx";
import { RequestList } from "./views/requests.tsx";
import { loadSettings, saved, SavedList, savedLink, SavedOpen } from "./views/saved.tsx";
import { RoutesView } from "./views/routes.tsx";
import { SearchView } from "./views/search.tsx";
import { SettingsView } from "./views/settings.tsx";
import { StatementsView } from "./views/statements.tsx";
import { StreamView } from "./views/stream.tsx";

/** The views the rail lists, in its order. `views` are the route views it is current on. */
const RAIL: ReadonlyArray<{ href: string; label: string; icon: IconName; views: string[] }> = [
  { href: "/", label: "Requests", icon: "requests", views: ["list", "request", "compare"] },
  { href: "/routes", label: "Routes", icon: "routes", views: ["routes"] },
  { href: "/queries", label: "Queries", icon: "queries", views: ["queries"] },
  { href: "/exceptions", label: "Errors", icon: "errors", views: ["exceptions"] },
  { href: "/log", label: "Logs", icon: "logs", views: ["log"] },
  { href: "/stream", label: "Stream", icon: "stream", views: ["stream"] },
];

/** The views the command palette lists: the rail's, the search, which the top bar's box opens, and
 *  the settings at the rail's foot. */
const VIEWS = [
  ...RAIL,
  { href: "/search", label: "Search", icon: "search" as const, views: ["search"] },
  { href: "/settings", label: "Settings", icon: "settings" as const, views: ["settings"] },
];

const THEME_NAMES: Record<Theme, string> = { system: "System", dark: "Dark", light: "Light" };
const THEME_ICONS: Record<Theme, IconName> = { system: "system", dark: "moon", light: "sun" };

export function App() {
  let search!: HTMLInputElement;
  let keys!: HTMLDialogElement;
  let openPalette!: () => void;
  const at = createMemo(() => route(path()));
  const request = createMemo(() => {
    const now = at();
    return now.view === "request" ? now : null;
  });
  const compare = createMemo(() => {
    const now = at();
    return now.view === "compare" ? now : null;
  });
  const savedView = createMemo(() => {
    const now = at();
    return now.view === "saved" ? now : null;
  });
  const view = createMemo(() => at().view);
  const errors = createErrorCount();

  // On the search's page the box shows the search.
  createEffect(() => {
    if (view() === "search") search.value = new URLSearchParams(query()).get("q") ?? "";
  });

  const find = (text: string) => navigate(`/search?${new URLSearchParams({ q: text })}`);

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
    } else find(text);
  };

  // A view's link has no base path, so under an editor's frame a click with a modifier moves inside
  // the viewer too: a new window would open the path without the prefix.
  const onClick = (e: MouseEvent) => {
    if (e.defaultPrevented || e.button !== 0) return;
    if (!embedded && (e.ctrlKey || e.metaKey || e.shiftKey || e.altKey)) return;
    const link = e.target instanceof Element ? e.target.closest("a[href]") : null;
    const href = link?.getAttribute("href");
    if (!href?.startsWith("/") || href.startsWith("//") || href.startsWith(under("/api/"))) return;
    e.preventDefault();
    navigate(href);
  };

  const onKey = (e: KeyboardEvent) => {
    dispatch(e);
  };

  onMount(() => {
    onCleanup(
      bind([
        { keys: ["Mod+k"], action: "Open the command palette", run: () => openPalette() },
        { keys: ["/"], action: "Go to the search box", run: () => search.focus() },
        { keys: ["?"], action: "Show this list", run: () => keys.showModal() },
      ]),
    );
    document.addEventListener("keydown", onKey);
    document.addEventListener("click", onClick);
    onCleanup(listenToEditor());
    loadSettings();
  });
  onCleanup(() => {
    document.removeEventListener("keydown", onKey);
    document.removeEventListener("click", onClick);
  });

  /** What the palette lists besides the commands a page registered: the views, the saved filters and
   *  the frame's own actions. */
  const commands = (): Command[] => [
    ...VIEWS.map((one): Command => ({ label: `Go to ${one.label}`, group: "Views", icon: one.icon, run: () => navigate(one.href) })),
    ...(saved()?.filters ?? []).map((one): Command => ({ label: one.name, group: "Saved filters", run: () => navigate(savedLink(one.name)) })),
    ...(["dark", "light", "system"] as const).map(
      (one): Command => ({
        label: one === "system" ? "Follow the system theme" : `Use the ${one} theme`,
        group: "Actions",
        icon: THEME_ICONS[one],
        run: () => setTheme(one),
      }),
    ),
    { label: "Show the keyboard shortcuts", group: "Actions", icon: "keys", hint: "?", run: () => keys.showModal() },
  ];

  return (
    <>
      <header class="top">
        <span class="logo" aria-hidden="true">
          N
        </span>
        <h1 class="brand">
          Novis <span>debug</span>
        </h1>
        <form class="search" role="search" onSubmit={submit}>
          <Icon name="search" />
          <input
            ref={search}
            type="search"
            aria-label="Search"
            aria-keyshortcuts="/"
            placeholder="Search requests, queries and logs, or paste a trace id"
            spellcheck={false}
            autocomplete="off"
            onKeyDown={(e) => {
              if (e.key === "Escape") search.blur();
            }}
          />
          <button type="button" class="search-palette" aria-label="Open the command palette" aria-keyshortcuts="Control+K Meta+K" onClick={() => openPalette()}>
            <kbd>{label("Mod+k")}</kbd>
          </button>
        </form>
        <LivePill />
        <button
          type="button"
          class="icon-btn theme"
          aria-label={`Theme: ${THEME_NAMES[theme()]}`}
          title={`Theme: ${THEME_NAMES[theme()]}. Click to change.`}
          onClick={() => setTheme(nextTheme())}
        >
          <Icon name={THEME_ICONS[theme()]} />
        </button>
        <button type="button" class="icon-btn" aria-label="Keyboard shortcuts" title="Keyboard shortcuts" aria-keyshortcuts="?" onClick={() => keys.showModal()}>
          <Icon name="keys" />
        </button>
      </header>
      <div class="frame">
        <nav class="rail" aria-label="Views">
          <For each={RAIL}>
            {(one) => (
              <a href={one.href} class="rail-item" aria-current={one.views.includes(view()) ? "page" : undefined}>
                <Icon name={one.icon} />
                <span class="rail-label">{one.label}</span>
                <Show when={one.href === "/exceptions" && errors() > 0}>
                  <span class="rail-count" title="Error groups thrown since this tab opened">
                    {errors()}
                  </span>
                </Show>
              </a>
            )}
          </For>
          <SavedList />
          <div class="rail-foot">
            <a href="/settings" class="rail-item editor-open" aria-current={view() === "settings" ? "page" : undefined}>
              <Icon name="settings" />
              <span class="rail-label">Settings</span>
            </a>
          </div>
        </nav>
        <main id="view">
          {/* Discarding the index raises `version`, which loads the view on screen again. */}
          <For each={[version()]}>
            {() => (
              <Switch fallback={<For each={[query()]}>{(q) => <RequestList query={q} />}</For>}>
                <Match when={request()}>{(r) => <RequestView trace={r().trace} span={r().span} />}</Match>
                <Match when={compare()}>{(c) => <CompareView left={c().left} right={c().right} />}</Match>
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
                <Match when={view() === "routes"}>
                  <For each={[query()]}>{(q) => <RoutesView query={q} />}</For>
                </Match>
                <Match when={view() === "queries"}>
                  <StatementsView />
                </Match>
                <Match when={view() === "settings"}>
                  <SettingsView />
                </Match>
                <Match when={savedView()}>{(s) => <SavedOpen name={s().name} />}</Match>
              </Switch>
            )}
          </For>
        </main>
      </div>
      <Dialog ref={(el) => (keys = el)} class="keys" title="Keyboard shortcuts">
        <table>
          <tbody>
            <For each={bindings()}>
              {(one) => (
                <tr>
                  <th scope="row">
                    <For each={one.keys}>{(name) => <kbd>{label(name)}</kbd>}</For>
                  </th>
                  <td>{one.action}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
        <form method="dialog">
          <button class="btn">Close</button>
        </form>
      </Dialog>
      <Palette ref={(open) => (openPalette = open)} extra={commands} search={find} />
      <Toasts />
    </>
  );
}
