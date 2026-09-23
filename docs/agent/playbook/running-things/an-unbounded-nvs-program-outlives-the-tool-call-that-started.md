- **An unbounded `.nvs` program outlives the tool call that started it and keeps
  `target/debug/nvs.exe` open.** The next `cargo build` then fails with `error: failed to remove
  file ... Zugriff verweigert (os error 5)`, which reads as a permissions problem while the process
  is still growing — one left running here reached 6 GB. Start anything that may not terminate
  under `timeout 60`, and check `Get-Process nvs` before believing a build error about that file.
  [until: reviewed 2026-09-19]
