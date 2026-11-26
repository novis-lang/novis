// An exception's stack as the viewer shows it: its frames, innermost first, and below them each
// exception in its `previous` chain with its own frames. A frame shows the function and its site as
// a link to the editor (`ui/site.tsx`). A frame of the program is drawn in full weight, and a frame
// of `Core` is dimmed, so the program's own code is the first thing a reader sees. The request
// page's Exception tab and one exception group's page (`views/exceptions.tsx`) both use it.
// What it spends: nothing past the elements it draws.

import { For, Show } from "solid-js";
import type { Frame, Stack as StackData } from "../api.ts";
import { Site } from "./site.tsx";

/** Whether `frame` runs code of `Core`, not code of the program. */
export function isCoreFrame(frame: Frame): boolean {
  return frame.function.startsWith("Core\\");
}

/** The frames of one exception, innermost first, and a note when the list was cut. */
function Frames(props: { frames: Frame[]; cut: boolean }) {
  return (
    <>
      <ol class="frames">
        <For each={props.frames}>
          {(frame) => (
            <li class={isCoreFrame(frame) ? "frame frame-core" : "frame frame-own"}>
              <span class="frame-fn">{frame.function}</span>
              <Show when={frame.file !== undefined && frame.line !== undefined}>
                <Site site={{ file: frame.file ?? "", line: frame.line ?? 0 }} class="frame-site" />
              </Show>
            </li>
          )}
        </For>
      </ol>
      <Show when={props.cut}>
        <p class="note cut-note">Only the innermost 64 frames were recorded.</p>
      </Show>
    </>
  );
}

/** The frames of an exception and its `previous` chain. A stack with neither draws nothing. */
export function Stack(props: { stack: StackData }) {
  const frames = () => props.stack.frames ?? [];
  const previous = () => props.stack.previous ?? [];
  return (
    <div class="stack">
      <Show when={frames().length > 0}>
        <h4 class="stack-title">Stack</h4>
        <Frames frames={frames()} cut={props.stack.frames_cut ?? false} />
      </Show>
      <For each={previous()}>
        {(link) => (
          <section class="cause" aria-label={`Caused by ${link.class}`}>
            <h4 class="stack-title">Caused by</h4>
            <p class="exc-head">
              <span class="exc-class bad">{link.class}</span>: <span class="exc-message">{link.message}</span>
            </p>
            <Show when={link.file !== undefined && link.line !== undefined}>
              <p class="site exc-site">
                Thrown at <Site site={{ file: link.file ?? "", line: link.line ?? 0 }} />
              </p>
            </Show>
            <Show when={(link.frames ?? []).length > 0}>
              <Frames frames={link.frames ?? []} cut={link.frames_cut ?? false} />
            </Show>
          </section>
        )}
      </For>
      <Show when={props.stack.previous_cut}>
        <p class="note cut-note">Only the nearest 8 causes were recorded.</p>
      </Show>
    </div>
  );
}
