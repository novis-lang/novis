- **A `.nvst` helper cannot take a `Core` enum parameter, and the diagnostic names the same type
  twice.** `function m(string $s, Core\Charset $c)` called with `Core\Charset::Ascii` is `E0401`
  reading *expected `Core\Charset`, found `Core\Charset`*: a source annotation does not unify with
  the registry's `CoreTy::Enum`, and `mixed` is refused at the `Core` call inside. Write the enum at
  each call site, one inline `try`/`catch` per row. [until: reviewed 2026-09-06]
