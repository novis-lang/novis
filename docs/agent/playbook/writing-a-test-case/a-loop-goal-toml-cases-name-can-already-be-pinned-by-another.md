- **A `loop-goal.toml` `cases` name can already be pinned by another check of the same file, at a
  granularity a `.nvst` case cannot reach.** A claim needing two connections to one server or a
  worker loop lives in a `cargo-named` test or an example fixture, so grepping `tests/` for the
  drafted name finds nothing and the claim reads as open work forever. Search the goal file's other
  checks before the corpus: `grep -n` the drafted name's distinctive words with the underscores
  swapped in. [until: reviewed 2026-09-06]
