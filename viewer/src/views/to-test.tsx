// The Make a test button of a request's page, and the dialog that shows the test. The service prints
// the test with the same function as `nvs debug to-test`, from `GET /api/to-test`. The dialog has a
// copy button, and the text is also selectable.
// What it spends: the text of one test, while the dialog is open.

import { createResource, createSignal, onMount, Show } from "solid-js";
import { type RequestDetail, toTest } from "../api.ts";

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** The Make a test button and its dialog. `ref` gets the function that opens the dialog, which does
 *  nothing while the button is disabled. */
export function ToTestAction(props: { detail: RequestDetail; ref?: (open: () => void) => void }) {
  const [open, setOpen] = createSignal(false);
  /** Why the button is disabled, or `null` when it is not. */
  const why = (): string | null => {
    if (props.detail.input === null) return "The input of this request was not recorded, so no test can be made from it.";
    if (!props.detail.request.span) return "This request has no span id, so no test can be made from it.";
    return null;
  };
  const show = () => {
    if (why() === null) setOpen(true);
  };
  props.ref?.(show);
  return (
    <>
      <button
        type="button"
        class="btn to-test-action"
        disabled={why() !== null}
        aria-keyshortcuts="t"
        aria-describedby={why() ? "to-test-why" : undefined}
        onClick={show}
      >
        Make a test
      </button>
      <Show when={why()}>
        <span id="to-test-why" class="note">
          {why()}
        </span>
      </Show>
      <Show when={open()}>
        <ToTestDialog detail={props.detail} onClose={() => setOpen(false)} />
      </Show>
    </>
  );
}

/** The dialog itself. It opens when it renders, loads the test once, and calls `onClose` when it
 *  closes, which removes it. */
function ToTestDialog(props: { detail: RequestDetail; onClose: () => void }) {
  let dialog!: HTMLDialogElement;
  let close!: HTMLButtonElement;
  const row = props.detail.request;
  const [test] = createResource(() => toTest({ trace: row.trace, span: row.span! }));
  const [copied, setCopied] = createSignal<string | null>(null);

  onMount(() => {
    dialog.showModal();
    close.focus();
  });

  const copy = () => {
    const text = test()?.text;
    if (!text) return;
    navigator.clipboard
      .writeText(text)
      .then(() => setCopied("The test is copied."))
      .catch(() => setCopied("The browser did not allow copying. Select the text and copy it."));
  };

  return (
    <dialog ref={dialog} class="ask to-test" role="dialog" aria-modal="true" aria-labelledby="to-test-title" onClose={() => props.onClose()}>
      <h2 id="to-test-title">A test made from this request</h2>
      <p>
        Save this test in your project and run it with <code>nvs test .</code>. It sends the same request and checks that the response is the
        same.
      </p>
      <Show when={test.error}>
        <p class="note bad" role="alert">
          No test was made: {message(test.error)}
        </p>
      </Show>
      <Show when={test.loading}>
        <p class="note">Loading.</p>
      </Show>
      <Show when={test()}>{(t) => <pre class="to-test-text">{t().text}</pre>}</Show>
      <Show when={copied()}>
        <p class="note" role="status">
          {copied()}
        </p>
      </Show>
      <div class="ask-buttons">
        <button ref={close} type="button" class="btn" onClick={() => dialog.close()}>
          Close
        </button>
        <button type="button" class="btn" disabled={!test()} onClick={copy}>
          Copy
        </button>
      </div>
    </dialog>
  );
}
