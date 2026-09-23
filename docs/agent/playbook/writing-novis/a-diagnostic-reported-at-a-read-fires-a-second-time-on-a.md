- **A diagnostic reported at a read fires a second time on a declaration another diagnostic already
  refused, and the cases it turns red look unrelated.** `E0792` — a class constant whose value folds
  to nothing — is reported at the read on purpose, and then reported over the top of `E0246` (`const
  LIMIT = 9;` has no type, so it folds to nothing *because* it was refused) and `E0727` (`const
  secret bytes BLOB = "b";` cannot fold because there is no `bytes` literal). Before reporting on a
  use of a declaration, ask which declared types can never have reached that point cleanly and
  return early for each with the other diagnostic named. [until: reviewed 2026-09-06]
