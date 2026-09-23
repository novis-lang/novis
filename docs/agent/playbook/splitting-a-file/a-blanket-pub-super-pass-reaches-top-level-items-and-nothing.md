- **A blanket `pub(super)` pass reaches top-level items and nothing else, and the compiler names the
  rest in one build.** Fields, `impl` methods and an `unsafe fn` all sit outside a `^(fn |struct
  |…)` regex, so the first build after a split is a wall of `E0616`/`E0624` errors, one mechanical
  widening each. The one a build does not show is a doc link: `[`hydrate`]` stops resolving when
  `hydrate` moves to a sibling, and only `verify.py --doc` says so. [until: reviewed 2026-09-06]
