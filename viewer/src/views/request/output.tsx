// The Response tab of a request's page: the status, the headers and the body the request returned.
// A development server records them.
// What it spends: nothing beyond the request's record, which the page already loaded.

import { Show } from "solid-js";
import type { RequestDetail } from "../../api.ts";
import { Status } from "../../ui/badges.tsx";
import { Empty } from "../../ui/empty.tsx";
import { BodyView, HeaderTable } from "./body.tsx";

export function RequestOutput(props: { detail: RequestDetail }) {
  const headers = () => props.detail.input?.response_headers ?? null;
  return (
    <Show
      when={headers()}
      fallback={
        <Empty title="What this request returned was not recorded.">
          <p>Only a server that runs in development records it.</p>
        </Empty>
      }
    >
      {(list) => (
        <section class="rt" aria-label="Response">
          <p class="rt-line">
            Status <Status code={props.detail.request.status} />
          </p>
          <h3 class="rt-head">Headers</h3>
          <HeaderTable headers={list()} label="Response headers" />
          <h3 class="rt-head">Body</h3>
          <Show when={props.detail.input?.response_body} fallback={<p class="note">The body was not recorded.</p>}>
            {(body) => <BodyView body={body()} headers={list()} prefix="response-body" name="Response body" />}
          </Show>
        </section>
      )}
    </Show>
  );
}
