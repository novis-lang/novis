- **A `@symbol` target names the member alone, so `@Shared::sweep` and `@SafepointView::request`
  answer *no definition or mention* — which reads like the symbol has been deleted.** `peek.py`
  matches a Rust definition by its own name, and a method's name does not carry its type; the
  qualified spelling is the one a `Core` member and a `.nvst` case take. Ask for `@sweep`, or
  `--locate sweep request`, and read the `impl` the hit lands in. [until: gone tools/peek.py:@symbol]
