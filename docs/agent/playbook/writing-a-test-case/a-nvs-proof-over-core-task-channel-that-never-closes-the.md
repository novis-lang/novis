- **A `.nvs` proof over `Core\Task\Channel` that never closes the channel hangs, and the hang is only
  reported when the timeout runs out.** A reader's `foreach` waits for as long as the channel is open
  and empty, so "nobody ever closes it" is a deadlock the runtime does not break, and an attack
  written around it spends its whole `timeout-ms` before it is named as a failure. Close on every
  path out of the producing task, and pin the deadlock's *absence* — a close that arrives while the
  reader is already waiting — rather than its presence. [until: reviewed 2026-09-21]
