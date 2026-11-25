// A recorded header list and a recorded body, as the Request and Response tabs of a request's page
// draw them. A header list is a table of names and values. A JSON body is a value tree. Any other text
// is shown as text in a `<pre>`, never as markup. A binary body shows its size alone.
// What it spends: the parsed JSON of one body and its tree, for as long as the tab is open.

import { For, Show } from "solid-js";
import { REDACTED, type Json, type RecordedBody } from "../../api.ts";
import { bodyNote } from "../compare.tsx";
import { ValueTree } from "../value-tree.tsx";

/** `[name, value]` pairs as a table. A value the server replaced with `REDACTED` shows muted. */
export function HeaderTable(props: { headers: [string, string][]; label: string }) {
  return (
    <Show when={props.headers.length > 0} fallback={<p class="note">No headers.</p>}>
      <table class="rt-table" aria-label={props.label}>
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">Value</th>
          </tr>
        </thead>
        <tbody>
          <For each={props.headers}>
            {([name, value]) => (
              <tr>
                <th scope="row" class="rt-code">
                  {name}
                </th>
                <Show when={value !== REDACTED} fallback={<td class="rt-redacted">redacted</td>}>
                  <td class="rt-code">{value}</td>
                </Show>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </Show>
  );
}

/** The value of the `content-type` header, or `""`. */
function contentType(headers: [string, string][] | null): string {
  return headers?.find(([name]) => name.toLowerCase() === "content-type")?.[1] ?? "";
}

/** True when `value` has a key that starts with `$`. The value tree reads such a key as a part of a
 *  dumped value, so a body with one is shown as text instead. */
function hasTagKey(value: Json): boolean {
  if (Array.isArray(value)) return value.some(hasTagKey);
  if (value === null || typeof value !== "object") return false;
  return Object.entries(value).some(([key, item]) => key.startsWith("$") || hasTagKey(item));
}

/** The JSON value of a body's text: when the content type names JSON, or the text is an object or a
 *  list that parses. `undefined` otherwise. */
function parsed(text: string, type: string): Json | undefined {
  const trimmed = text.trimStart();
  if (!type.toLowerCase().includes("json") && !trimmed.startsWith("{") && !trimmed.startsWith("[")) return undefined;
  try {
    return JSON.parse(text) as Json;
  } catch {
    return undefined;
  }
}

/** How many bytes a base64 text decodes to. */
function decodedBytes(base64: string): number {
  const clean = base64.replace(/\s/g, "");
  const padding = clean.endsWith("==") ? 2 : clean.endsWith("=") ? 1 : 0;
  return Math.max(0, Math.floor((clean.length * 3) / 4) - padding);
}

/** One recorded body. `prefix` keeps the DOM ids of its tree apart from the page's other trees, and
 *  `name` is the tree's accessible name. A cut body shows the part that was recorded, then the note. */
export function BodyView(props: { body: RecordedBody; headers: [string, string][] | null; prefix: string; name: string }) {
  const note = () => bodyNote(props.body);
  const shown = () => props.body.read && !props.body.withheld;
  const text = () => props.body.text ?? "";
  const json = () => (props.body.base64 === null ? parsed(text(), contentType(props.headers)) : undefined);
  return (
    <div class="rt-body">
      <Show when={shown()}>
        <Show
          when={props.body.base64}
          fallback={
            <Show when={text() !== "" || props.body.cut} fallback={<p class="note">The body is empty.</p>}>
              <Show when={json() !== undefined} fallback={<pre class="body-text">{text()}</pre>}>
                <Show when={!hasTagKey(json()!)} fallback={<pre class="body-text">{JSON.stringify(json(), null, 2)}</pre>}>
                  <ValueTree value={json()!} prefix={props.prefix} name={props.name} />
                </Show>
              </Show>
            </Show>
          }
        >
          {(base64) => <p class="note">{decodedBytes(base64())} bytes of binary data. This body is not text, so it is not shown.</p>}
        </Show>
      </Show>
      <Show when={note()}>{(sentence) => <p class="note cut-note">{sentence()}</p>}</Show>
    </div>
  );
}
