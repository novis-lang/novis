- **The coverage gate matches a member's *fully qualified* call spelling, so a case written through
  `use Core\Test;` covers nothing it calls.** `crates/nvs-stdlib/tests/conformance_coverage.rs`
  greps `--FILE--` sections for `Class::member(` with the whole class name, so `Test::assertTrue(`
  after a `use` answers for nothing and `cargo test` fails with "N registered `Core` member(s) are
  never used by a conformance case". Write each new member once in the `Core\…` spelling somewhere
  in the case. [until: reviewed 2026-09-06]
