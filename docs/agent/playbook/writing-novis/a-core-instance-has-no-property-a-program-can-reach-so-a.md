- **A `Core` instance has no property a program can reach, so a rule's `$x->thing` example is a
  surface the registry cannot express.** `CoreClass` has `methods`, `instance`, `slots` and
  `constants` and no field roster, a fact stated only in `CoreTy::Instance`'s doc and in comments on
  other classes. Check that doc before believing a spec'd property spelling; the fix is a reader
  with parentheses, folded back into the rule and the spec line in the same commit.
  [until: reviewed 2026-09-06]
