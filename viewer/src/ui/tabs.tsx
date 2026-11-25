// A row of tabs. Each tab is a link, because the tab is part of the page's link
// (`/requests/<trace>/<span>?tab=queries`), so a reload or a shared link opens the same tab. The
// current one has `aria-current="page"`.

import { For, Show } from "solid-js";

export interface Tab {
  id: string;
  label: string;
  href: string;
  /** A count shown beside the label, such as the number of queries. */
  count?: number;
}

export function Tabs(props: { label: string; tabs: Tab[]; current: string }) {
  return (
    <nav class="tabs" aria-label={props.label}>
      <For each={props.tabs}>
        {(tab) => (
          <a href={tab.href} aria-current={tab.id === props.current ? "page" : undefined}>
            {tab.label}
            <Show when={tab.count !== undefined}>
              <span class="tab-count">{tab.count}</span>
            </Show>
          </a>
        )}
      </For>
    </nav>
  );
}
