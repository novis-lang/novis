// The Request tab of a request's page: the method, the path with its query string, the headers and the
// body the request sent. A development server records them.
// What it spends: nothing beyond the request's record, which the page already loaded.

import { Show } from "solid-js";
import type { RequestDetail } from "../../api.ts";
import { Method } from "../../ui/badges.tsx";
import { Empty } from "../../ui/empty.tsx";
import { BodyView, HeaderTable } from "./body.tsx";

export function RequestInput(props: { detail: RequestDetail }) {
  const row = () => props.detail.request;
  return (
    <Show
      when={props.detail.input}
      fallback={
        <Empty title="What this request sent was not recorded.">
          <p>Only a server that runs in development records it.</p>
        </Empty>
      }
    >
      {(input) => (
        <section class="rt" aria-label="Request">
          <p class="rt-line">
            <Method value={row().method} />{" "}
            <code class="rt-path">
              {row().path}
              {input().query ? `?${input().query}` : ""}
            </code>
          </p>
          <h3 class="rt-head">Headers</h3>
          <HeaderTable headers={input().headers} label="Request headers" />
          <h3 class="rt-head">Body</h3>
          <BodyView body={input().body} headers={input().headers} prefix="request-body" name="Request body" />
        </section>
      )}
    </Show>
  );
}
