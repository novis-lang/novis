A shape key reuses the options bag's two diagnostics, widened from "option" to "shape key": an argument
that is not a written anonymous object, and a key the member does not declare. No new diagnostic code is
introduced.

A **missing required key** needs no code of its own, because an incomplete literal genuinely *is* a value of
the wrong type and the ordinary argument-type mismatch already reports it, naming the parameter. The
asymmetry is not an accident: an *extra* key is the one that needed a code, precisely because a shape type's
width subtyping (`rule:types/shape-type`) would otherwise have accepted it.

This matters more than it looks. Both type diagnostic bands are full, so a new type diagnostic would have
forced a third band open as a side effect of adding a parameter kind — the wrong reason to take that
decision.
