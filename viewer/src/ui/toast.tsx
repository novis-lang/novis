// A short message at the foot of the page, such as "Copied". `toast()` shows one, and it goes away
// after four seconds. The region is a live region, so a screen reader reads each message once.

import { createSignal, Show } from "solid-js";

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
  return (
    <div class="toasts" role="status" aria-live="polite">
      <Show when={message()}>{(m) => <p class={`toast${m().bad ? " bad" : ""}`}>{m().text}</p>}</Show>
    </div>
  );
}
