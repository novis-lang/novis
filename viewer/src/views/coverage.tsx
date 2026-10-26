// This view shows which lines of a file one request ran. A line that ran more often has a darker
// background. Each line number opens that line in your editor. Only the lines on screen are in the
// page, so a long file stays fast. The view notes when the file changed after the request ran.
// What it spends: one record's counts, and the text of the one file on screen.

import { createMemo, createResource, createSignal, For, Match, Show, Switch } from "solid-js";
import { coverage, source, type CoverageRef } from "../api.ts";
import { editorLink } from "../editor-link.ts";
import { heat } from "../heat.ts";
import { saved } from "./saved.tsx";

/** A line's height in pixels. `.source-line` in `style.css` has the same. */
export const LINE_HEIGHT = 20;
/** Lines kept in the DOM above and below the screen, so a short scroll shows no gap. */
const OVERSCAN = 10;
function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function Coverage(props: { coverage: CoverageRef }) {
  const [counts] = createResource(() => props.coverage.id, (id) => coverage(id));
  const files = createMemo(() => (counts.error ? [] : Object.keys(counts()?.files ?? {}).sort()));
  const [picked, setPicked] = createSignal<string | null>(null);
  const file = () => picked() ?? files()[0] ?? null;
  const [text] = createResource(
    () => {
      const f = file();
      return f === null ? false : { id: props.coverage.id, file: f };
    },
    ({ id, file }) => source(id, file),
  );
  const lines = createMemo(() => (text.error ? [] : (text()?.text.replace(/\n$/, "").split("\n") ?? [])));
  const hits = createMemo(() => {
    const f = file();
    return f === null ? {} : (counts()?.files[f] ?? {});
  });
  const most = createMemo(() => Math.max(0, ...Object.values(hits())));
  const ran = () => Object.keys(hits()).length;

  return (
    <section class="coverage" aria-labelledby="coverage-title">
      <h3 id="coverage-title">Coverage</h3>
      <Switch>
        <Match when={props.coverage.elided}>
          <p class="note cut-note">The coverage of this request was too long to record.</p>
        </Match>
        <Match when={counts.error}>
          <p class="note bad" role="alert">
            The coverage could not load: {message(counts.error)}
          </p>
        </Match>
        <Match when={counts.loading}>
          <p class="note">Loading the coverage.</p>
        </Match>
        <Match when={files().length === 0}>
          <p class="note">This request ran no lines that were counted.</p>
        </Match>
        <Match when={file()}>
          {(f) => (
            <>
              <p class="coverage-head">
                <label>
                  File{" "}
                  <select class="coverage-file" value={f()} onChange={(e) => setPicked(e.currentTarget.value)}>
                    <For each={files()}>{(name) => <option value={name}>{name}</option>}</For>
                  </select>
                </label>
                <span class="note">
                  {ran()} {ran() === 1 ? "line" : "lines"} ran. The most-run line ran {most()} {most() === 1 ? "time" : "times"}.
                </span>
              </p>
              <Switch>
                <Match when={text.error}>
                  <p class="note bad" role="alert">
                    The file could not load: {message(text.error)}
                  </p>
                </Match>
                <Match when={text.loading}>
                  <p class="note">Loading the file.</p>
                </Match>
                <Match when={text()}>
                  {(loaded) => (
                    <>
                      <Show when={loaded().changed}>
                        <p class="note coverage-changed">This file changed after the request ran. The counts may not match the lines.</p>
                      </Show>
                      <Source file={f()} lines={lines()} hits={hits()} most={most()} />
                    </>
                  )}
                </Match>
              </Switch>
            </>
          )}
        </Match>
      </Switch>
    </section>
  );
}

/** The lines of `file`, with only those on screen in the DOM. */
function Source(props: { file: string; lines: string[]; hits: Record<string, number>; most: number }) {
  const [scrollTop, setScrollTop] = createSignal(0);
  const [height, setHeight] = createSignal(512);
  const first = createMemo(() => Math.max(0, Math.floor(scrollTop() / LINE_HEIGHT) - OVERSCAN));
  const end = createMemo(() => Math.min(props.lines.length, Math.ceil((scrollTop() + height()) / LINE_HEIGHT) + OVERSCAN));
  const shown = createMemo(() => {
    const out: number[] = [];
    for (let i = first(); i < end(); i++) out.push(i);
    return out;
  });
  return (
    <div
      class="source"
      tabindex="0"
      role="region"
      aria-label={`Source of ${props.file}, with the times each line ran`}
      ref={(el) => queueMicrotask(() => setHeight(el.clientHeight || 512))}
      onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
    >
      <div class="source-lines" style={{ height: `${props.lines.length * LINE_HEIGHT}px` }}>
        <For each={shown()}>
          {(i) => {
            const count = () => props.hits[String(i + 1)] ?? 0;
            const step = () => heat(count(), props.most);
            return (
              <div class={`source-line${step() ? ` heat-${step()}` : ""}`} style={{ top: `${i * LINE_HEIGHT}px` }}>
                <span class="number">
                  <Show when={editorLink({ file: props.file, line: i + 1 }, saved())} fallback={i + 1}>
                    {(to) => (
                      <a href={to()} class="site-link" title={`Open ${props.file}:${i + 1} in your editor`}>
                        {i + 1}
                      </a>
                    )}
                  </Show>
                </span>
                <span class="count">{count() ? `${count()}×` : ""}</span>
                <span class="code">{props.lines[i]}</span>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
