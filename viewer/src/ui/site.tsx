// A source site as the viewer shows it: `file:line`, as a link that opens that line in the developer's
// editor (`editor-link.ts`), or as plain text when the settings give no link. The link follows the
// settings signal (`views/saved.tsx`), so a change in the editor settings reaches every site on screen.
// When an editor frames the viewer, a click sends the site to that editor (`embed.ts`), and every site
// is a link.

import { Show } from "solid-js";
import { embedded } from "../base.ts";
import { editorLink, type Site as SiteData } from "../editor-link.ts";
import { openInEditor } from "../embed.ts";
import { saved } from "../views/saved.tsx";

/** The click on a link to `site`: the editor that frames the page opens it, and the link is not
 *  followed. Without such an editor the link is followed. */
export function siteClick(site: SiteData): (e: MouseEvent) => void {
  return (e) => {
    e.stopPropagation();
    const link = e.currentTarget instanceof Element ? e.currentTarget.getAttribute("href") : null;
    if (openInEditor(site, saved()?.path_mappings ?? []) || link === "#") e.preventDefault();
  };
}

/** `file:line` of `site`, linked to the editor. A site inside a listbox row passes `tabindex={-1}`,
 *  because the row is reached with the arrow keys and opens with Enter. */
export function Site(props: { site: SiteData; tabindex?: number; class?: string }) {
  const text = () => `${props.site.file}:${props.site.line}${props.site.column === undefined ? "" : `:${props.site.column}`}`;
  const href = () => editorLink(props.site, saved()) ?? (embedded ? "#" : null);
  return (
    <Show when={href()} fallback={<span class={`site-text ${props.class ?? ""}`}>{text()}</span>}>
      {(to) => (
        <a
          href={to()}
          class={`site-link ${props.class ?? ""}`}
          tabindex={props.tabindex}
          title="Open in your editor"
          onClick={(e) => siteClick(props.site)(e)}
        >
          {text()}
        </a>
      )}
    </Show>
  );
}
