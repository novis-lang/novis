// The editor section of the settings page (`settings.tsx`). `editor-link.ts` builds the links.
// The developer picks the editor that every `file:line` link opens, or writes a link template.
// Path mappings rewrite a recorded path for a program that ran in a container or a virtual machine.
// Saving sends the whole settings, and the form shows the service's message when it refuses them.

import { createEffect, createSignal, For, Index, Show } from "solid-js";
import { EDITORS, type Editor, type PathMapping, type Settings } from "../api.ts";
import { toast } from "../ui/toast.tsx";
import { putSettings, saved } from "./saved.tsx";

/** The most path mappings the service keeps. */
const MAX_MAPPINGS = 20;

/** What the settings call each editor. */
export const EDITOR_NAMES: Record<Editor, string> = {
  vscode: "VS Code",
  "vscode-insiders": "VS Code Insiders",
  vscodium: "VSCodium",
  cursor: "Cursor",
  zed: "Zed",
  idea: "IntelliJ IDEA",
  phpstorm: "PhpStorm",
  custom: "Other editor",
};

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** The editor, its link template and the path mappings, as one form with a Save button. */
export function EditorSettings() {
  const [editor, setEditor] = createSignal<Editor>("vscode");
  const [template, setTemplate] = createSignal("");
  const [mappings, setMappings] = createSignal<PathMapping[]>([]);
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  /** Fills the form from the settings, when they load and after each save. */
  createEffect(() => {
    const now = saved();
    if (!now) return;
    setEditor(now.editor);
    setTemplate(now.editor_template ?? "");
    setMappings(now.path_mappings.map((mapping) => ({ ...mapping })));
  });

  const change = (i: number, key: keyof PathMapping, value: string) =>
    setMappings(mappings().map((mapping, j) => (j === i ? { ...mapping, [key]: value } : mapping)));

  const submit = (e: SubmitEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    putSettings((base) => {
      const { editor_template: _, ...rest } = base;
      const next: Settings = {
        ...rest,
        editor: editor(),
        path_mappings: mappings().filter((mapping) => mapping.from !== "" || mapping.to !== ""),
      };
      if (editor() === "custom") next.editor_template = template().trim();
      return next;
    })
      .then(() => toast("The editor settings are saved."))
      .catch((err) => setError(message(err)))
      .finally(() => setBusy(false));
  };

  return (
    <section class="settings-part editor-settings" aria-labelledby="editor-title">
      <h3 id="editor-title">Editor links</h3>
      <form onSubmit={submit}>
        <p class="note">Each file and line in the viewer is a link. The link opens that line in your editor.</p>
        <label class="editor-field">
          Editor{" "}
          <select value={editor()} onChange={(e) => setEditor(e.currentTarget.value as Editor)}>
            <For each={EDITORS}>{(id) => <option value={id}>{EDITOR_NAMES[id]}</option>}</For>
          </select>
        </label>
        <Show when={editor() === "custom"}>
          <label class="editor-field">
            Link template{" "}
            <input
              type="text"
              class="editor-template"
              value={template()}
              spellcheck={false}
              autocomplete="off"
              placeholder="myeditor://open?file={file}&line={line}"
              aria-describedby="editor-template-hint"
              onInput={(e) => setTemplate(e.currentTarget.value)}
            />
          </label>
          <p id="editor-template-hint" class="note">
            The viewer replaces <code>{"{file}"}</code>, <code>{"{line}"}</code> and <code>{"{column}"}</code> with the
            place in the file.
          </p>
        </Show>
        <fieldset class="mappings">
          <legend>Path mappings</legend>
          <p class="note">
            A mapping replaces the start of a recorded path. Use it when your program runs in a container, a virtual
            machine or WSL.
          </p>
          <Index each={mappings()}>
            {(mapping, i) => (
              <div class="mapping">
                <input
                  type="text"
                  class="mapping-from"
                  aria-label={`Recorded path ${i + 1}`}
                  placeholder="/app/"
                  spellcheck={false}
                  autocomplete="off"
                  value={mapping().from}
                  onInput={(e) => change(i, "from", e.currentTarget.value)}
                />
                <span aria-hidden="true">→</span>
                <input
                  type="text"
                  class="mapping-to"
                  aria-label={`Path on this computer ${i + 1}`}
                  placeholder="/home/you/project/"
                  spellcheck={false}
                  autocomplete="off"
                  value={mapping().to}
                  onInput={(e) => change(i, "to", e.currentTarget.value)}
                />
                <button
                  type="button"
                  class="btn small"
                  aria-label={`Remove mapping ${i + 1}`}
                  onClick={() => setMappings(mappings().filter((_, j) => j !== i))}
                >
                  Remove
                </button>
              </div>
            )}
          </Index>
          <button
            type="button"
            class="btn mapping-add"
            disabled={mappings().length >= MAX_MAPPINGS}
            onClick={() => setMappings([...mappings(), { from: "", to: "" }])}
          >
            Add a mapping
          </button>
        </fieldset>
        <Show when={error()}>
          <p class="note bad" role="alert">
            {error()}
          </p>
        </Show>
        <div class="settings-buttons">
          <button type="submit" class="btn" disabled={busy()}>
            Save
          </button>
        </div>
      </form>
    </section>
  );
}
