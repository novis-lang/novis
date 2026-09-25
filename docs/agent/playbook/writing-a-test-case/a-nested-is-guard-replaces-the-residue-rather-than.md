- **A nested `is` guard *replaces* the residue rather than intersecting with it.** There is
  no intersection type, so inside `if ($v is Labelled) { if ($v is Counted) { … } }`
  the subject is a `Counted` and nothing else, and `$v->label()` there is `E0405: `Counted` has no
  method named `label``, pointing at the inner interface for a member the outer guard proved. Read
  what the outer guard bought into a local before writing the second test, as
  `tests/conformance/lang/an-is-guard-narrows-to-an-interface.nvst` does.
  [until: gone tests/conformance/lang/an-is-guard-narrows-to-an-interface.nvst:Counted]
