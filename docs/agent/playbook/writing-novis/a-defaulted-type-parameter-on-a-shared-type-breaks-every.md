- **A defaulted type parameter on a shared type breaks every unqualified associated path on it,
  three files away.** `StatementCache<H = String>` compiles, then
  `StatementCache::capacity_for(block)` fails with `E0283: cannot infer type of the type parameter
  H`, because a default is not inference fallback and rustc unifies `H` only from a concrete
  inherent impl. Move the items that never mention `H` out of the impl into free functions rather
  than reaching for a turbofish; a 20-line `rustc --crate-type lib` probe under `.agent-tmp/`
  answers this in one call. [until: reviewed 2026-09-06]
