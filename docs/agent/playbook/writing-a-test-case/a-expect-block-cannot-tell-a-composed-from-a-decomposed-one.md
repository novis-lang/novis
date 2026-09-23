- **A `--EXPECT--` block cannot tell a composed `é` from a decomposed one, and the failure prints as
  two identical-looking blocks.** A subject built with `\u{301}` fails against an expectation typed
  as the composed character, and *expected* and *actual* render alike; only `od -c` shows it. Assert
  a decomposed cluster against a source-escaped literal (`… == "e\u{301}fac" ? "1" : "0"`), and keep
  the eyeball line on a subject with one spelling. [until: reviewed 2026-09-06]
