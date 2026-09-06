# Handoff

## State

**ADR 0042 is wired at every producer of a unit, measured, and now within its start budget.**
`cache::unit_for` (`crates/nvs-cli/src/cache.rs:1193`) is the whole decision and all four call
sites make it — `run_run`, `runner::compile`, and `script::Compiler::compile` for a `spawn script`
isolate and `serve`'s per-core compiler. `tools/bench.py --warm-start` reports 3.7 ms of Novis work
against the goal's 6 ms check and 8.4 ms total against m6's 10 ms; every timed rep is a § 3 warm
hit, and that path costs 0.5 ms end to end.

**What was over budget was the trust check, not the cache.** ADR 0103 § 6's Windows half asked
`GetEffectiveRightsFromAclW` once per principal per path — five principals, the path and its
parent, 0.8 ms a call — so `Cache::new` paid 8 ms before it could look at an artifact.
`nvs_config::trust::effective_rights` (`crates/nvs-config/src/trust.rs:436`) now walks the DACL
itself and answers the same question: deny entries ahead of grants, inherited entries as ordinary
ones, `INHERIT_ONLY` skipped, generic bits mapped. Every `nvs` boot on Windows is 8 ms a path
cheaper. Nothing is blocked.

## Next group

**What the wiring is not yet asserted to do.** One file set — `crates/nvs-cli/src/runner.rs`,
`crates/nvs-cli/src/script.rs`, `crates/nvs-cli/src/cache.rs`.

- [ ] **One program, two subcommands, one artifact**: a test that a `nvs run` and a `nvs test` of
      the same program address the same key, which is the claim `runner::compile`'s comment makes
      (`crates/nvs-cli/src/runner.rs:559`). Both lower through `ENTRY_SCRIPT_LABEL` over the same
      file list, so the assertion is `cache::program_digest` equality plus a `Provenance::Loaded`
      out of the runner's compile over a cache the run populated.
- [ ] **A `spawn script` isolate warms across processes**: two `Compiler`s over one path sharing a
      cache directory, the second one's resolve answering out of it rather than compiling
      (`crates/nvs-cli/src/script.rs:243`). The playbook's scheduler-and-reactor bullet is what a
      fixture that actually spawns needs; a bare `Compiler::new` pair does not.
- [ ] **Narrow or drop `#![allow(dead_code)]`** now that every site is wired
      (`crates/nvs-cli/src/cache.rs:175`). It was the scaffolding for a module with no caller and
      it now hides a real one; whatever it still covers is either wired or wants deleting.

## Backlog

- `cache::from_config` runs twice per `nvs run` — once at `crates/nvs-cli/src/main.rs:1178` and
  once inside `script::Compiler::new` — building two `Cache`s and checking one directory twice.
  Cheap now that the check is microseconds, but it is still two answers to one question.
- No Windows case covers `trust::exposure`'s advisory half; `crates/nvs-config/tests/trust.rs` now
  has the `icacls` helper that would make one three lines.
- ADR 0042 § 2's *Known gaps* still names `aarch64`, where `mprotect` does not imply the
  instruction-cache maintenance a loaded payload needs — `crates/nvs-cli/src/cache.rs`'s module doc
  owns it.
