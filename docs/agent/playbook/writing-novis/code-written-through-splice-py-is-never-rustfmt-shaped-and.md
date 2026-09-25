- **Code written through `nv splice` is never rustfmt-shaped, and `nv verify` runs `fmt` after the
  build.** A slice that compiles and tests green still costs two full gate runs, because the
  formatting failure arrives only after build and tests have already run. Run `cargo fmt --all`
  before `nv verify`; and a `match` arm whose body is only `if cond { … }` is `collapsible_match`
  under `-D warnings`, so write it as a match guard. [until: gone tools/nv/cmd/verify.ts:fmt]
