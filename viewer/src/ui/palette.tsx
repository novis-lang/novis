// The command palette, opened with `Ctrl+K` (`Cmd+K` on macOS). It is a dialog with a text box and
// a listbox. It lists the views, the saved filters and the actions the frame and the page on screen
// registered, and the last entry searches the records for the typed text. Typing filters the list,
// the arrow keys move the selection, and Enter runs the selected entry. The list is in the page only
// while the palette is open, so a closed palette adds no options to the page.

import { createMemo, createSignal, For, Show } from "solid-js";
import { Icon, type IconName } from "./icon.tsx";

/** One entry of the palette. */
export interface Command {
  /** What the entry says, such as "Go to Requests". */
  label: string;
  /** The group it is listed under. */
  group: "Views" | "Saved filters" | "Actions";
  icon?: IconName;
  /** A key that runs the same, shown at the right. */
  hint?: string;
  run: () => void;
}

const [registered, setRegistered] = createSignal<readonly Command[]>([]);

/** Adds `list` to the palette and returns the function that removes it, which a view calls when it
 *  leaves the screen. */
export function addCommands(list: Command[]): () => void {
  setRegistered((now) => [...now, ...list]);
  return () => setRegistered((now) => now.filter((one) => !list.includes(one)));
}

/** The palette dialog. `ref` gets the function that opens it, and `search` runs a search for text. */
export function Palette(props: { ref: (open: () => void) => void; extra: () => Command[]; search: (text: string) => void }) {
  let dialog!: HTMLDialogElement;
  let input!: HTMLInputElement;
  const [text, setText] = createSignal("");
  const [at, setAt] = createSignal(0);
  const [isOpen, setOpen] = createSignal(false);

  const shown = createMemo(() => {
    if (!isOpen()) return [];
    const words = text().trim().toLowerCase().split(/\s+/).filter(Boolean);
    const all = [...props.extra(), ...registered()];
    const found = all.filter((one) => words.every((w) => one.label.toLowerCase().includes(w)));
    const query = text().trim();
    if (query) found.push({ label: `Search for “${query}”`, group: "Actions", icon: "search", run: () => props.search(query) });
    return found;
  });

  const open = () => {
    setText("");
    setAt(0);
    setOpen(true);
    dialog.showModal();
    input.focus();
  };
  props.ref(open);

  const run = (one: Command | undefined) => {
    if (!one) return;
    dialog.close();
    one.run();
  };

  const onKey = (e: KeyboardEvent) => {
    const count = shown().length;
    if (e.key === "ArrowDown") setAt(count ? (at() + 1) % count : 0);
    else if (e.key === "ArrowUp") setAt(count ? (at() - 1 + count) % count : 0);
    else if (e.key === "Home" && e.ctrlKey) setAt(0);
    else if (e.key === "End" && e.ctrlKey) setAt(Math.max(0, count - 1));
    else if (e.key === "Enter") run(shown()[at()]);
    else return;
    e.preventDefault();
    document.getElementById(`palette-${at()}`)?.scrollIntoView({ block: "nearest" });
  };

  return (
    <dialog
      ref={dialog}
      class="dialog palette"
      aria-label="Command palette"
      onClose={() => setOpen(false)}
      onClick={(e) => {
        // A click on the backdrop lands on the dialog itself, and closes it.
        if (e.target === dialog) dialog.close();
      }}
    >
      <div class="palette-find">
        <Icon name="search" />
        <input
          ref={input}
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="palette-list"
          aria-autocomplete="list"
          aria-activedescendant={shown().length ? `palette-${at()}` : undefined}
          aria-label="Type a command or a view"
          placeholder="Type a command or a view"
          spellcheck={false}
          autocomplete="off"
          value={text()}
          onInput={(e) => {
            setText(e.currentTarget.value);
            setAt(0);
          }}
          onKeyDown={onKey}
        />
      </div>
      <Show when={isOpen()}>
        <ul id="palette-list" role="listbox" aria-label="Commands" class="palette-list">
          <For each={shown()}>
            {(one, i) => (
              <li id={`palette-${i()}`} role="option" aria-selected={i() === at()} class="palette-item" onMouseMove={() => setAt(i())} onClick={() => run(one)}>
                <Show when={one.icon}>{(icon) => <Icon name={icon()} />}</Show>
                <span class="palette-label">{one.label}</span>
                <span class="palette-group">{one.group}</span>
                <Show when={one.hint}>
                  <kbd>{one.hint}</kbd>
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={isOpen() && shown().length === 0}>
        <p class="palette-none">Nothing matches.</p>
      </Show>
    </dialog>
  );
}
