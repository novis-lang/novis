- **An attack whose `nvs.toml` writes no `process.exec` line really starts every program it tries.**
  A CLI run with the key unwritten may run any program, so `Core\Process::run('icacls', ...)` changed
  the checkout's ACLs and PowerShell hung the Windows proofs job. An attack that expects a denial
  writes `[capabilities.process]` `exec = []`, as `tests/hostile/tools/install/windows/nvs.toml` does.
