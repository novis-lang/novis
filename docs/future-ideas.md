# this file as a temporary decision and future plans note
Do not read it, do not use it in your daily workflow. Its a reminder note for humans only.


## nvs doc — an API documentation generator
Novis has no answer to phpDocumentor/Doctum: nothing renders API docs from a user's own program.
The compiler already holds every signature and doc comment (Part B of docs/novis.md is generated
from the same registry), so an `nvs doc` is cheap the day someone wants it. Decision 2026-09-01:
recorded here only — no milestone owns it, and it gets an ADR when it is scheduled.
docs/adr/tooling-parity.md carries the tool-by-tool context.


## nvs fix — the reopen trigger for Novis-to-Novis rewrites
ADR 0039 § 9 declined an `nvs fix` batch-rewrite verb ("no user has asked for one yet"), and
`nvs convert` only covers PHP to Novis. Decision 2026-09-01: the door stays closed before 1.0, but
the first breaking change to the language surface after 1.0 must ship with an automated migration
rewrite — reopening ADR 0039's declination is part of that change's cost, so it is written down
here where a human planning that change will look.