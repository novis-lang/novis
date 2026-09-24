- **`nv verify`'s `nvs-fmt` step rewrites a `.nvs` proof in place, and inside a property hook's
  `set (T $v) { … }` body it de-indents every statement by four.** What it hands back disagrees with
  the hook examples in `docs/reference/lang/50-classes.md`, and the wrap commits the formatter's
  version, so the indentation that lands is not the one you wrote. Write the hook body the way it
  reads best, run `bun nv verify` before the wrap so the rewrite happens there, and keep
  what `nvs-fmt` wrote — re-indenting it by hand only dirties the next session's tree.
  [until: reviewed 2026-09-19]
