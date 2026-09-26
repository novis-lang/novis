- **A footprint recorded with a warm compile cache is missing the whole code generator, and the
  selection then skips the cases a codegen edit breaks.** A cached compile never runs the code
  generator, so none of its functions reach the coverage profile. Record through
  `tools/nv/select/record.ts` (a sweep, `bun nv verify`, `bun nv select --seed`), which starts every
  recorded run with its cache empty or off, and set `NOVIS_NO_FILE_CACHE=1` on any recording run
  started by hand.
  [until: gone tools/nv/select/record.ts:NOVIS_NO_FILE_CACHE]
