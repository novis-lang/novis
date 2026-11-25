// The settings page at `/settings`, which the rail's foot opens. It has five parts, top to bottom:
// the theme, the editor links and path mappings (`editor.tsx`), the saved filters, what the recording
// lost, and the button that discards the index.
//
// The part about lost records has the id `lost`, so `/settings#lost` opens at it. The live pill links
// there when records were lost (`health.tsx`). It lists what `GET /api/health` counts, links to those
// records in the stream, and lists the record files the service stopped reading.
//
// What it spends: one health answer and the list of files the service stopped reading, while the page
// is open.

import { createResource, For, onMount, Show } from "solid-js";
import { discardIndex, health, quarantine } from "../api.ts";
import { reloadView, viewLink } from "../router.ts";
import { setTheme, theme, type Theme } from "../theme.ts";
import { toast } from "../ui/toast.tsx";
import { EditorSettings } from "./editor.tsx";
import { Confirm, SavedFilters } from "./saved.tsx";
import { when } from "./stream.tsx";

const THEMES: ReadonlyArray<{ value: Theme; label: string }> = [
  { value: "system", label: "Follow the system" },
  { value: "dark", label: "Dark" },
  { value: "light", label: "Light" },
];

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** The theme: three radio buttons. The choice is kept in this browser. */
function ThemePart() {
  return (
    <section class="settings-part" aria-labelledby="theme-title">
      <h3 id="theme-title">Theme</h3>
      <div class="theme-choice" role="radiogroup" aria-labelledby="theme-title">
        <For each={THEMES}>
          {(one) => (
            <label>
              <input type="radio" name="theme" value={one.value} checked={theme() === one.value} onChange={() => setTheme(one.value)} />{" "}
              {one.label}
            </label>
          )}
        </For>
      </div>
      <p class="note">This browser keeps the choice. The theme button in the top bar changes it too.</p>
    </section>
  );
}

/** What the recording lost: the counts, links to those records, and the files that could not be read. */
function LostPart() {
  const [counts] = createResource(() => health());
  const [files] = createResource(() => quarantine());
  const list = () => (files.error ? [] : (files()?.files ?? []));
  return (
    <section id="lost" class="settings-part lost" aria-labelledby="lost-title" tabindex="-1">
      <h3 id="lost-title">Lost records</h3>
      <Show when={counts.error}>
        <p class="note bad" role="alert">
          The counts could not load: {message(counts.error)}
        </p>
      </Show>
      <Show when={counts()}>
        {(now) => (
          <ul class="lost-counts">
            <li>
              <span class="num">{now().dropped_records}</span> records were dropped by a program, in {now().dropped} reports.{" "}
              <Show when={now().dropped > 0}>
                <a href={viewLink("stream", "kind=dropped")}>Show the reports</a>
              </Show>
            </li>
            <li>
              <span class="num">{now().unparseable}</span> lines could not be read.{" "}
              <Show when={now().unparseable > 0}>
                <a href={viewLink("stream", "kind=unparseable")}>Show the lines</a>
              </Show>
            </li>
            <li>
              <span class="num">{now().quarantine}</span> record files are not read now.
            </li>
          </ul>
        )}
      </Show>
      <Show when={files.error}>
        <p class="note bad" role="alert">
          The list of files could not load: {message(files.error)}
        </p>
      </Show>
      <Show when={list().length > 0}>
        <table class="quarantined">
          <thead>
            <tr>
              <th scope="col">File</th>
              <th scope="col">Error</th>
              <th scope="col" class="num">
                Failures
              </th>
              <th scope="col">Next try</th>
            </tr>
          </thead>
          <tbody>
            <For each={list()}>
              {(file) => (
                <tr>
                  <td class="site">{file.path}</td>
                  <td class="bad">{file.reason}</td>
                  <td class="num">{file.failures}</td>
                  <td>{when(file.retry_us)}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>
    </section>
  );
}

/** The button that discards the index, and the dialog that asks first. */
function IndexPart() {
  let ask!: () => void;
  return (
    <section class="settings-part" aria-labelledby="index-title">
      <h3 id="index-title">Index</h3>
      <p class="note">The viewer reads the records from an index. Discarding it deletes every record the viewer has read.</p>
      <button type="button" class="btn danger discard" onClick={() => ask()}>
        Discard the index
      </button>
      <Confirm
        ref={(open) => (ask = open)}
        title="Discard the index?"
        action="Discard"
        onConfirm={async () => {
          await discardIndex();
          toast("The index is discarded.");
          reloadView();
        }}
      >
        <p>This deletes every record the viewer has read. Your saved filters are kept.</p>
      </Confirm>
    </section>
  );
}

/** The page at `/settings`. */
export function SettingsView() {
  onMount(() => {
    if (location.hash === "#lost") document.getElementById("lost")?.focus();
  });
  return (
    <section class="settings" aria-label="Settings">
      <h2 class="title">Settings</h2>
      <ThemePart />
      <EditorSettings />
      <SavedFilters />
      <LostPart />
      <IndexPart />
    </section>
  );
}
