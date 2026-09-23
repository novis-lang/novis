- **Code written through `splice.py` is never rustfmt-shaped, and `verify.py` runs `fmt` after the
  build.** A slice that compiles and tests green still costs two full gate runs, because the
  formatting failure arrives only after build and tests have already run. Run `cargo fmt --all`
  before `verify.py`; and a `match` arm whose body is only `if cond { … }` is `collapsible_match`
  under `-D warnings`, so write it as a match guard. [until: reviewed 2026-09-06]
