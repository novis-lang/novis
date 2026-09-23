- **A hostile case's per-step comment has two lines, and the "this is caught" note does not fit
  beside the step.** `python tools/dossier.py --comments` holds a numbered step's comment to 2 lines
  and the top comment to 4, while an attack that names its trick *and* says its error is caught
  needs three, so every step of a fresh case missed the bound on the first write. Put "every error
  is caught here, so every step runs" once in the top comment, and leave each step one sentence
  saying what it tries. [until: reviewed 2026-09-22]
