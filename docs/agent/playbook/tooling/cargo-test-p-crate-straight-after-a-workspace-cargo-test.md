- **`cargo test -p <crate>` straight after a workspace `cargo test` recompiles the crate, and the
  workspace run recompiles it back.** A package selected alone unifies its dependencies' features
  differently from the whole workspace, so its `-p` artifact is a second one that every source edit
  stales, which is why `Goal.crate_tests` in `tools/loop.py` runs a crate's test executables off one
  workspace build. When you scope by hand, scope `nv verify -p` and every `cargo` call the same way
  for the whole session, or budget the rebuild at each switch. [until: reviewed 2026-09-06]
