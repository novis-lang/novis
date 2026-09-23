- **`nvs_runtime::affordable` is not the last check, and the gap is one header wide.** It accepts
  any size up to `isize::MAX` and knows nothing of the container header the allocation prepends, so
  a `Core\Str` producer handed exactly `isize::MAX` cleared it and panicked in `str_layout`'s two
  `expect`s — a FATAL out of the fallible `NvsStr::try_build`; `try_str_layout` closes it for every
  `built_fallibly` caller. The shape to recognise is a member whose size check and allocation are
  two different expressions (`padding_run` bounds the run while `built_fallibly` allocates the run
  plus the subject), and the boundary count is the largest the first accepts, not a round number.
  [until: reviewed 2026-09-06]
