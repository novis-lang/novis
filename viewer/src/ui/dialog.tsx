// A modal dialog: the browser's own `<dialog>`, which keeps the focus inside it, closes on Esc and
// makes the page behind it inert. `ref` gets the element, and the caller opens it with `showModal()`.
// It carries `role="dialog"` and `aria-modal`, and `describedby` names the element that explains it.

import { type JSX } from "solid-js";

export function Dialog(props: {
  ref: (el: HTMLDialogElement) => void;
  /** The class the dialog's styles and its tests select, such as `keys`. */
  class: string;
  /** The heading, which also names the dialog for a screen reader. */
  title: string;
  /** The id of the element that says what the dialog does. */
  describedby?: string;
  onClose?: () => void;
  children: JSX.Element;
}) {
  const id = `dialog-${props.class.split(" ")[0]}`;
  return (
    <dialog
      ref={props.ref}
      class={`dialog ${props.class}`}
      role="dialog"
      aria-modal="true"
      aria-labelledby={id}
      aria-describedby={props.describedby}
      onClose={() => props.onClose?.()}
    >
      <h2 id={id}>{props.title}</h2>
      {props.children}
    </dialog>
  );
}
