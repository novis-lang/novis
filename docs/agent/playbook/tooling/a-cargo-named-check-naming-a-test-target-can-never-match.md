- **A `cargo-named` check naming a test *target* can never match anything.**
  `tests = ["spec_registry_coverage"]` read "did not run" while all ten tests in that file passed:
  `crate_tests` runs each test executable directly, so the `Running tests/<target>.rs` line cargo
  would have printed is nowhere in the output the check greps. When the name is a *file* under
  `tests/`, re-point it at the `fn` names inside it, in both toml copies.
  [until: gone tools/loop.py:def crate_tests]
