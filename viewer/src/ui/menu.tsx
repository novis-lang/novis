// A filter chip that opens a menu of choices: `ChipMenu`. The chip shows its label, and its label
// and the choice when one is set. Picking a choice closes the menu. The first choice is "Any", which
// removes the filter.
//
// The menu is the browser's own popover, so a click outside it or `Escape` closes it, and it is drawn
// above the page. Its place under the chip is set through the element's style properties when it
// opens, which the viewer's `Content-Security-Policy` allows. The arrow keys, `Home` and `End` move
// between the choices, and the list's own keys do nothing while a choice has the focus.

import { createSignal, createUniqueId, For, onCleanup, Show } from "solid-js";

/** One choice: what the menu shows, and the value it sets, or `null` for "Any". */
export interface Choice {
  label: string;
  value: string | null;
}

export function ChipMenu(props: {
  /** The filter's name, such as `Status`. */
  label: string;
  /** The value set now, or `null` when the filter is not set. */
  value: string | null;
  /** How the value set now is shown on the chip. The value itself when it is not set. */
  shown?: string | undefined;
  choices: Choice[];
  /** Shown in the menu in place of the choices, such as while they load. */
  note?: string | null;
  /** Runs each time the menu opens, before it is drawn. */
  onOpen?: () => void;
  onPick: (value: string | null) => void;
}) {
  const id = `menu-${createUniqueId()}`;
  const [open, setOpen] = createSignal(false);
  let chip!: HTMLButtonElement;
  let menu!: HTMLDivElement;

  const items = () => [...menu.querySelectorAll<HTMLButtonElement>("[role=menuitemradio]")];

  const onToggle = (e: Event) => {
    const now = (e as ToggleEvent).newState === "open";
    setOpen(now);
    if (!now) return;
    props.onOpen?.();
    const at = chip.getBoundingClientRect();
    menu.style.setProperty("left", `${Math.round(at.left)}px`);
    menu.style.setProperty("top", `${Math.round(at.bottom + 4)}px`);
    queueMicrotask(() => (items().find((item) => item.ariaChecked === "true") ?? items()[0])?.focus());
  };

  const onKey = (e: KeyboardEvent) => {
    const all = items();
    const at = all.indexOf(document.activeElement as HTMLButtonElement);
    let to: number;
    if (e.key === "ArrowDown") to = at + 1;
    else if (e.key === "ArrowUp") to = at - 1;
    else if (e.key === "Home") to = 0;
    else if (e.key === "End") to = all.length - 1;
    else {
      // Every other key stays in the menu, so the list's own keys do nothing while it is open.
      if (e.key !== "Escape" && e.key !== "Tab") e.stopPropagation();
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    all[Math.min(all.length - 1, Math.max(0, to))]?.focus();
  };

  const pick = (value: string | null) => {
    menu.hidePopover();
    chip.focus();
    props.onPick(value);
  };

  onCleanup(() => {
    if (menu?.matches(":popover-open")) menu.hidePopover();
  });

  return (
    <>
      <button
        ref={chip}
        type="button"
        class="chip"
        classList={{ set: props.value !== null }}
        popovertarget={id}
        aria-haspopup="menu"
        aria-expanded={open() ? "true" : "false"}
      >
        {props.value === null ? props.label : `${props.label}: ${props.shown ?? props.value}`}
        <span class="chip-caret" aria-hidden="true">
          ▾
        </span>
      </button>
      <div ref={menu} id={id} popover="auto" class="menu" role="menu" aria-label={props.label} onToggle={onToggle} onKeyDown={onKey}>
        <button type="button" role="menuitemradio" aria-checked={props.value === null ? "true" : "false"} onClick={() => pick(null)}>
          Any
        </button>
        <Show when={props.note}>
          <p class="menu-note">{props.note}</p>
        </Show>
        <For each={props.choices}>
          {(choice) => (
            <button
              type="button"
              role="menuitemradio"
              aria-checked={props.value === choice.value ? "true" : "false"}
              onClick={() => pick(choice.value)}
            >
              {choice.label}
            </button>
          )}
        </For>
      </div>
    </>
  );
}
