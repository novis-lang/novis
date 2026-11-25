// The figures a page leads with: a `Tile` with a label and a value, a `Spark` of bars over time,
// and a `Meter` that splits one total into parts, such as a request's time by kind.
//
// A bar's height and a part's width change at run time, so they are set through the element's style
// properties, which the viewer's `Content-Security-Policy` allows. An inline `style` attribute in the
// page's markup is not allowed.

import { For, type JSX, Show } from "solid-js";

/** A labelled figure. `unit` is drawn smaller beside the value, and `children` below it. */
export function Tile(props: { label: string; value: string; unit?: string | undefined; children?: JSX.Element }) {
  return (
    <div class="tile">
      <h3 class="tile-label">{props.label}</h3>
      <p class="tile-value">
        {props.value}
        <Show when={props.unit}>
          <small> {props.unit}</small>
        </Show>
      </p>
      {props.children}
    </div>
  );
}

/** One bar per value, scaled to the largest. A value in `bad` is drawn in the error colour. */
export function Spark(props: { values: number[]; bad?: boolean[]; label: string }) {
  const top = () => Math.max(1, ...props.values);
  return (
    <div class="spark" role="img" aria-label={props.label}>
      <For each={props.values}>
        {(value, i) => (
          <i class={props.bad?.[i()] ? "bad" : undefined} style={{ height: `${Math.max(4, (value / top()) * 100)}%` }} />
        )}
      </For>
    </div>
  );
}

/** One part of a `Meter`: its kind names its colour (`k-query`, `k-own`, ...), and `value` its size. */
export interface Part {
  kind: string;
  value: number;
  label: string;
}

/** The parts side by side, each as wide as its share of `total`, or of their sum without one. */
export function Meter(props: { parts: Part[]; total?: number; label: string }) {
  const total = () => props.total ?? props.parts.reduce((sum, p) => sum + p.value, 0);
  return (
    <div class="meter" role="img" aria-label={props.label}>
      <For each={props.parts}>
        {(part) => <i class={`k-${part.kind}`} title={part.label} style={{ width: `${total() ? (part.value / total()) * 100 : 0}%` }} />}
      </For>
    </div>
  );
}
