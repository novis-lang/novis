- **A checkout on a non-system Windows drive grants `Authenticated Users` modify, so this
  repository's own `nvs.toml` fails `rule:config/ownership-is-the-trust-boundary`.** Such a drive's
  root carries that ACE by default and everything under it inherits it, so a run reading the tree
  through `Files::trust` stops with `E0607`. The fix is on the machine and needs the user's say-so —
  `icacls <path> /inheritance:d` then `icacls <path> /remove:g "<the account>"` for `nvs.toml` and
  the checkout root, account names being localized; a scratch tree under `%TEMP%` passes as it is.
  [until: reviewed 2026-09-06]
