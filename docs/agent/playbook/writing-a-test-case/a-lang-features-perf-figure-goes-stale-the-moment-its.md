- **A `lang:` feature's perf figure goes stale the moment its *reference chapter* is edited, because
  the chapter is what the feature is "implemented at".** Fixing one stale sentence in
  `docs/reference/lang/90-attributes.md` turned every measured `lang:attributes/…` figure into
  `perf: stale: … changed since it was last measured`, including features this session never touched.
  Re-measure with `python tools/dossier.py --record-perf --only '<feature>' …` in the same session,
  and budget for it whenever a slice corrects the chapter its own feature is read from.
  [until: gone tools/dossier.py:impl_hash]
