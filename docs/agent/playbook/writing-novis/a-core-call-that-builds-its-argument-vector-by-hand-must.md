- **A `Core` call that builds its argument vector by hand must call `account_for_arg` itself, and
  only a valgrind run will say it did not.** `nvs_ir::lower::lower_route_link` bypasses
  `lower_call_args` and lowered `$params` without staging it on `owned_temporaries`, so every
  `Core\Router::url("…", ["id" => 7])` leaked one array header per call while compiling, running,
  and passing every test. Wherever lowering hand-rolls what a shared helper does, the accounting is
  the half that gets dropped: run `MSYS_NO_PATHCONV=1 wsl.exe -- bash tools/leak-check.sh <fixture>`
  from the Bash tool, and pin it as `a_resolved_route_link_releases_its_params_array` does — require
  a `Release` of the argument's own `ValueId`, never a count of releases.
  [until: gone crates/nvs-ir/src/lower/tests.rs:a_resolved_route_link_releases_its_params_array]
