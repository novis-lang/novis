// A modal dialog: the browser's own `<dialog>`, which keeps the focus inside it, closes on Esc and
// makes the page behind it inert. `ref` gets the element, and the caller opens it with `showModal()`.

import { type JSX } from "solid-js";

export function Dialog(props: {
  ref: (el: HTMLDialogElement) => void;
  /** The class the dialog's styles and its tests select, such as `keys`. */
  class: string;
  /** The heading, which also names the dialog for a screen reader. */
  title: string;
  onClose?: () => void;
  children: JSX.Element;
}) {
  const id = `dialog-${props.class.split(" ")[0]}`;
  return (
    <dialog ref={props.ref} class={`dialog ${props.class}`} aria-labelledby={id} onClose={() => props.onClose?.()}>
      <h2 id={id}>{props.title}</h2>
      {props.children}
    </dialog>
  );
}
