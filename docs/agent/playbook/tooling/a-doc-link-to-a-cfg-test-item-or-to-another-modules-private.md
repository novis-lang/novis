- **A doc link to a `#[cfg(test)]` item, or to another module's private `fn`, builds clean, passes
  clippy and fails only `verify.py --doc`.** `verify.py`'s default gate does not include the doc leg
  and only a `DONE` claim runs it, so every unresolvable link a goal writes is inherited by the goal's
  last session as a wall of `broken-intra-doc-links`. Write a `#[cfg(test)]` item in plain backticks,
  path-qualify one that lives in a sibling module (`[`registration::Action`]`), and add `()` to a name
  that is both a function and a module. [until: reviewed 2026-09-15]
