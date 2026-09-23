- **Two `catch` blocks in one scope may not name the same variable, and a multi-step attack file is
  where that bites.** A hostile case is several numbered steps in one program, so a second
  `catch (RecursionError $error)` under a first `catch (LogicError $error)` is
  `E0406: $error is already declared`, and the case then reports as "never ran -- it does not
  compile", which reads exactly like work nobody has written yet. Give each step's catch its own
  name — `$stopped`, `$refused` — because a `.nvs` file's top-level statements share one scope.
  [until: reviewed 2026-09-20]
