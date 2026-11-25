// The viewer's icons: inline SVG paths drawn on a 16 pixel grid with one stroke width, so no icon
// package and no file is fetched. An icon is decoration: it is hidden from a screen reader, and the
// text or the `aria-label` beside it says what it means.

const PATHS = {
  requests: "M2 4h12M2 8h12M2 12h8",
  routes: "M3 13V3M3 5h6l-1.5 2L9 9H3M11 13h2",
  queries: "M3 4c0 1.1 2.2 2 5 2s5-.9 5-2-2.2-2-5-2-5 .9-5 2zM3 4v8c0 1.1 2.2 2 5 2s5-.9 5-2V4M3 8c0 1.1 2.2 2 5 2s5-.9 5-2",
  errors: "M8 2 1.5 13.5h13zM8 6.5v3.5M8 12h.01",
  logs: "M3 2.5h10v11H3zM5.5 5.5h5M5.5 8h5M5.5 10.5h3",
  stream: "M1.5 8h3l2-5 3 10 2-5h3",
  search: "M7 2.5a4.5 4.5 0 1 0 0 9 4.5 4.5 0 0 0 0-9zM10.5 10.5l3 3",
  settings:
    "M8 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.5 3.5 5 5M11 11l1.5 1.5M3.5 12.5 5 11M11 5l1.5-1.5",
  sun: "M8 5a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM8 1.5V3M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1 1M11.6 11.6l1 1M3.4 12.6l1-1M11.6 4.4l1-1",
  moon: "M13.5 9.5A5.5 5.5 0 0 1 6.5 2.5a5.5 5.5 0 1 0 7 7z",
  system: "M2 3h12v8H2zM6 14h4M8 11v3",
  keys: "M1.5 4.5h13v7h-13zM4 7h.01M6.5 7h.01M9 7h.01M11.5 7h.01M5 9.5h6",
  command: "M5.5 2.5a2 2 0 1 0 0 4h5a2 2 0 1 0-2-2v7a2 2 0 1 0 2-2h-5a2 2 0 1 0 2 2v-7a2 2 0 0 0-2-2z",
  close: "M4 4l8 8M12 4l-8 8",
  info: "M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM8 7v4M8 5h.01",
} as const;

export type IconName = keyof typeof PATHS;

/** The icon `name`, 16 pixels square, drawn in the text colour around it. */
export function Icon(props: { name: IconName; class?: string }) {
  return (
    <svg class={`icon ${props.class ?? ""}`} viewBox="0 0 16 16" aria-hidden="true">
      <path d={PATHS[props.name]} />
    </svg>
  );
}
