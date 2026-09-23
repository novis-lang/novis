- **A `Core` helper that returns `Err` must not release its transferred argument; the double release
  aborts the process.** `spawn script`'s `args:` is `ArgOwnership::Transferred` and the value is
  still on the temporaries stack when `emit_fallible` builds the fault edge, so the landing block
  already releases it on every `Err` — an added `release()` is `thread caused non-unwinding panic.
  aborting.` inside `nvs_value_release`. `nvs_ir::lower::TemporaryKind`'s doc says it: `Transferred`
  is released on the error edge only, and that edge is the caller's. [until: reviewed 2026-09-06]
