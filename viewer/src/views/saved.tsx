// The saved filters: the settings file `GET /api/settings` reads, the list of filters beside the
// views, the "Save" action of the stream, log, search and exceptions views, and the dialog that asks
// before a filter or the index is deleted. The settings are one signal, read once when the viewer
// starts, so the editor links of every view (`site.tsx`) follow a change.
//
// A filter is a name, a view and the part of the view's link after `?`, so opening one is opening that
// link. Saving one sends the whole settings back with `PUT /api/settings`, as the service replaces the
// file whole; the editor, its path mappings and the theme go back as they were read.
//
// What it spends: the settings, at most a few hundred filters of a short query each, for the life of
// the tab.

import { createEffect, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";
import { saveSettings, settings, type SavedFilter, type SavedView, type Settings } from "../api.ts";
import { ignored } from "../keys.ts";
import { navigate, viewLink } from "../router.ts";

const [saved, setSaved] = createSignal<Settings | null>(null);
const [loadError, setLoadError] = createSignal<string | null>(null);

/** The settings as last read or written, or `null` until they load. */
export { saved };

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** Reads the settings file. */
export function loadSettings(): void {
  settings()
    .then((next) => {
      setSaved(next);
      setLoadError(null);
    })
    .catch((err) => setLoadError(message(err)));
}

/** Writes the settings `change` makes from the ones last read, and keeps what the service gives back.
 *  Every view reads the settings from `saved`, so its editor links follow at once. */
export async function putSettings(change: (base: Settings) => Settings): Promise<void> {
  const base = saved() ?? (await settings());
  setSaved(await saveSettings(change(base)));
}

/** Writes the settings with `filters` in place of the saved filters. */
function putFilters(filters: SavedFilter[]): Promise<void> {
  return putSettings((base) => ({ ...base, filters }));
}

/** The link of a saved filter. */
export function savedLink(name: string): string {
  return `/saved/${encodeURIComponent(name)}`;
}

/** A dialog that asks before something is deleted. Cancel has the focus, so Enter deletes nothing. */
export function Confirm(props: {
  ref: (open: () => void) => void;
  title: string;
  children: JSX.Element;
  action: string;
  onConfirm: () => Promise<void>;
}) {
  let dialog!: HTMLDialogElement;
  let cancel!: HTMLButtonElement;
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  props.ref(() => {
    setError(null);
    dialog.showModal();
    cancel.focus();
  });
  const confirm = () => {
    setBusy(true);
    props
      .onConfirm()
      .then(() => dialog.close())
      .catch((err) => setError(message(err)))
      .finally(() => setBusy(false));
  };
  return (
    <dialog ref={dialog} class="ask" aria-labelledby={`${props.action}-title`} aria-describedby={`${props.action}-text`}>
      <h2 id={`${props.action}-title`}>{props.title}</h2>
      <div id={`${props.action}-text`}>{props.children}</div>
      <Show when={error()}>
        <p class="note bad" role="alert">
          {error()}
        </p>
      </Show>
      <div class="ask-buttons">
        <button ref={cancel} type="button" class="btn" autofocus onClick={() => dialog.close()}>
          Cancel
        </button>
        <button type="button" class="btn danger" disabled={busy()} onClick={confirm}>
          {props.action}
        </button>
      </div>
    </dialog>
  );
}

/** The "Save" button of a view and the dialog that asks for a name. The `s` key opens it too. */
export function SaveButton(props: { view: SavedView; query: () => string }) {
  let dialog!: HTMLDialogElement;
  let name!: HTMLInputElement;
  const [error, setError] = createSignal<string | null>(null);
  const [replaces, setReplaces] = createSignal(false);

  const open = () => {
    setError(null);
    setReplaces(false);
    name.value = "";
    dialog.showModal();
    name.focus();
  };

  const submit = (e: SubmitEvent) => {
    e.preventDefault();
    const filter: SavedFilter = { name: name.value.trim(), view: props.view, query: props.query() };
    if (!filter.name) return setError("Type a name for the filter.");
    const others = (saved()?.filters ?? []).filter((other) => other.name !== filter.name);
    putFilters([...others, filter])
      .then(() => dialog.close())
      .catch((err) => setError(message(err)));
  };

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e) || e.key !== "s") return;
    e.preventDefault();
    open();
  };
  onMount(() => document.addEventListener("keydown", onKey));
  onCleanup(() => document.removeEventListener("keydown", onKey));

  return (
    <>
      <button type="button" class="btn save" onClick={open} aria-keyshortcuts="s">
        Save
      </button>
      <dialog ref={dialog} class="ask" aria-labelledby="save-title">
        <h2 id="save-title">Save this filter</h2>
        <form onSubmit={submit}>
          <label>
            Name{" "}
            <input
              ref={name}
              type="text"
              maxlength="80"
              autocomplete="off"
              onInput={() => {
                setError(null);
                setReplaces((saved()?.filters ?? []).some((other) => other.name === name.value.trim()));
              }}
            />
          </label>
          <Show when={replaces()}>
            <p class="note">A saved filter already has this name. Saving replaces it.</p>
          </Show>
          <Show when={error()}>
            <p class="note bad" role="alert">
              {error()}
            </p>
          </Show>
          <div class="ask-buttons">
            <button type="button" class="btn" onClick={() => dialog.close()}>
              Cancel
            </button>
            <button type="submit" class="btn">
              Save
            </button>
          </div>
        </form>
      </dialog>
    </>
  );
}

/** The saved filters, listed beside the views. Each has its link and a button that deletes it. */
export function SavedList() {
  const [doomed, setDoomed] = createSignal("");
  let ask!: () => void;
  return (
    <>
      <Show when={loadError()}>
        <span class="note bad">The saved filters could not load: {loadError()}</span>
      </Show>
      <Show when={(saved()?.filters.length ?? 0) > 0}>
        <ul class="saved" aria-label="Saved filters">
          <For each={saved()?.filters ?? []}>
            {(filter) => (
              <li>
                <a href={savedLink(filter.name)}>{filter.name}</a>
                <button
                  type="button"
                  class="btn small"
                  aria-label={`Delete the saved filter ${filter.name}`}
                  title="Delete"
                  onClick={() => {
                    setDoomed(filter.name);
                    ask();
                  }}
                >
                  ×
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Confirm
        ref={(open) => (ask = open)}
        title="Delete this saved filter?"
        action="Delete"
        onConfirm={() => putFilters((saved()?.filters ?? []).filter((other) => other.name !== doomed()))}
      >
        <p>This deletes the filter {`"${doomed()}"`}. The records it shows are kept.</p>
      </Confirm>
    </>
  );
}

/** The page at `/saved/<name>`: opens the filter's view with its query, once the settings load. */
export function SavedOpen(props: { name: string }) {
  const filter = () => saved()?.filters.find((one) => one.name === props.name);
  // Moving on replaces this link in the history, so Back goes to the page before it.
  const go = () => {
    const found = filter();
    if (found) navigate(viewLink(found.view, found.query), true);
  };
  createEffect(go);
  return (
    <section class="request" aria-label="Saved filter">
      <Show when={loadError()} fallback={<p class="note">{saved() && !filter() ? `No saved filter is named "${props.name}".` : "Loading."}</p>}>
        <p class="note bad" role="alert">
          The saved filters could not load: {loadError()}
        </p>
      </Show>
    </section>
  );
}
