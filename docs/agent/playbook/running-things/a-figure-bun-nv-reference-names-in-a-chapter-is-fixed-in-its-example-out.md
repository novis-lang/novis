- **A chapter line `bun nv reference` names as wrong is fixed in its example's `.out`, never in the
  chapter.** `docs/reference/lang/*.md` is rendered from `docs/examples/`, so an edit to the chapter
  passes `bun nv reference` and then fails `bun nv render --check` as "is not what its sources
  render", one verify round later. `git grep` the old line, edit every `.out` and
  `website/snippets/` file it is in, then run `bun nv render` and `bun nv render --website`.
  `until: gone tools/nv/cmd/render.ts:is not what its sources render`
