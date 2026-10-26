// The Replay button of a request's page, and the dialog that asks before the request is sent again.
// The dialog asks every time, because the handler runs again with all its effects.
// A value typed for a redacted header is sent once and is never stored.
// What it spends: the target address, for as long as the page is open.

import { createResource, createSignal, For, onMount, Show } from "solid-js";
import { REDACTED, replay, replayTarget, type RequestDetail } from "../api.ts";
import { compareLink, navigate } from "../router.ts";

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** The Replay button and its dialog. `ref` gets the function that opens the dialog, which does nothing
 *  while the button is disabled. The button reads `GET /api/replay` once, when it renders. A `null`
 *  target means the service was started by hand, so the button is disabled and says why. The target is
 *  the address of the server that started the service, and the browser never chooses it. */
export function ReplayAction(props: { detail: RequestDetail; ref?: (open: () => void) => void }) {
  const [target] = createResource(() => replayTarget());
  const [open, setOpen] = createSignal(false);
  /** Why the button is disabled, or `null` when it is not, or while the target loads. */
  const why = (): string | null => {
    if (target.loading) return null;
    if (target.error) return `Replay is not available: ${message(target.error)}`;
    if (target()?.target == null) return "This service was started by hand, so it has no server to send a replay to.";
    if (props.detail.input === null) return "The input of this request was not recorded, so it cannot be sent again.";
    if (!props.detail.request.span) return "This request has no span id, so it cannot be sent again.";
    return null;
  };
  const ready = () => !target.loading && why() === null;
  const show = () => {
    if (ready()) setOpen(true);
  };
  props.ref?.(show);
  return (
    <>
      <button
        type="button"
        class="btn replay-action"
        disabled={!ready()}
        aria-keyshortcuts="r"
        aria-describedby={why() ? "replay-why" : undefined}
        onClick={show}
      >
        Replay
      </button>
      <Show when={why()}>
        <span id="replay-why" class="note">
          {why()}
        </span>
      </Show>
      <Show when={open()}>
        <ReplayDialog detail={props.detail} target={target()!.target!} onClose={() => setOpen(false)} />
      </Show>
    </>
  );
}

/** The dialog itself. It opens when it renders, with Cancel focused so Enter sends nothing, and calls
 *  `onClose` when it closes, which removes it. Each header recorded as `REDACTED` gets an empty field.
 *  Their values are read once, into the `POST /api/replay` body, and the fields go with the dialog: no
 *  signal, setting or storage keeps them. A header left empty is not sent. A plain text field is used,
 *  not a password field, so the browser does not offer to save the value. A replay the server names
 *  opens beside the original in the compare view. */
function ReplayDialog(props: { detail: RequestDetail; target: string; onClose: () => void }) {
  let dialog!: HTMLDialogElement;
  let cancel!: HTMLButtonElement;
  const fields: HTMLInputElement[] = [];
  const [error, setError] = createSignal<string | null>(null);
  const [status, setStatus] = createSignal<number | null>(null);
  const [busy, setBusy] = createSignal(false);
  const row = props.detail.request;
  const input = props.detail.input!;
  const full = input.query ? `${row.path}?${input.query}` : row.path;
  /** Each redacted header once, by the name it first arrived with. */
  const hidden = [...new Map(input.headers.filter(([, value]) => value === REDACTED).map(([name]) => [name.toLowerCase(), name])).values()];

  onMount(() => {
    dialog.showModal();
    cancel.focus();
  });

  const send = (e: SubmitEvent) => {
    e.preventDefault();
    const headers: Record<string, string> = {};
    for (const field of fields) if (field.value) headers[field.name] = field.value;
    setBusy(true);
    setError(null);
    setStatus(null);
    replay({ trace: row.trace, span: row.span! }, headers)
      .then((answer) => {
        if (answer.trace && answer.span) {
          dialog.close();
          navigate(compareLink({ trace: row.trace, span: row.span! }, { trace: answer.trace, span: answer.span }));
        } else setStatus(answer.status);
      })
      .catch((err) => setError(message(err)))
      .finally(() => setBusy(false));
  };

  return (
    <dialog
      ref={dialog}
      class="ask replay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="replay-title"
      aria-describedby="replay-text"
      onClose={() => props.onClose()}
    >
      <form onSubmit={send}>
        <h2 id="replay-title">Send this request again?</h2>
        <div id="replay-text">
          <p>
            This sends <code class="replay-request">{`${row.method} ${full}`}</code> to <code class="replay-target">{props.target}</code>.
          </p>
          <p>Your handler runs again. Anything it changes, such as a payment or an email, happens again.</p>
        </div>
        <Show when={hidden.length > 0}>
          <fieldset class="replay-headers">
            <legend>Redacted headers</legend>
            <p class="note">The server hid these values when it recorded the request. Type a value to send it. An empty header is not sent.</p>
            <For each={hidden}>
              {(name) => (
                <label>
                  <span class="cmp-code">{name}</span>
                  <input ref={(el) => fields.push(el)} name={name} type="text" autocomplete="off" spellcheck={false} />
                </label>
              )}
            </For>
          </fieldset>
        </Show>
        <Show when={error()}>
          <p class="note bad" role="alert">
            {error()}
          </p>
        </Show>
        <Show when={status()}>
          <p class="note" role="status">
            The server answered with status {status()}. It did not name the new request, so it cannot be compared.
          </p>
        </Show>
        <div class="ask-buttons">
          <button ref={cancel} type="button" class="btn" onClick={() => dialog.close()}>
            Cancel
          </button>
          <button type="submit" class="btn danger" disabled={busy()}>
            Send
          </button>
        </div>
      </form>
    </dialog>
  );
}
