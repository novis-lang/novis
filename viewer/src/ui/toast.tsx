// A short message at the foot of the page, such as "Copied". `toast()` shows one, and it goes away
// after four seconds. The region is a live region, so a screen reader reads each message once.
//
// The region is a manual popover, shown again with each message, so it is in the top layer and
// draws above a modal dialog that is open, such as the one a copy is made from.

import { createEffect, createSignal, Show } from "solid-js";

const [message, setMessage] = createSignal<{ text: string; bad: boolean } | null>(null);
let timer: ReturnType<typeof setTimeout> | undefined;

/** Shows `text` for four seconds. `bad` draws it in the error colour. */
export function toast(text: string, bad = false): void {
  clearTimeout(timer);
  setMessage({ text, bad });
  timer = setTimeout(() => setMessage(null), 4000);
}

/** The region the frame renders once. */
export function Toasts() {
  let region!: HTMLDivElement;
  createEffect(() => {
    const shown = message() !== null;
    if (region.matches(":popover-open")) region.hidePopover();
    if (shown) region.showPopover();
  });
  return (
    <div ref={region} class="toasts" popover="manual" role="status" aria-live="polite">
      <Show when={message()}>{(m) => <p class={`toast${m().bad ? " bad" : ""}`}>{m().text}</p>}</Show>
    </div>
  );
}
