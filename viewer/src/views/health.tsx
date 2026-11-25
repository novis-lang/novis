// The frame's live state and the Errors count on the rail.
//
// The live state is one pill in the top bar. It reads `GET /api/health` and `GET /api/replay`. It is
// *Live* with the port of the server that started the service, or *Recording stopped* when the
// service was started by hand. When records were lost, it turns to the warn colour with their count,
// and a click opens the list of what was lost on the settings page (`settings.tsx`).
//
// The Errors count is the number of exception groups thrown since the tab opened. It reads the first
// page of `GET /api/exceptions`.
//
// Both read again after a new watermark, at most once a second.
//
// What it spends: one health answer and one page of exception groups, while the tab is open.

import { createResource, createSignal, onCleanup, Show } from "solid-js";
import { exceptions, health, replayTarget, watermarks } from "../api.ts";

/** The least time between two reads, in milliseconds. */
const EVERY_MS = 1000;

/** When the tab opened, in microseconds, as the records count time. */
const OPENED_US = Date.now() * 1000;

const [tick, setTick] = createSignal(0);
let started = false;

/** Starts listening for watermarks, once for the tab, and raises `tick` at most once a second. */
function listen(): void {
  if (started) return;
  started = true;
  let last = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  watermarks(() => {
    if (timer !== undefined) return;
    const wait = Math.max(0, last + EVERY_MS - Date.now());
    timer = setTimeout(() => {
      timer = undefined;
      last = Date.now();
      setTick(tick() + 1);
    }, wait);
  }, new AbortController().signal);
}

/** The port of a socket address such as `127.0.0.1:8080` or `[::1]:8080`. */
export function port(address: string): string {
  return address.slice(address.lastIndexOf(":") + 1);
}

/** The live state in the top bar. */
export function LivePill() {
  listen();
  const [state] = createResource(tick, () => health().catch(() => undefined));
  const [target] = createResource(() => replayTarget().catch(() => ({ target: null })));
  const lost = () => {
    const now = state.latest;
    return now ? now.dropped_records + now.unparseable + now.quarantine : 0;
  };
  const lostTitle = () => {
    const now = state.latest;
    if (!now) return "";
    return `${now.dropped_records} records dropped, ${now.unparseable} lines unreadable, ${now.quarantine} files not read. Click to see the list.`;
  };
  const address = () => target.latest?.target ?? null;
  return (
    <Show
      when={lost() === 0}
      fallback={
        <a href="/settings#lost" class="live warn" role="status" title={lostTitle()}>
          <span class="dot" aria-hidden="true" />
          {lost()} lost
        </a>
      }
    >
      <Show
        when={target.loading || address() !== null}
        fallback={
          <span class="live stopped" role="status" title="This service was started by hand. It shows what was recorded.">
            <span class="dot" aria-hidden="true" />
            Recording stopped
          </span>
        }
      >
        <span class="live" role="status" title={address() ? `Recording the server at ${address()}` : undefined}>
          <span class="dot" aria-hidden="true" />
          Live
          <Show when={address()}>{(at) => <span class="live-port">:{port(at())}</span>}</Show>
        </span>
      </Show>
    </Show>
  );
}

/** The number of exception groups thrown since the tab opened. */
export function createErrorCount(): () => number {
  listen();
  const controller = new AbortController();
  onCleanup(() => controller.abort());
  const [page] = createResource(tick, () => exceptions(undefined, controller.signal).catch(() => undefined));
  return () => (page.latest?.groups ?? []).filter((group) => group.last_us >= OPENED_US).length;
}
