- **A `Core\X::member` spelling does not survive `spec_registry_coverage.rs`'s `cells()`, and the
  failure is silent.** `cells()` reads a `\` as a cell escape and re-emits it twice —
  `Core\Str::trim` comes back as `Core\\Str::trim` — so a member regex over its output matches
  nothing. Parse `docs/spec/02-php-migration.md` with `tools/check-migration.py`'s own row regex,
  transcribed into the Rust walk.
  [until: gone crates/nvs-stdlib/tests/spec_registry_coverage.rs:fn cells]
