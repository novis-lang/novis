// What a view shows when it has nothing to list: a title, and one or two lines that say how to get
// something into it.

import { type JSX } from "solid-js";
import { Icon, type IconName } from "./icon.tsx";

export function Empty(props: { icon?: IconName; title: string; children?: JSX.Element }) {
  return (
    <div class="empty">
      <Icon name={props.icon ?? "info"} />
      <p class="empty-title">{props.title}</p>
      <div class="empty-text">{props.children}</div>
    </div>
  );
}
