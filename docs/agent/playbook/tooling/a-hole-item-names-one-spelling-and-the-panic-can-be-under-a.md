- **A hole item names one spelling, and the panic can be under a different one.** An item saying
  `$a?->b = v` panics can be long closed while `$a?->b++` still does, because `check_write_target`
  was reached from the assignment arms and not from `PreIncDec`/`PostIncDec`; likewise a catch-all
  naming one route (`mixed`) is reached by every receiver `class_qname_of` cannot resolve. Run one
  scratch file per spelling and per receiver family against `target/debug/nvs.exe` before editing
  the site the item names; `git log -S` on the code it quotes says whether its half already landed.
  [until: reviewed 2026-09-06]
