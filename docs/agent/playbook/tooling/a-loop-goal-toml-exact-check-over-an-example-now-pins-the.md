- **A `loop-goal.toml` `exact` check over an example now pins the *line numbers* of that example's
  own producers, so a comment added above one turns the floor red.** A record's envelope carries
  `source` since goal `record-origin`'s stage 2 (`rule:errors/a-record-names-where-it-was-produced`), and
  `examples/logging.nvs`'s two `Core\Log::write` lines are in the check's `want` verbatim. Edit such
  an example only below its last producer, or run `target/debug/nvs.exe run <example>` afterwards and
  move the `want` with it — the check reports stdout line for line and says nothing about why a
  number moved. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
