- **Renaming a test to match the live goal's check name can turn a *carried floor* check red, and the
  report then reads as unwritten work.** The floor is the previous goal's list verbatim and may never
  be edited to make something pass, so where two checks claim one test the live goal's check is the
  half that moves. Grep all of `docs/agent/loop-goal.toml` for the name the tree already has before
  renaming anything: a hit under `stage = "1 floor"` means rename the check, in both toml copies.
  [until: reviewed 2026-09-16]
