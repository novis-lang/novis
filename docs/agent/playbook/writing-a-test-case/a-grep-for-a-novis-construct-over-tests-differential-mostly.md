- **A grep for a Novis construct over `tests/differential/` mostly matches PHP, not Novis.** Every
  case there carries a `--FILE--` Novis half and an `--ORACLE--` PHP half in one file, so a sweep
  measuring how much of the corpus a language change would break counts every `__construct` and
  free `function` in the oracle as a Novis site and can overstate the blast radius by an order of
  magnitude. Scope the measurement to `tests/conformance/` — which has no oracle section
  ([the bullet above](../../playbook.md)) — or split each differential file at `--ORACLE--` before
  counting. [until: gone crates/nvs-test/src/case.rs:ORACLE]
