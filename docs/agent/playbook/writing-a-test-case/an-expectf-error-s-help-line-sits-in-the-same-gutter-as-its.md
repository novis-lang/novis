- **An `--EXPECTF-ERROR--`'s `= help:` line sits in the same gutter as its `-->` line, so its
  indentation widens with the line number too.** A block pinning the same help sentence at
  `case.nvs:5` and again at `case.nvs:15` needs two spaces on the first and three on the second, and
  the diff reads as two identical lines refused because the `%A` above absorbed the caret excerpt and
  left the leading space to the literal. Copy each `= help:`/`= note:` line out of the runner's actual
  half per error block rather than writing one and repeating it. [until: reviewed 2026-09-06]
