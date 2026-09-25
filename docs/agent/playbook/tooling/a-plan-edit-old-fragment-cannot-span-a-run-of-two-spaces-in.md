- **A `## plan-edit:` `--- old` fragment cannot span a run of two spaces in the field, and the
  refusal reads as if the words were wrong.** `tools/nv/cmd/session.ts`'s `normalize` collapses
  every run of whitespace in a fragment to one space, but the plan keeps the spacing inside a line
  of the field, so a stray double space in the plan matches no normalized quote; the tell is a
  refusal quoting your fragment back word for word. Quote a shorter run on one side of the anomaly,
  or two runs as two pairs — `bun nv plan --get 'Open now'` prints the field with the double space
  invisible. [until: gone tools/nv/cmd/session.ts:export function normalize]
