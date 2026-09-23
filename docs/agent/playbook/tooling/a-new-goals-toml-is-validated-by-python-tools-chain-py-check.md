- **A new goal's TOML is validated by `python tools/chain.py --check` and by nothing else, so run it
  before you believe the file.** Two mistakes look identical to a reading eye: a `[[check]]` `kind`
  the driver does not know (the set is `cargo-named`, `command`, `contains`, `exact`, `min-bytes`,
  `nvs-suite`, `ordered` — there is no `nvst`), and a `kind = "command"` naming a tool a *later*
  goal builds. `--check` catches the first and cannot catch the second, so prefer a `cargo-named`
  check against a test that exists today over a `command` against a tool that does not.
  [until: reviewed 2026-09-06]
