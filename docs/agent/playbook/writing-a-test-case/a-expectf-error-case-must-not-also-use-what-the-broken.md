- **A `--EXPECTF-ERROR--` case must not also *use* what the broken declaration would have
  provided.** Diagnostics are ordered by phase, not by file, so an `E0303` from the entry point's
  reference prints before the resolution error the case exists to pin, and the block no longer
  matches at its first line. A compile-error case's entry file should do the least that reaches the
  diagnostic — often a bare `require` — and `%A` covers the span between two diagnostics, notes
  included. [until: reviewed 2026-09-06]
