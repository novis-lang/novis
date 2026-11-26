// The small labels every list draws: a filter `Chip`, a request's `Method` and its `Status`. Each
// colour is a role from `tokens.css`, and a status class has one colour everywhere: 2xx green, 3xx
// blue, 4xx orange and 5xx red.

import { type JSX, Show } from "solid-js";

/** A filter chip. `set` draws it as a filter in use, and `add` as the chip that adds one. With
 *  `onClick` it is a button, otherwise a label. */
export function Chip(props: { set?: boolean; add?: boolean; onClick?: () => void; title?: string; children: JSX.Element }) {
  const cls = () => `chip${props.set ? " set" : ""}${props.add ? " add" : ""}`;
  return (
    <Show when={props.onClick} fallback={<span class={cls()}>{props.children}</span>}>
      <button type="button" class={cls()} title={props.title} aria-pressed={props.set ? "true" : undefined} onClick={() => props.onClick?.()}>
        {props.children}
      </button>
    </Show>
  );
}

/** An HTTP method, in the colour of what it does: reads blue, writes green, changes orange and
 *  deletes red. */
export function Method(props: { value: string }) {
  const tone = () => TONES[props.value.toUpperCase()] ?? "other";
  return <span class={`method m-${tone()}`}>{props.value}</span>;
}

const TONES: Record<string, string> = {
  GET: "read",
  HEAD: "read",
  OPTIONS: "read",
  POST: "write",
  PUT: "change",
  PATCH: "change",
  DELETE: "delete",
};

/** An HTTP status code, coloured by its class. `0` or a missing code is drawn as a dash. */
export function Status(props: { code: number | null | undefined; text?: string }) {
  const shown = () => (props.code ? String(props.code) : "–");
  const cls = () => (props.code ? `s${Math.floor(props.code / 100)}` : "s0");
  return (
    <span class={`status ${cls()}`}>
      {shown()}
      <Show when={props.text}>{(text) => <span class="status-text"> {text()}</span>}</Show>
    </span>
  );
}

/** A cache read's result: a green `Hit` when the store had the entry, and an orange `Miss` when it
 *  did not. */
export function Hit(props: { hit: boolean }) {
  return <span class={`hit ${props.hit ? "hit-yes" : "hit-no"}`}>{props.hit ? "Hit" : "Miss"}</span>;
}

/** The gold count beside a statement that ran more than once at one site, such as `5×`. `title`
 *  replaces the text a pointer shows when the count means something else. */
export function Repeat(props: { count: number; title?: string }) {
  return (
    <span class="repeat" title={props.title ?? `Ran ${props.count} times at one site`}>
      {props.count}×
    </span>
  );
}
